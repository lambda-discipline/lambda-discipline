//! Hindley-Milner type inference (algorithm J with level-based
//! generalization, as in OCaml) over [`hir`].
//!
//! * Top-level definitions are grouped into strongly connected components,
//!   so mutually recursive definitions are inferred together and every
//!   group is generalized before its users are checked.
//! * An annotated definition (`f : a -> a = ...`) is checked against its
//!   annotation; its type variables are rigid, so the body must really be
//!   that polymorphic. Other code sees the annotation, which also breaks
//!   dependency cycles.
//! * `let` is recursive and generalized (let-polymorphism).
//! * Type variables written in lambda annotations are scoped over the whole
//!   definition, like Haskell's `ScopedTypeVariables`.
//! * The unit's `type` declarations are added to the type environment
//!   first, so annotations and other declarations can use them in any order.
//!   A constructor's type is `forall params. fields -> Type<params>`.
//! * Pattern variables are monomorphic, like lambda parameters, and every
//!   `match` must cover all values of its scrutinee's type
//!   (see [`super::exhaustive`]).
//! * Type classes (`Eq`, `Ord`, `Show`) are sets of classes on type
//!   variables. Binding a variable to a type requires the type to belong to
//!   the variable's classes, which reduces to requirements on the type's
//!   arguments or fails for functions and actions. Generalization keeps the
//!   classes of quantified variables; classes on a variable that does not
//!   appear in the type are dropped, since the builtins work structurally
//!   and the choice of type cannot change what they do.

use std::collections::HashMap;
use std::rc::Rc;

use crate::builtins::Builtin;
use crate::diagnostics::Diagnostic;
use crate::hir::{
    self, Body, ExprKind, GlobalDef, GlobalId, LocalId, PatternKind, TyConId, TypeDef, Unit,
};
use crate::source::{SourceMap, Span};
use crate::syntax::ast::{Constraint, TypeExpr, TypeExprKind, TypeScheme};

use super::env::{ConInfo, TypeEnv, TypeInfo};
use super::exhaustive;
use super::ty::{Class, Classes, Scheme, TyCon, TyVar, Type, display_together};

/// The result of checking one unit.
#[derive(Debug, Default)]
pub struct Typed {
    /// Schemes of the unit's definitions, indexed like `Unit::defs`.
    pub def_schemes: Vec<Scheme>,
    /// Types of the unit's top-level expressions, in order.
    pub entry_schemes: Vec<Scheme>,
    /// The type environment with the unit's declared types added.
    pub types: TypeEnv,
    pub diagnostics: Vec<Diagnostic>,
}

/// What the top-level expressions of a unit may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopLevel {
    /// A program: every top-level expression is an `IO` action. Running the
    /// program performs them in source order.
    Actions,
    /// REPL input: any expression. Actions are performed and other values
    /// are shown.
    Any,
}

/// Type-checks a unit. `prior` holds the schemes of globals from earlier
/// units, indexed by [`GlobalId`]; the unit's own ids start at `prior.len()`.
/// `types` holds the types known before the unit.
pub fn check_unit(unit: &Unit, prior: &[Scheme], types: &TypeEnv, top_level: TopLevel) -> Typed {
    let mut checker = Checker::new(prior, types.clone());
    let first = prior.len();
    checker.declare_types(&unit.types);

    // Annotated definitions are known up front.
    let mut annotations: HashMap<GlobalId, Option<Rigid>> = HashMap::new();
    for def in &unit.defs {
        if let Some(ann) = &def.ann {
            let rigid = checker.annotation_scheme(ann);
            if let Some(rigid) = &rigid {
                checker
                    .globals
                    .insert(def.id, GlobalType::Known(rigid.scheme.clone()));
            }
            annotations.insert(def.id, rigid);
        }
    }

    for group in dependency_groups(unit, first) {
        checker.check_group(
            &group.iter().map(|&i| &unit.defs[i]).collect::<Vec<_>>(),
            &annotations,
        );
    }

    let def_schemes = unit
        .defs
        .iter()
        .map(|def| match checker.globals.get(&def.id) {
            Some(GlobalType::Known(scheme)) => scheme.clone(),
            _ => Scheme::poly(1, Type::Gen(0)),
        })
        .collect();

    let entry_schemes = unit
        .entries
        .iter()
        .map(|body| checker.check_entry(body, top_level))
        .collect();

    Typed {
        def_schemes,
        entry_schemes,
        types: checker.env,
        diagnostics: checker.diagnostics,
    }
}

/// Infers the type of a single expression body (the REPL's `:type`).
pub fn check_expr(body: &Body, prior: &[Scheme], types: &TypeEnv) -> (Scheme, Vec<Diagnostic>) {
    let mut checker = Checker::new(prior, types.clone());
    let scheme = checker.check_entry(body, TopLevel::Any);
    (scheme, checker.diagnostics)
}

/// The scheme of a builtin, parsed from its signature once per thread.
/// Panics if the signature is malformed, which the builtin tests rule out.
pub fn builtin_scheme(builtin: Builtin) -> Scheme {
    thread_local! {
        static CACHE: std::cell::RefCell<HashMap<Builtin, Scheme>> = std::cell::RefCell::new(HashMap::new());
    }
    CACHE.with(|cache| {
        cache
            .borrow_mut()
            .entry(builtin)
            .or_insert_with(|| parse_builtin_scheme(builtin))
            .clone()
    })
}

fn parse_builtin_scheme(builtin: Builtin) -> Scheme {
    let mut sources = SourceMap::new();
    let file = sources.add_virtual("<builtin>", builtin.signature());
    let ann = crate::syntax::parse_scheme(file, builtin.signature())
        .unwrap_or_else(|_| panic!("malformed signature for `{builtin}`"));
    let mut checker = Checker::new(&[], TypeEnv::new());
    checker
        .annotation_scheme(&ann)
        .unwrap_or_else(|| panic!("ill-formed signature for `{builtin}`"))
        .scheme
}

/// Orders definitions into strongly connected components (Tarjan), each
/// group after every group it depends on. Edges into annotated definitions
/// are ignored because their types are already known.
fn dependency_groups(unit: &Unit, first: usize) -> Vec<Vec<usize>> {
    // The resolver numbers a unit's definitions consecutively from `first`.
    debug_assert!(
        unit.defs
            .iter()
            .enumerate()
            .all(|(i, d)| d.id.index() == first + i)
    );
    let edges: Vec<Vec<usize>> = unit
        .defs
        .iter()
        .map(|def| {
            let mut out = Vec::new();
            def.body.expr.for_each_global(&mut |id| {
                let target = id.index().checked_sub(first);
                if let Some(target) =
                    target.filter(|&t| unit.defs.get(t).is_some_and(|d| d.ann.is_none()))
                {
                    out.push(target);
                }
            });
            out
        })
        .collect();

    struct Tarjan<'a> {
        edges: &'a [Vec<usize>],
        index: Vec<Option<usize>>,
        low: Vec<usize>,
        on_stack: Vec<bool>,
        stack: Vec<usize>,
        next: usize,
        groups: Vec<Vec<usize>>,
    }

    impl Tarjan<'_> {
        fn visit(&mut self, v: usize) {
            self.index[v] = Some(self.next);
            self.low[v] = self.next;
            self.next += 1;
            self.stack.push(v);
            self.on_stack[v] = true;
            for &w in &self.edges[v] {
                match self.index[w] {
                    None => {
                        self.visit(w);
                        self.low[v] = self.low[v].min(self.low[w]);
                    }
                    Some(index) if self.on_stack[w] => self.low[v] = self.low[v].min(index),
                    Some(_) => {}
                }
            }
            if Some(self.low[v]) == self.index[v] {
                let mut group = Vec::new();
                while let Some(w) = self.stack.pop() {
                    self.on_stack[w] = false;
                    group.push(w);
                    if w == v {
                        break;
                    }
                }
                group.sort_unstable();
                self.groups.push(group);
            }
        }
    }

    let n = unit.defs.len();
    let mut tarjan = Tarjan {
        edges: &edges,
        index: vec![None; n],
        low: vec![0; n],
        on_stack: vec![false; n],
        stack: Vec::new(),
        next: 0,
        groups: Vec::new(),
    };
    for v in 0..n {
        if tarjan.index[v].is_none() {
            tarjan.visit(v);
        }
    }
    tarjan.groups
}

/// An annotation turned into a scheme, plus its rigid instantiation.
#[derive(Debug, Clone)]
struct Rigid {
    scheme: Scheme,
    /// The annotation with its variables as [`Type::Rigid`].
    ty: Type,
    names: Vec<Rc<str>>,
    /// The classes the context gives each of `names`.
    classes: Vec<Classes>,
}

#[derive(Debug, Clone)]
enum GlobalType {
    /// Final type (annotated, or an already generalized group).
    Known(Scheme),
    /// Monomorphic placeholder while its group is being inferred.
    Pending(Type),
}

#[derive(Debug, Clone)]
enum VarState {
    Unbound { level: u32, classes: Classes },
    Link(Type),
}

/// Per-body state: local types and annotation type variables in scope.
#[derive(Default)]
struct BodyCtx {
    locals: HashMap<LocalId, Scheme>,
    type_vars: HashMap<Rc<str>, Type>,
    /// Only the first error of a body is reported, to avoid cascades.
    failed: bool,
}

struct Checker<'a> {
    prior: &'a [Scheme],
    env: TypeEnv,
    globals: HashMap<GlobalId, GlobalType>,
    vars: Vec<VarState>,
    level: u32,
    rigid_levels: HashMap<Rc<str>, u32>,
    /// The classes each rigid variable's annotation declares.
    rigid_classes: HashMap<Rc<str>, Classes>,
    /// Where the classes of a variable were first required (the use of
    /// `eq` that made a variable need `Eq`), for error messages.
    class_origins: HashMap<TyVar, Span>,
    diagnostics: Vec<Diagnostic>,
}

struct Mismatch {
    expected: Type,
    found: Type,
    reason: MismatchReason,
}

enum MismatchReason {
    Different,
    /// A variable occurs in the type it would be bound to. `expected` holds
    /// the variable and `found` that type, whichever side of the
    /// unification the variable was on.
    Infinite,
    Escape(Rc<str>),
    /// `found` (part of `expected`) does not belong to the class, which was
    /// required at the span, if known.
    NoInstance(Class, Option<Span>),
}

impl<'a> Checker<'a> {
    fn new(prior: &'a [Scheme], env: TypeEnv) -> Self {
        Self {
            prior,
            env,
            globals: HashMap::new(),
            vars: Vec::new(),
            level: 0,
            rigid_levels: HashMap::new(),
            rigid_classes: HashMap::new(),
            class_origins: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    // ---- variables, levels and unification -------------------------------

    fn fresh(&mut self) -> Type {
        self.fresh_in(Classes::NONE)
    }

    /// A fresh variable that must belong to `classes`.
    fn fresh_in(&mut self, classes: Classes) -> Type {
        let var = TyVar(self.vars.len() as u32);
        self.vars.push(VarState::Unbound {
            level: self.level,
            classes,
        });
        Type::Var(var)
    }

    fn enter(&mut self) {
        self.level += 1;
    }

    fn exit(&mut self) {
        self.level -= 1;
    }

    /// Follows links at the top of a type (with path compression).
    fn shallow(&mut self, ty: &Type) -> Type {
        match ty {
            Type::Var(v) => match self.vars[v.0 as usize].clone() {
                VarState::Link(target) => {
                    let resolved = self.shallow(&target);
                    self.vars[v.0 as usize] = VarState::Link(resolved.clone());
                    resolved
                }
                VarState::Unbound { .. } => ty.clone(),
            },
            _ => ty.clone(),
        }
    }

    /// Fully substitutes solved variables.
    fn zonk(&mut self, ty: &Type) -> Type {
        match self.shallow(ty) {
            Type::Con(name, args) => Type::Con(name, args.iter().map(|a| self.zonk(a)).collect()),
            Type::Fun(from, to) => Type::fun(self.zonk(&from), self.zonk(&to)),
            other => other,
        }
    }

    fn unify(&mut self, expected: &Type, found: &Type) -> Result<(), Mismatch> {
        let a = self.shallow(expected);
        let b = self.shallow(found);
        match (&a, &b) {
            (Type::Var(x), Type::Var(y)) if x == y => Ok(()),
            (Type::Var(v), other) | (other, Type::Var(v)) => self.bind(*v, other, &a, &b),
            (Type::Rigid(x), Type::Rigid(y)) if x == y => Ok(()),
            (Type::Con(n1, args1), Type::Con(n2, args2))
                if n1 == n2 && args1.len() == args2.len() =>
            {
                for (x, y) in args1.iter().zip(args2) {
                    self.unify(x, y).map_err(|m| self.widen(m, &a, &b))?;
                }
                Ok(())
            }
            (Type::Fun(p1, r1), Type::Fun(p2, r2)) => {
                self.unify(p1, p2).map_err(|m| self.widen(m, &a, &b))?;
                self.unify(r1, r2).map_err(|m| self.widen(m, &a, &b))
            }
            _ => Err(Mismatch {
                expected: a,
                found: b,
                reason: MismatchReason::Different,
            }),
        }
    }

    /// Reports a mismatch in terms of the outer types being unified, which
    /// reads better than the innermost conflicting pieces.
    fn widen(&self, mismatch: Mismatch, expected: &Type, found: &Type) -> Mismatch {
        match mismatch.reason {
            MismatchReason::Different => Mismatch {
                expected: expected.clone(),
                found: found.clone(),
                reason: MismatchReason::Different,
            },
            _ => mismatch,
        }
    }

    fn bind(
        &mut self,
        var: TyVar,
        ty: &Type,
        expected: &Type,
        found: &Type,
    ) -> Result<(), Mismatch> {
        let VarState::Unbound { level, classes } = self.vars[var.0 as usize] else {
            unreachable!("bind is only called on unbound variables");
        };
        let escapes = match ty {
            Type::Rigid(name) => self
                .rigid_levels
                .get(name)
                .is_some_and(|&rigid| level < rigid),
            _ => false,
        };
        if let (true, Type::Rigid(name)) = (escapes, ty) {
            return Err(Mismatch {
                expected: expected.clone(),
                found: found.clone(),
                reason: MismatchReason::Escape(name.clone()),
            });
        }
        if self.occurs_and_adjust(var, level, ty) {
            return Err(Mismatch {
                expected: Type::Var(var),
                found: ty.clone(),
                reason: MismatchReason::Infinite,
            });
        }
        self.vars[var.0 as usize] = VarState::Link(ty.clone());
        let origin = self.class_origins.get(&var).copied();
        self.require(classes, ty, origin)
            .map_err(|(class, culprit)| Mismatch {
                expected: ty.clone(),
                found: culprit,
                reason: MismatchReason::NoInstance(class, origin),
            })
    }

    /// Requires `ty` to belong to `classes`. On failure, returns a class and
    /// the part of `ty` that does not belong to it.
    /// The error names the most specific class missing (`Ord` rather than
    /// `Eq`). `origin` is where the classes were required.
    fn require(
        &mut self,
        classes: Classes,
        ty: &Type,
        origin: Option<Span>,
    ) -> Result<(), (Class, Type)> {
        let Some(first) = classes.shown().next() else {
            return Ok(());
        };
        match self.shallow(ty) {
            Type::Var(v) => {
                if let VarState::Unbound {
                    level,
                    classes: own,
                } = self.vars[v.0 as usize]
                {
                    self.vars[v.0 as usize] = VarState::Unbound {
                        level,
                        classes: own.union(classes),
                    };
                    if let Some(origin) = origin {
                        self.class_origins.entry(v).or_insert(origin);
                    }
                }
                Ok(())
            }
            Type::Rigid(name) => {
                let declared = self.rigid_classes.get(&name).copied().unwrap_or_default();
                match classes.minus(declared).shown().next() {
                    Some(class) => Err((class, Type::Rigid(name))),
                    None => Ok(()),
                }
            }
            Type::Con(con, args) => {
                // `Ord` implies `Eq`, and a type's `Ord` instance needs the
                // same parameters as its `Eq` instance.
                for class in classes.shown() {
                    let Some(params) = self.env.instance(con.id, class).map(<[usize]>::to_vec)
                    else {
                        return Err((class, Type::Con(con, args)));
                    };
                    for index in params {
                        self.require(Classes::of(class), &args[index], origin)?;
                    }
                }
                Ok(())
            }
            fun @ Type::Fun(..) => Err((first, fun)),
            Type::Gen(_) => Ok(()),
        }
    }

    /// Occurs check; also lowers the levels of variables in `ty` to `level`
    /// so they are not generalized too early.
    fn occurs_and_adjust(&mut self, var: TyVar, level: u32, ty: &Type) -> bool {
        match self.shallow(ty) {
            Type::Var(v) if v == var => true,
            Type::Var(v) => {
                if let VarState::Unbound {
                    level: other,
                    classes,
                } = self.vars[v.0 as usize]
                {
                    if other > level {
                        self.vars[v.0 as usize] = VarState::Unbound { level, classes };
                    }
                }
                false
            }
            Type::Con(_, args) => args.iter().any(|a| self.occurs_and_adjust(var, level, a)),
            Type::Fun(from, to) => {
                self.occurs_and_adjust(var, level, &from) || self.occurs_and_adjust(var, level, &to)
            }
            Type::Gen(_) | Type::Rigid(_) => false,
        }
    }

    /// Quantifies over variables created at a deeper level than the current
    /// one, keeping their classes.
    fn generalize(&mut self, ty: &Type) -> Scheme {
        let ty = self.zonk(ty);
        let mut mapping: HashMap<TyVar, u32> = HashMap::new();
        let ty = self.generalize_in(&ty, &mut mapping, &HashMap::new());
        let mut bounds = vec![Classes::NONE; mapping.len()];
        for (var, index) in mapping {
            if let VarState::Unbound { classes, .. } = self.vars[var.0 as usize] {
                bounds[index as usize] = classes;
            }
        }
        Scheme {
            arity: bounds.len() as u32,
            bounds,
            ty,
        }
    }

    fn generalize_in(
        &self,
        ty: &Type,
        vars: &mut HashMap<TyVar, u32>,
        rigids: &HashMap<Rc<str>, u32>,
    ) -> Type {
        match ty {
            Type::Var(v) => match self.vars[v.0 as usize] {
                VarState::Unbound { level, .. } if level > self.level => {
                    let next = (vars.len() + rigids.len()) as u32;
                    Type::Gen(*vars.entry(*v).or_insert(next))
                }
                _ => ty.clone(),
            },
            Type::Rigid(name) => rigids
                .get(name)
                .map_or_else(|| ty.clone(), |&g| Type::Gen(g)),
            Type::Con(name, args) => Type::Con(
                name.clone(),
                args.iter()
                    .map(|a| self.generalize_in(a, vars, rigids))
                    .collect(),
            ),
            Type::Fun(from, to) => Type::fun(
                self.generalize_in(from, vars, rigids),
                self.generalize_in(to, vars, rigids),
            ),
            Type::Gen(_) => ty.clone(),
        }
    }

    fn instantiate(&mut self, scheme: &Scheme) -> Type {
        let fresh: Vec<Type> = (0..scheme.arity)
            .map(|index| self.fresh_in(scheme.bound(index)))
            .collect();
        substitute_gens(&scheme.ty, &fresh)
    }

    /// Instantiates the scheme of a name used at `span`, which is where its
    /// variables' classes are required.
    fn instantiate_at(&mut self, scheme: &Scheme, span: Span) -> Type {
        let fresh: Vec<Type> = (0..scheme.arity)
            .map(|index| {
                let classes = scheme.bound(index);
                let var = self.fresh_in(classes);
                if let (false, Type::Var(v)) = (classes.is_empty(), &var) {
                    self.class_origins.insert(*v, span);
                }
                var
            })
            .collect();
        substitute_gens(&scheme.ty, &fresh)
    }

    // ---- annotations --------------------------------------------------------

    /// Converts an annotation whose free variables are all quantified.
    /// Reports ill-formed types and returns `None` for them.
    fn annotation_scheme(&mut self, ann: &TypeScheme) -> Option<Rigid> {
        let mut names: Vec<Rc<str>> = Vec::new();
        let mut ok = true;
        let ty = self.convert(
            &ann.ty,
            &mut |name, _| {
                let index = names.iter().position(|n| n == name).unwrap_or_else(|| {
                    names.push(name.clone());
                    names.len() - 1
                });
                Some(Type::Rigid(names[index].clone()))
            },
            &mut ok,
        )?;
        if !ok {
            return None;
        }
        let classes = self.context(&ann.context, &names, |_| false)?;
        let rigids: HashMap<Rc<str>, u32> = names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i as u32))
            .collect();
        let scheme = Scheme {
            arity: names.len() as u32,
            bounds: classes.clone(),
            ty: self.generalize_in(&ty, &mut HashMap::new(), &rigids),
        };
        Some(Rigid {
            scheme,
            ty,
            names,
            classes,
        })
    }

    /// Converts a context to the classes of each of `names`, the variables
    /// the annotation quantifies. `outer` says whether a name belongs to an
    /// enclosing annotation instead. Reports errors and returns `None`.
    fn context(
        &mut self,
        context: &[Constraint],
        names: &[Rc<str>],
        outer: impl Fn(&str) -> bool,
    ) -> Option<Vec<Classes>> {
        let mut classes = vec![Classes::NONE; names.len()];
        let mut ok = true;
        for constraint in context {
            let Some(class) = Class::lookup(&constraint.class.name) else {
                ok = false;
                self.diagnostics.push(
                    Diagnostic::error(format!("unknown class `{}`", constraint.class.name))
                        .at(constraint.class.span)
                        .with_label("not a known class")
                        .with_note("the classes are `Eq`, `Ord` and `Show`"),
                );
                continue;
            };
            let TypeExprKind::Var(var) = &constraint.arg.kind else {
                ok = false;
                self.diagnostics.push(
                    Diagnostic::error("a constraint applies to a type variable")
                        .at(constraint.arg.span)
                        .with_label("not a type variable")
                        .with_help(format!(
                            "constrain the variables inside it instead, e.g. `{class}<a>`"
                        )),
                );
                continue;
            };
            match names.iter().position(|name| *name == var.name) {
                Some(index) => classes[index] = classes[index].with(class),
                None if outer(&var.name) => {
                    ok = false;
                    self.diagnostics.push(
                        Diagnostic::error(format!(
                            "`{}` belongs to an enclosing annotation",
                            var.name
                        ))
                        .at(constraint.span)
                        .with_label("cannot be constrained here")
                        .with_help(format!(
                            "add `{class}<{}>` to the annotation that introduces `{}`",
                            var.name, var.name
                        )),
                    );
                }
                None => {
                    ok = false;
                    self.diagnostics.push(
                        Diagnostic::error(format!(
                            "the type variable `{}` is constrained but not used",
                            var.name
                        ))
                        .at(constraint.span)
                        .with_label("does not appear in the type")
                        .with_note(
                            "a constraint must apply to a variable of the type it qualifies",
                        ),
                    );
                }
            }
        }
        ok.then_some(classes)
    }

    /// Converts a type expression; `var` maps a type variable name to a type.
    fn convert(
        &mut self,
        ann: &TypeExpr,
        var: &mut dyn FnMut(&Rc<str>, &mut Self) -> Option<Type>,
        ok: &mut bool,
    ) -> Option<Type> {
        Some(match &ann.kind {
            TypeExprKind::Unit => Type::unit(),
            TypeExprKind::Var(name) => var(&name.name, self)?,
            TypeExprKind::Fun(from, to) => {
                let from = self.convert(from, var, ok);
                let to = self.convert(to, var, ok);
                Type::fun(from?, to?)
            }
            TypeExprKind::Con { name, args } => {
                let converted: Vec<Option<Type>> =
                    args.iter().map(|a| self.convert(a, var, ok)).collect();
                let known = self
                    .env
                    .lookup_type(&name.name)
                    .map(|info| (info.con.clone(), info.arity()));
                match known {
                    None => {
                        *ok = false;
                        let mut diagnostic =
                            Diagnostic::error(format!("unknown type `{}`", name.name))
                                .at(name.span)
                                .with_label("not a known type");
                        if let Some(hint) = type_hint(&name.name) {
                            diagnostic = diagnostic.with_help(hint);
                        }
                        self.diagnostics.push(diagnostic);
                        return None;
                    }
                    Some((_, arity)) if arity != args.len() => {
                        *ok = false;
                        self.diagnostics.push(
                            Diagnostic::error(format!(
                                "`{}` takes {} type argument{}, but {} {} given",
                                name.name,
                                arity,
                                if arity == 1 { "" } else { "s" },
                                args.len(),
                                if args.len() == 1 { "was" } else { "were" },
                            ))
                            .at(ann.span)
                            .with_label("wrong number of type arguments")
                            .with_help(match arity {
                                0 => format!("write `{}`", name.name),
                                1 => format!("write `{}<a>`", name.name),
                                _ => format!(
                                    "write `{}<{}>`",
                                    name.name,
                                    (0..arity)
                                        .map(|i| ((b'a' + i as u8) as char).to_string())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
                            }),
                        );
                        return None;
                    }
                    Some((con, _)) => Type::Con(con, converted.into_iter().collect::<Option<_>>()?),
                }
            }
        })
    }

    /// Converts an annotation inside a body: variables already in scope are
    /// reused, new ones become flexible variables scoped to the body.
    fn body_annotation(&mut self, ann: &TypeExpr, ctx: &mut BodyCtx) -> Option<Type> {
        let mut ok = true;
        let ty = self.convert(
            ann,
            &mut |name, this| {
                Some(
                    ctx.type_vars
                        .entry(name.clone())
                        .or_insert_with(|| this.fresh())
                        .clone(),
                )
            },
            &mut ok,
        )?;
        ok.then_some(ty)
    }

    // ---- type declarations ------------------------------------------------------

    /// Adds the unit's types to the environment, then their constructors,
    /// whose field types may refer to any of the types.
    fn declare_types(&mut self, types: &[TypeDef]) {
        for def in types {
            self.env.add_type(TypeInfo {
                con: TyCon {
                    id: def.id,
                    name: def.name.name.clone(),
                },
                params: def.params.iter().map(|p| p.name.clone()).collect(),
                constructors: def.constructors.iter().map(|c| c.id).collect(),
                span: Some(def.name.span),
                // Filled in by `derive_instances`.
                instances: Default::default(),
            });
        }
        for def in types {
            self.declare_constructors(def);
        }
        self.derive_instances(types);
    }

    /// Works out which classes each declared type belongs to: a type
    /// belongs to a class when all its fields do, which for a field of a
    /// parameter's type means the parameter must too. Types may be
    /// recursive, so this is a greatest fixed point: every type starts in
    /// every class with no requirements, and classes are removed and
    /// requirements added until nothing changes.
    fn derive_instances(&mut self, types: &[TypeDef]) {
        for def in types {
            self.env.set_instances(
                def.id,
                [Some(Vec::new()), Some(Vec::new()), Some(Vec::new())],
            );
        }
        loop {
            let mut changed = false;
            for def in types {
                let info = self.env.ty(def.id).clone();
                let mut instances = info.instances.clone();
                for class in Class::ALL {
                    let Some(current) = &instances[class.index()] else {
                        continue;
                    };
                    let mut needed = vec![false; info.arity()];
                    for &param in current {
                        needed[param] = true;
                    }
                    let mut ok = true;
                    for &con in &info.constructors {
                        let mut ty = &self.env.constructor(con).scheme.ty;
                        while let Type::Fun(field, rest) = ty {
                            ok &= self.field_requires(class, field, &mut needed);
                            ty = rest;
                        }
                    }
                    let derived = ok.then(|| (0..needed.len()).filter(|&i| needed[i]).collect());
                    if derived != instances[class.index()] {
                        instances[class.index()] = derived;
                        changed = true;
                    }
                }
                self.env.set_instances(def.id, instances);
            }
            if !changed {
                break;
            }
        }
    }

    /// Whether a field of type `ty` can belong to `class`, marking the
    /// parameters (quantified variables below `needed.len()`) it needs.
    fn field_requires(&self, class: Class, ty: &Type, needed: &mut [bool]) -> bool {
        match ty {
            Type::Gen(index) => {
                // Higher indices stand in for fields that failed to convert.
                if let Some(slot) = needed.get_mut(*index as usize) {
                    *slot = true;
                }
                true
            }
            Type::Con(con, args) => match self.env.instance(con.id, class) {
                None => false,
                Some(params) => params
                    .iter()
                    .all(|&param| self.field_requires(class, &args[param], needed)),
            },
            Type::Fun(..) => false,
            Type::Var(_) | Type::Rigid(_) => true,
        }
    }

    fn declare_constructors(&mut self, def: &TypeDef) {
        let params: Vec<Rc<str>> = def.params.iter().map(|p| p.name.clone()).collect();
        for (i, param) in def.params.iter().enumerate() {
            if let Some(first) = def.params[..i].iter().find(|p| p.name == param.name) {
                self.diagnostics.push(
                    Diagnostic::error(format!(
                        "the type parameter `{}` is declared more than once",
                        param.name
                    ))
                    .at(param.span)
                    .with_label("declared again here")
                    .with_secondary(first.span, "first declared here"),
                );
            }
        }
        let result = Type::Con(
            self.env.ty(def.id).con.clone(),
            (0..params.len() as u32).map(Type::Gen).collect(),
        );
        for ctor in &def.constructors {
            // A field that fails to convert becomes a fresh quantified
            // variable, so uses of the constructor do not add more errors.
            let mut extra = 0;
            let mut fields = Vec::new();
            for field in &ctor.fields {
                let mut ok = true;
                let converted = self.convert(
                    field,
                    &mut |name, this| match params.iter().position(|p| p == name) {
                        Some(index) => Some(Type::Gen(index as u32)),
                        None => {
                            this.diagnostics.push(
                                Diagnostic::error(format!(
                                    "the type variable `{name}` is not a parameter of `{}`",
                                    def.name.name
                                ))
                                .at(field.span)
                                .with_label("not declared")
                                .with_help(format!(
                                    "declare it after the type's name: `type {}<{}>`",
                                    def.name.name,
                                    params
                                        .iter()
                                        .map(|p| p.as_ref())
                                        .chain([name.as_ref()])
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                )),
                            );
                            None
                        }
                    },
                    &mut ok,
                );
                fields.push(converted.filter(|_| ok).unwrap_or_else(|| {
                    extra += 1;
                    Type::Gen((params.len() + extra - 1) as u32)
                }));
            }
            self.env.add_constructor(ConInfo {
                id: ctor.id,
                name: ctor.name.name.clone(),
                ty: def.id,
                scheme: Scheme::poly(
                    (params.len() + extra) as u32,
                    Type::curried(fields, result.clone()),
                ),
                arity: ctor.fields.len(),
                span: Some(ctor.name.span),
            });
        }
    }

    // ---- definitions ----------------------------------------------------------

    fn check_group(
        &mut self,
        group: &[&GlobalDef],
        annotations: &HashMap<GlobalId, Option<Rigid>>,
    ) {
        self.enter();
        for def in group {
            if def.ann.is_none() {
                let ty = self.fresh();
                self.globals.insert(def.id, GlobalType::Pending(ty));
            }
        }
        for def in group {
            let mut ctx = BodyCtx::default();
            let expected = match annotations.get(&def.id) {
                Some(Some(rigid)) => {
                    for (name, &classes) in rigid.names.iter().zip(&rigid.classes) {
                        self.rigid_levels.insert(name.clone(), self.level);
                        self.rigid_classes.insert(name.clone(), classes);
                        ctx.type_vars
                            .insert(name.clone(), Type::Rigid(name.clone()));
                    }
                    rigid.ty.clone()
                }
                // An ill-formed annotation was already reported.
                Some(None) => {
                    ctx.failed = true;
                    self.fresh()
                }
                None => match &self.globals[&def.id] {
                    GlobalType::Pending(ty) => ty.clone(),
                    GlobalType::Known(_) => unreachable!("unannotated definitions start pending"),
                },
            };
            let found = self.infer(&def.body.expr, &mut ctx);
            if let Err(mismatch) = self.unify(&expected, &found) {
                let annotated = def.ann.is_some();
                self.report(&mut ctx, mismatch, def.body.expr.span, |expected, found| {
                    if annotated {
                        format!("the annotation says `{expected}`, but the definition has type `{found}`")
                    } else {
                        format!("expected `{expected}`, found `{found}`")
                    }
                });
            }
        }
        self.exit();
        for def in group {
            if let Some(GlobalType::Pending(ty)) = self.globals.get(&def.id).cloned() {
                let scheme = self.generalize(&ty);
                self.globals.insert(def.id, GlobalType::Known(scheme));
            }
        }
    }

    fn check_entry(&mut self, body: &Body, top_level: TopLevel) -> Scheme {
        self.enter();
        let mut ctx = BodyCtx::default();
        let ty = self.infer(&body.expr, &mut ctx);
        if top_level == TopLevel::Actions {
            let action = Type::io(self.fresh());
            if self.unify(&action, &ty).is_err() {
                let found = self.zonk(&ty);
                self.error(
                    &mut ctx,
                    Diagnostic::error("a top-level expression must be an `IO` action")
                        .at(body.expr.span)
                        .with_label(format!("this has type `{found}`"))
                        .with_note("a program runs its top-level actions in source order; any other value would be discarded")
                        .with_help("print it with `(println ...)`, or name it with a definition"),
                );
            }
        }
        self.exit();
        self.generalize(&ty)
    }

    // ---- expressions ----------------------------------------------------------

    fn infer(&mut self, expr: &hir::Expr, ctx: &mut BodyCtx) -> Type {
        match &expr.kind {
            ExprKind::Number(_) => Type::number(),
            ExprKind::String(_) => Type::string(),
            ExprKind::Unit => Type::unit(),
            ExprKind::Error => self.fresh(),
            ExprKind::Local(id) => match ctx.locals.get(id).cloned() {
                Some(scheme) => self.instantiate_at(&scheme, expr.span),
                None => self.fresh(),
            },
            ExprKind::Global(id) => match self.global_type(*id) {
                GlobalType::Known(scheme) => self.instantiate_at(&scheme, expr.span),
                GlobalType::Pending(ty) => ty,
            },
            ExprKind::Builtin(builtin) => {
                if builtin.is_variadic() {
                    self.error(
                        ctx,
                        Diagnostic::error(format!("`{builtin}` must be called directly"))
                            .at(expr.span)
                            .with_label("used as a value here")
                            .with_note(format!("`{builtin}` takes any number of arguments, so it has no single function type"))
                            .with_help(format!("wrap it in a lambda, e.g. `\\x. ({builtin} x)`")),
                    );
                    return self.fresh();
                }
                let scheme = builtin_scheme(*builtin);
                self.instantiate_at(&scheme, expr.span)
            }
            ExprKind::Lambda { param, ann, body } => {
                let param_ty = match ann {
                    Some(ann) => self.body_annotation(ann, ctx).unwrap_or_else(|| {
                        ctx.failed = true;
                        self.fresh()
                    }),
                    None => self.fresh(),
                };
                ctx.locals.insert(*param, Scheme::mono(param_ty.clone()));
                let body_ty = self.infer(body, ctx);
                Type::fun(param_ty, body_ty)
            }
            ExprKind::Con(con) => {
                let scheme = self.env.constructor(*con).scheme.clone();
                self.instantiate(&scheme)
            }
            ExprKind::Match { scrutinee, arms } => self.infer_match(scrutinee, arms, ctx),
            ExprKind::App { func, args } => self.infer_app(func, args, ctx),
            ExprKind::List(items) => {
                let element = self.fresh();
                for item in items {
                    let found = self.infer(item, ctx);
                    if let Err(mismatch) = self.unify(&element, &found) {
                        self.report(ctx, mismatch, item.span, |expected, found| {
                            format!("list elements must all have the same type: expected `{expected}`, found `{found}`")
                        });
                    }
                }
                Type::list(element)
            }
            ExprKind::Let {
                local,
                ann,
                value,
                body,
            } => {
                self.enter();
                let expected = match ann {
                    Some(ann) => self.let_annotation(ann, ctx),
                    None => None,
                };
                let (scheme, mono, new_names) = match &expected {
                    Some((scheme, rigid_ty, new_names)) => {
                        (scheme.clone(), rigid_ty.clone(), new_names.clone())
                    }
                    None => {
                        let ty = self.fresh();
                        (Scheme::mono(ty.clone()), ty, Vec::new())
                    }
                };
                ctx.locals.insert(*local, scheme);
                // The annotation's own variables are in scope inside the value.
                for name in &new_names {
                    ctx.type_vars
                        .insert(name.clone(), Type::Rigid(name.clone()));
                }
                let found = self.infer(value, ctx);
                for name in &new_names {
                    ctx.type_vars.remove(name);
                }
                if let Err(mismatch) = self.unify(&mono, &found) {
                    self.report(ctx, mismatch, value.span, |expected, found| {
                        format!("expected `{expected}`, found `{found}`")
                    });
                }
                self.exit();
                if expected.is_none() {
                    let scheme = self.generalize(&mono);
                    ctx.locals.insert(*local, scheme);
                }
                self.infer(body, ctx)
            }
        }
    }

    /// A `let` annotation quantifies its own new type variables.
    fn let_annotation(
        &mut self,
        ann: &TypeScheme,
        ctx: &mut BodyCtx,
    ) -> Option<(Scheme, Type, Vec<Rc<str>>)> {
        let mut ok = true;
        let mut new_names: Vec<Rc<str>> = Vec::new();
        let level = self.level;
        let ty = self.convert(
            &ann.ty,
            &mut |name, _| {
                Some(ctx.type_vars.get(name).cloned().unwrap_or_else(|| {
                    if !new_names.contains(name) {
                        new_names.push(name.clone());
                    }
                    Type::Rigid(name.clone())
                }))
            },
            &mut ok,
        );
        let classes = match ty {
            Some(_) if ok => self.context(&ann.context, &new_names, |name| {
                ctx.type_vars.contains_key(name)
            }),
            _ => None,
        };
        let (Some(ty), Some(classes)) = (ty, classes) else {
            ctx.failed = true;
            return None;
        };
        for (name, &classes) in new_names.iter().zip(&classes) {
            self.rigid_levels.insert(name.clone(), level);
            self.rigid_classes.insert(name.clone(), classes);
        }
        let rigids: HashMap<Rc<str>, u32> = new_names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i as u32))
            .collect();
        let zonked = self.zonk(&ty);
        let scheme = Scheme {
            arity: new_names.len() as u32,
            bounds: classes,
            ty: self.generalize_in(&zonked, &mut HashMap::new(), &rigids),
        };
        Some((scheme, ty, new_names))
    }

    fn global_type(&mut self, id: GlobalId) -> GlobalType {
        if let Some(scheme) = self.prior.get(id.index()) {
            return GlobalType::Known(scheme.clone());
        }
        match self.globals.get(&id) {
            Some(known) => known.clone(),
            // A definition whose own checking failed earlier.
            None => GlobalType::Pending(self.fresh()),
        }
    }

    fn infer_app(&mut self, func: &hir::Expr, args: &[hir::Expr], ctx: &mut BodyCtx) -> Type {
        if matches!(&func.kind, ExprKind::Builtin(builtin) if builtin.is_variadic()) {
            for arg in args {
                let ty = self.infer(arg, ctx);
                let origin = Some(func.span);
                if let Err((class, culprit)) = self.require(Classes::of(Class::Show), &ty, origin) {
                    let mismatch = Mismatch {
                        expected: ty,
                        found: culprit,
                        reason: MismatchReason::NoInstance(class, origin),
                    };
                    self.report(ctx, mismatch, arg.span, |_, _| String::new());
                }
            }
            return Type::io(Type::unit());
        }
        let mut current = self.infer(func, ctx);
        for (index, arg) in args.iter().enumerate() {
            let (param, result) = match self.shallow(&current) {
                Type::Fun(param, result) => (*param, *result),
                Type::Var(_) => {
                    let param = self.fresh();
                    let result = self.fresh();
                    // Fails only if the variable must belong to a class.
                    if let Err(mismatch) =
                        self.unify(&current, &Type::fun(param.clone(), result.clone()))
                    {
                        self.report(ctx, mismatch, func.span, |_, _| String::new());
                        for rest in &args[index..] {
                            self.infer(rest, ctx);
                        }
                        return self.fresh();
                    }
                    (param, result)
                }
                other => {
                    let shown = self.zonk(&other);
                    let diagnostic = if index == 0 {
                        Diagnostic::error(format!("`{shown}` is not a function"))
                            .at(func.span)
                            .with_label(format!("this has type `{shown}`"))
                    } else {
                        Diagnostic::error(format!(
                            "too many arguments: this function takes {index}, but {} were given",
                            args.len()
                        ))
                        .at(arg.span)
                        .with_label("unexpected argument")
                        .with_secondary(
                            func.span,
                            format!(
                                "returns `{shown}` after {index} argument{}",
                                if index == 1 { "" } else { "s" }
                            ),
                        )
                    };
                    self.error(ctx, diagnostic);
                    for rest in &args[index..] {
                        self.infer(rest, ctx);
                    }
                    return self.fresh();
                }
            };
            let found = self.infer(arg, ctx);
            if let Err(mismatch) = self.unify(&param, &found) {
                let callee = describe_callee(func);
                let position = index + 1;
                self.report(ctx, mismatch, arg.span, |expected, found| {
                    format!("expected `{expected}`, found `{found}`")
                });
                if let Some(last) = self
                    .diagnostics
                    .last_mut()
                    .filter(|last| last.span == Some(arg.span))
                {
                    last.notes
                        .push(format!("in argument {position} of {callee}"));
                }
            }
            current = result;
        }
        current
    }

    // ---- match ----------------------------------------------------------------

    fn infer_match(&mut self, scrutinee: &hir::Expr, arms: &[hir::Arm], ctx: &mut BodyCtx) -> Type {
        let matched = self.infer(scrutinee, ctx);
        let result = self.fresh();
        for arm in arms {
            self.check_pattern(&arm.pattern, &matched, ctx);
            let found = self.infer(&arm.body, ctx);
            if let Err(mismatch) = self.unify(&result, &found) {
                self.report(ctx, mismatch, arm.body.span, |expected, found| {
                    format!("the arms of a `match` must all have the same type: expected `{expected}`, found `{found}`")
                });
            }
        }
        // A pattern with a type error could make a complete match look
        // incomplete, so only well-typed matches are checked.
        if !ctx.failed {
            let patterns: Vec<&hir::Pattern> = arms.iter().map(|arm| &arm.pattern).collect();
            if let Some(missing) = exhaustive::missing(&self.env, &patterns) {
                let ty = self.zonk(&matched);
                self.error(
                    ctx,
                    Diagnostic::error(format!(
                        "this `match` does not cover every value of type `{ty}`"
                    ))
                    .at(scrutinee.span)
                    .with_label(format!("`{missing}` is not covered"))
                    .with_help(if missing == "_" {
                        "add an arm `| _ -> ...` to cover every other value".to_owned()
                    } else {
                        format!(
                            "add an arm `| {missing} -> ...`, or `| _ -> ...` to cover every other value"
                        )
                    }),
                );
            }
        }
        result
    }

    /// Checks that `pattern` can match values of type `expected` and gives
    /// its variables their types.
    fn check_pattern(&mut self, pattern: &hir::Pattern, expected: &Type, ctx: &mut BodyCtx) {
        let found = match &pattern.kind {
            PatternKind::Wild | PatternKind::Error => return,
            PatternKind::Bind(local) => {
                ctx.locals.insert(*local, Scheme::mono(expected.clone()));
                return;
            }
            PatternKind::Number(_) => Type::number(),
            PatternKind::String(_) => Type::string(),
            PatternKind::Unit => Type::unit(),
            PatternKind::Con { con, args } => {
                let info = self.env.constructor(*con).clone();
                let mut ty = self.instantiate(&info.scheme);
                let mut fields = Vec::with_capacity(info.arity);
                while let Type::Fun(param, result) = ty {
                    fields.push(*param);
                    ty = *result;
                }
                if args.len() != info.arity {
                    let plural = |n: usize| if n == 1 { "" } else { "s" };
                    self.error(
                        ctx,
                        Diagnostic::error(format!(
                            "the constructor `{}` has {} field{}, but this pattern has {}",
                            info.name,
                            info.arity,
                            plural(info.arity),
                            args.len()
                        ))
                        .at(pattern.span)
                        .with_label(format!(
                            "expected {} field pattern{}",
                            info.arity,
                            plural(info.arity)
                        ))
                        .with_help(format!(
                            "write `{}{}`, with `_` for each field you do not need",
                            info.name,
                            " _".repeat(info.arity)
                        )),
                    );
                }
                if let Err(mismatch) = self.unify(expected, &ty) {
                    self.report(ctx, mismatch, pattern.span, |expected, found| {
                        format!("this pattern has type `{found}`, but the value matched has type `{expected}`")
                    });
                }
                for (arg, field) in args.iter().zip(fields) {
                    self.check_pattern(arg, &field, ctx);
                }
                // Bind the variables of surplus patterns so the body can use them.
                for arg in args.iter().skip(info.arity) {
                    let unknown = self.fresh();
                    self.check_pattern(arg, &unknown, ctx);
                }
                return;
            }
        };
        if let Err(mismatch) = self.unify(expected, &found) {
            self.report(ctx, mismatch, pattern.span, |expected, found| {
                format!(
                    "this pattern has type `{found}`, but the value matched has type `{expected}`"
                )
            });
        }
    }

    // ---- errors -------------------------------------------------------------

    fn error(&mut self, ctx: &mut BodyCtx, diagnostic: Diagnostic) {
        if !ctx.failed {
            ctx.failed = true;
            self.diagnostics.push(diagnostic);
        }
    }

    fn report(
        &mut self,
        ctx: &mut BodyCtx,
        mismatch: Mismatch,
        span: Span,
        label: impl FnOnce(&str, &str) -> String,
    ) {
        let expected = self.zonk(&mismatch.expected);
        let found = self.zonk(&mismatch.found);
        let shown = display_together(&[&expected, &found]);
        let diagnostic = match mismatch.reason {
            MismatchReason::Different => Diagnostic::error("mismatched types")
                .at(span)
                .with_label(label(&shown[0], &shown[1])),
            MismatchReason::Infinite => Diagnostic::error("cannot construct an infinite type")
                .at(span)
                .with_label(format!("`{}` would have to contain itself: `{}`", shown[0], shown[1])),
            MismatchReason::Escape(name) => Diagnostic::error(format!(
                "type variable `{name}` would escape its scope"
            ))
            .at(span)
            .with_label(label(&shown[0], &shown[1]))
            .with_note("an annotated `let` is generic in its own type variables, so they cannot be tied to outer types"),
            MismatchReason::NoInstance(class, origin) => {
                no_instance(class, &found, &shown[0], &shown[1], span, origin)
            }
        };
        self.error(ctx, diagnostic);
    }
}

/// The error for a type that does not belong to `class`: `whole` is the
/// type that had to, and `part` (`culprit`) the piece of it that does not.
/// `span` is where the types met and `origin` where the class was required.
fn no_instance(
    class: Class,
    culprit: &Type,
    whole: &str,
    part: &str,
    span: Span,
    origin: Option<Span>,
) -> Diagnostic {
    let verb = class.verb();
    if let Type::Rigid(name) = culprit {
        // The use that needs the class is more useful than where the
        // annotation's variable met it, which may be the whole definition.
        return Diagnostic::error(format!("`{name}` cannot be {verb} here"))
            .at(origin.unwrap_or(span))
            .with_label(format!("this needs `{class}<{name}>`"))
            .with_note(format!(
                "`{name}` comes from an annotation, which does not say that it belongs to `{class}`"
            ))
            .with_help(format!(
                "add `{class}<{name}> =>` in front of the annotation that introduces `{name}`"
            ));
    }
    let label = if whole == part {
        format!("this has type `{whole}`")
    } else {
        format!("this has type `{whole}`, which contains `{part}`")
    };
    let (message, note) = match culprit {
        Type::Fun(..) => (format!("functions cannot be {verb}"), None),
        Type::Con(con, _) if con.id == TyConId::IO => {
            (format!("`IO` actions cannot be {verb}"), None)
        }
        _ => (
            format!("values of type `{part}` cannot be {verb}"),
            Some(format!(
                "a data type can be {verb} only if all its fields can; functions and `IO` actions cannot"
            )),
        ),
    };
    let mut diagnostic = Diagnostic::error(message)
        .at(span)
        .with_label(label)
        .with_note(note.unwrap_or_else(|| format!(
            "numbers, strings, `Unit`, lists and data types can be {verb}; functions and `IO` actions cannot"
        )));
    if let Some(origin) = origin.filter(|&origin| origin != span) {
        diagnostic = diagnostic.with_secondary(origin, format!("this needs `{class}<{whole}>`"));
    }
    diagnostic
}

fn substitute_gens(ty: &Type, with: &[Type]) -> Type {
    match ty {
        Type::Gen(i) => with[*i as usize].clone(),
        Type::Con(name, args) => Type::Con(
            name.clone(),
            args.iter().map(|a| substitute_gens(a, with)).collect(),
        ),
        Type::Fun(from, to) => Type::fun(substitute_gens(from, with), substitute_gens(to, with)),
        Type::Var(_) | Type::Rigid(_) => ty.clone(),
    }
}

fn describe_callee(func: &hir::Expr) -> String {
    match &func.kind {
        ExprKind::Builtin(builtin) => format!("`{builtin}`"),
        _ => "this call".to_owned(),
    }
}

fn type_hint(name: &str) -> Option<String> {
    match name {
        "Float" | "Int" | "Integer" | "Double" => Some("numbers have type `Number`".into()),
        "Any" => Some("there is no `Any` type; use a type variable such as `a`".into()),
        "Str" => Some("strings have type `String`".into()),
        "Boolean" => Some("booleans have type `Bool`".into()),
        "Eq" | "Ord" | "Show" => Some(format!(
            "`{name}` is a class, not a type: write `{name}<a> =>` in front of the type"
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_text;
    use crate::resolve::{EmptyScope, resolve};

    fn check(text: &str) -> (Unit, Typed) {
        let mut sources = SourceMap::new();
        let loaded = load_text(&mut sources, "<test>", text);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        let resolved = resolve(loaded.items, &EmptyScope);
        assert!(
            resolved.diagnostics.is_empty(),
            "{:?}",
            resolved.diagnostics
        );
        let typed = check_unit(&resolved.unit, &[], &TypeEnv::new(), TopLevel::Any);
        (resolved.unit, typed)
    }

    /// `name : type` for every definition.
    fn types_of(text: &str) -> Vec<String> {
        let (unit, typed) = check(text);
        assert!(typed.diagnostics.is_empty(), "{:#?}", typed.diagnostics);
        unit.defs
            .iter()
            .zip(&typed.def_schemes)
            .map(|(def, scheme)| format!("{} : {}", def.name, scheme))
            .collect()
    }

    fn type_of_entry(text: &str) -> String {
        let (_, typed) = check(text);
        assert!(typed.diagnostics.is_empty(), "{:#?}", typed.diagnostics);
        typed.entry_schemes.last().unwrap().to_string()
    }

    fn errors_of(text: &str) -> Vec<Diagnostic> {
        let (_, typed) = check(text);
        assert!(!typed.diagnostics.is_empty(), "expected a type error");
        typed.diagnostics
    }

    #[test]
    fn literals_and_lists() {
        assert_eq!(type_of_entry("1"), "Number");
        assert_eq!(type_of_entry("\"s\""), "String");
        assert_eq!(type_of_entry("()"), "Unit");
        assert_eq!(type_of_entry("[]"), "List<a>");
        assert_eq!(type_of_entry("[[1], []]"), "List<List<Number>>");
    }

    #[test]
    fn identity_and_composition_are_polymorphic() {
        assert_eq!(
            types_of(r"id = \x. x  compose = \f. \g. \x. (f (g x))"),
            ["id : a -> a", "compose : (a -> b) -> (c -> a) -> c -> b"]
        );
    }

    #[test]
    fn let_polymorphism() {
        assert_eq!(
            type_of_entry(r#"let id = \x. x in [(id 1), (length (id ["a"]))]"#),
            "List<Number>"
        );
    }

    #[test]
    fn lambda_bound_variables_are_monomorphic() {
        let errors = errors_of(r#"f = \id. [(id 1), (id "a")]"#);
        assert_eq!(errors[0].message, "mismatched types");
    }

    #[test]
    fn builtins_are_instantiated_freshly() {
        assert_eq!(type_of_entry("(map (add 1) [1, 2])"), "List<Number>");
        assert_eq!(type_of_entry(r#"(map show [True])"#), "List<String>");
        assert_eq!(type_of_entry("(foldr cons [] [1])"), "List<Number>");
    }

    #[test]
    fn partial_application() {
        assert_eq!(type_of_entry("(add 1)"), "Number -> Number");
        assert_eq!(type_of_entry("(if True)"), "a -> a -> a");
    }

    #[test]
    fn recursion_and_mutual_recursion() {
        assert_eq!(
            types_of(r"fact = \n. (if (eq n 0) 1 (mul n (fact (sub n 1))))"),
            ["fact : Number -> Number"]
        );
        assert_eq!(
            types_of(
                r"even = \n. (if (eq n 0) True (odd (sub n 1)))  odd = \n. (if (eq n 0) False (even (sub n 1)))"
            ),
            ["even : Number -> Bool", "odd : Number -> Bool"]
        );
    }

    #[test]
    fn groups_are_generalized_before_use() {
        assert_eq!(
            types_of(r#"use1 = (id 1)  use2 = (id "a")  id = \x. x"#),
            ["use1 : Number", "use2 : String", "id : a -> a"]
        );
    }

    #[test]
    fn annotations_are_checked_and_respected() {
        assert_eq!(
            types_of(r"f : Number -> Number = \x. x"),
            ["f : Number -> Number"]
        );
        assert_eq!(types_of(r"f : a -> a = \x. x"), ["f : a -> a"]);
        let errors = errors_of(r"f : a -> b = \x. x");
        assert_eq!(errors[0].message, "mismatched types");
        assert!(
            errors[0]
                .label
                .as_deref()
                .unwrap()
                .contains("the annotation says `a -> b`")
        );
    }

    #[test]
    fn annotations_enable_polymorphic_use_inside_their_group() {
        assert_eq!(
            types_of(r#"f : a -> a = \x. (const x (g 1))  g = \y. (f "s")  const = \a. \b. a"#)[0],
            "f : a -> a"
        );
    }

    #[test]
    fn lambda_annotations_share_scoped_type_variables() {
        assert_eq!(
            types_of(r"apply : (a -> b) -> a -> b = \f: (a -> b). \x: a. (f x)"),
            ["apply : (a -> b) -> a -> b"]
        );
        assert_eq!(
            types_of(r"pair = \x: a. \y: a. [x, y]"),
            ["pair : a -> a -> List<a>"]
        );
    }

    #[test]
    fn annotated_let() {
        assert_eq!(
            type_of_entry(r#"let id : a -> a = \x. x in [(show (id 1)), (id "s")]"#),
            "List<String>"
        );
    }

    #[test]
    fn rigid_variables_cannot_escape() {
        let errors = errors_of(r"f = \x. let g : a -> a = \y. x in g");
        assert_eq!(
            errors[0].message,
            "type variable `a` would escape its scope"
        );
    }

    #[test]
    fn mismatch_points_at_argument() {
        let errors = errors_of(r#"(add "hello" 5)"#);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "mismatched types");
        assert_eq!(
            errors[0].label.as_deref(),
            Some("expected `Number`, found `String`")
        );
        assert_eq!(errors[0].span.unwrap().range(), 5..12);
        assert_eq!(errors[0].notes, ["in argument 1 of `add`"]);
    }

    #[test]
    fn calling_a_non_function() {
        let errors = errors_of("(1 2)");
        assert_eq!(errors[0].message, "`Number` is not a function");
        let errors = errors_of("(add 1 2 3)");
        assert_eq!(
            errors[0].message,
            "too many arguments: this function takes 2, but 3 were given"
        );
    }

    #[test]
    fn occurs_check() {
        let errors = errors_of(r"f = \x. (x x)");
        assert_eq!(errors[0].message, "cannot construct an infinite type");
        assert_eq!(
            errors[0].label.as_deref(),
            Some("`a` would have to contain itself: `a -> b`")
        );
    }

    #[test]
    fn occurs_check_names_the_variable_first_when_it_is_found() {
        // The variable is on the found side here, unlike in `occurs_check`.
        let errors =
            errors_of(r"build = \n. \acc. (if (eq n 0) acc (build (sub n 1) (cons acc [])))");
        assert_eq!(errors[0].message, "cannot construct an infinite type");
        assert_eq!(
            errors[0].label.as_deref(),
            Some("`a` would have to contain itself: `List<a>`")
        );
    }

    #[test]
    fn heterogeneous_lists_are_rejected() {
        let errors = errors_of(r#"[1, "a"]"#);
        assert!(
            errors[0]
                .label
                .as_deref()
                .unwrap()
                .starts_with("list elements must all have the same type")
        );
    }

    #[test]
    fn unknown_types_and_wrong_arity() {
        let errors = errors_of(r"f : Float -> Float = \x. x");
        assert_eq!(errors[0].message, "unknown type `Float`");
        assert_eq!(
            errors[0].help.as_deref(),
            Some("numbers have type `Number`")
        );
        let errors = errors_of(r"f : List -> Number = length");
        assert_eq!(
            errors[0].message,
            "`List` takes 1 type argument, but 0 were given"
        );
    }

    #[test]
    fn variadic_builtins() {
        assert_eq!(type_of_entry(r#"(print 1 "a" [True])"#), "IO<Unit>");
        let errors = errors_of("(map print [1])");
        assert_eq!(errors[0].message, "`print` must be called directly");
        let errors = errors_of(r"(print 1 \x. x)");
        assert_eq!(errors[0].message, "functions cannot be shown");
    }

    #[test]
    fn contexts_are_inferred_and_generalized() {
        assert_eq!(
            types_of(r"member = \x. \xs. (foldr \y. \found. (or (eq x y) found) False xs)"),
            ["member : Eq<a> => a -> List<a> -> Bool"]
        );
        assert_eq!(
            types_of(r"f = \x. \y. (concat (show x) (show (sort y)))"),
            ["f : (Ord<b>, Show<a>, Show<b>) => a -> List<b> -> String"]
        );
        // `Ord` implies `Eq`.
        assert_eq!(
            types_of(r"f = \x. (and (eq x x) (lt x x))"),
            ["f : Ord<a> => a -> Bool"]
        );
        // No monomorphism restriction: a partial application generalizes.
        assert_eq!(
            types_of("isZero = (eq 0)\nsame = eq"),
            ["isZero : Number -> Bool", "same : Eq<a> => a -> a -> Bool"]
        );
        // A constraint on a variable that does not appear is dropped.
        assert_eq!(type_of_entry("(show [])"), "String");
        // Lambda-bound variables collect their constraints.
        assert_eq!(
            type_of_entry(r"\f. (eq (f 1) (f 2))"),
            "Eq<a> => (Number -> a) -> Bool"
        );
    }

    #[test]
    fn instances_reduce_to_their_arguments() {
        assert_eq!(type_of_entry("(eq [[1]] [])"), "Bool");
        assert_eq!(type_of_entry(r"\x. (show [x])"), "Show<a> => a -> String");
        assert_eq!(
            type_of_entry(&format!(r"{OPTION}\x. (lt (Some x) None)")),
            "Ord<a> => a -> Bool"
        );
        let errors = errors_of(r"(eq [\x. x] [])");
        assert_eq!(errors[0].message, "functions cannot be compared");
        assert_eq!(
            errors[0].label.as_deref(),
            Some("this has type `List<a -> a>`, which contains `a -> a`")
        );
        let errors = errors_of("(show (pure 1))");
        assert_eq!(errors[0].message, "`IO` actions cannot be shown");
        let errors = errors_of(r"(lt \x. x \x. x)");
        assert_eq!(errors[0].message, "functions cannot be ordered");
        // A constraint found later in the body still reports the use.
        let errors = errors_of(r"f = \x. (and (eq x x) (x 1))");
        assert_eq!(errors[0].message, "functions cannot be compared");
    }

    #[test]
    fn data_types_derive_instances_from_their_fields() {
        let ok = format!(
            "{OPTION}type Tree<a> = Leaf | Node Tree<a> a Tree<a>\n\
             type Phantom<a> = Phantom\n\
             type Even<a> = Zero | Even (Odd<a>)\n\
             type Odd<a> = Odd a (Even<a>)\n\
             a = \\x. (eq (Some x) None)\n\
             b = \\x. (sort [(Node Leaf x Leaf)])\n\
             tag : a -> Phantom<a> = \\x. Phantom\n\
             c = \\f. (eq (tag f) (tag f))\n\
             d = \\x. (show (Odd x Zero))"
        );
        let (_, typed) = check(&ok);
        let shown: Vec<String> = typed.def_schemes.iter().map(|s| s.to_string()).collect();
        assert_eq!(
            shown,
            [
                "Eq<a> => a -> Bool",
                "Ord<a> => a -> List<Tree<a>>",
                "a -> Phantom<a>",
                "a -> Bool",
                "Show<a> => a -> String"
            ],
            "{:#?}",
            typed.diagnostics
        );
        let errors = errors_of(
            "type Handler = Handler (Number -> Number)\n\
             type Wrap = Wrap Handler\n\
             x = (eq (Wrap (Handler neg)) (Wrap (Handler neg)))",
        );
        assert_eq!(
            errors[0].message,
            "values of type `Wrap` cannot be compared"
        );
        let errors = errors_of("type Job = Job (IO<Unit>)\nx = (show (Job (pure ())))");
        assert_eq!(errors[0].message, "values of type `Job` cannot be shown");
    }

    #[test]
    fn annotations_declare_contexts() {
        assert_eq!(
            types_of(
                r"member : Eq<a> => a -> List<a> -> Bool = \x. \xs. (foldr \y. \r. (or (eq x y) r) False xs)"
            ),
            ["member : Eq<a> => a -> List<a> -> Bool"]
        );
        // `Ord` in the annotation allows `eq` in the body.
        assert_eq!(
            types_of(r"f : Ord<a> => a -> Bool = \x. (eq x x)"),
            ["f : Ord<a> => a -> Bool"]
        );
        // A context may be wider than the body needs.
        assert_eq!(
            types_of(r"f : (Eq<a>, Show<b>) => a -> b -> a = \x. \y. x"),
            ["f : (Eq<a>, Show<b>) => a -> b -> a"]
        );
        // Polymorphic recursion at a type that needs the class.
        assert_eq!(
            types_of(
                r"f : Eq<a> => Number -> a -> Bool = \n. \x. (if (eq n 0) (eq x x) (f (sub n 1) [x]))"
            ),
            ["f : Eq<a> => Number -> a -> Bool"]
        );
        // Annotated `let`.
        assert_eq!(
            type_of_entry(r"let same : Eq<a> => a -> a -> Bool = eq in (same 1 2)"),
            "Bool"
        );
        let errors = errors_of(r"f : a -> Bool = \x. (lt x x)");
        assert_eq!(errors[0].message, "`a` cannot be ordered here");
        assert_eq!(errors[0].label.as_deref(), Some("this needs `Ord<a>`"));
        assert_eq!(
            errors[0].help.as_deref(),
            Some("add `Ord<a> =>` in front of the annotation that introduces `a`")
        );
        let errors = errors_of(r"f : Eq<a> => a -> String = \x. (show x)");
        assert_eq!(errors[0].message, "`a` cannot be shown here");
        // Callers must satisfy the annotation's context.
        let errors = errors_of("f : Eq<a> => a -> Bool = \\x. (eq x x)\ny = (f \\z. z)");
        assert_eq!(errors[0].message, "functions cannot be compared");
    }

    #[test]
    fn malformed_contexts_are_reported() {
        let errors = errors_of(r"f : Num<a> => a -> a = \x. x");
        assert_eq!(errors[0].message, "unknown class `Num`");
        let errors = errors_of(r"f : Eq<List<a>> => a -> a = \x. x");
        assert_eq!(errors[0].message, "a constraint applies to a type variable");
        let errors = errors_of(r"f : Eq<b> => a -> a = \x. x");
        assert_eq!(
            errors[0].message,
            "the type variable `b` is constrained but not used"
        );
        let errors = errors_of(r"f : a -> a = \x. let g : Eq<a> => a -> Bool = \y. (eq x y) in x");
        assert_eq!(errors[0].message, "`a` belongs to an enclosing annotation");
        let errors = errors_of(r"f : Eq -> Bool = \x. True");
        assert_eq!(
            errors[0].help.as_deref(),
            Some("`Eq` is a class, not a type: write `Eq<a> =>` in front of the type")
        );
    }

    #[test]
    fn actions_and_do_blocks() {
        assert_eq!(type_of_entry(r#"(readFile "f")"#), "IO<String>");
        assert_eq!(
            type_of_entry(
                r#"let s <- (readFile "f") in let _ <- (println s) in (pure (length (lines s)))"#
            ),
            "IO<Number>"
        );
        assert_eq!(
            types_of(r"twice = \action. (then action action)"),
            ["twice : IO<a> -> IO<a>"]
        );
        assert_eq!(
            type_of_entry("(sequence [(pure 1), (pure 2)])"),
            "IO<List<Number>>"
        );
        assert_eq!(
            type_of_entry(r#"let n: Number <- (pure 1) in (pure n)"#),
            "IO<Number>"
        );
        let errors = errors_of(r#"let x <- (add 1 2) in (println x)"#);
        assert_eq!(errors[0].message, "mismatched types");
        let errors = errors_of(r#"let x <- (pure 1) in x"#);
        assert_eq!(errors[0].message, "mismatched types");
    }

    /// Checks `text` as a program, whose top-level expressions are actions.
    fn program_errors(text: &str) -> Vec<Diagnostic> {
        let mut sources = SourceMap::new();
        let loaded = load_text(&mut sources, "<test>", text);
        let resolved = resolve(loaded.items, &EmptyScope);
        check_unit(&resolved.unit, &[], &TypeEnv::new(), TopLevel::Actions).diagnostics
    }

    #[test]
    fn program_top_level_expressions_must_be_actions() {
        assert!(
            program_errors(r#"x = 1  (println x)  (pure ())  (bind (pure 1) pure)"#).is_empty()
        );
        let errors = program_errors(r#"(add 1 2)  [(println "a")]"#);
        let messages: Vec<_> = errors.iter().map(|e| e.message.as_str()).collect();
        assert_eq!(
            messages,
            ["a top-level expression must be an `IO` action"; 2]
        );
        assert_eq!(
            errors[1].label.as_deref(),
            Some("this has type `List<IO<Unit>>`")
        );
        // An expression that is already wrong is reported once.
        assert_eq!(
            program_errors(r#"(add "a" 1)"#)[0].message,
            "mismatched types"
        );
        assert_eq!(program_errors(r#"(add "a" 1)"#).len(), 1);
    }

    #[test]
    fn one_error_per_definition() {
        let (_, typed) = check(r#"a = (add "x" "y")  b = (not 1)"#);
        assert_eq!(typed.diagnostics.len(), 2);
    }

    #[test]
    fn builtin_schemes_round_trip() {
        assert_eq!(
            builtin_scheme(Builtin::Map).to_string(),
            "(a -> b) -> List<a> -> List<b>"
        );
        assert_eq!(builtin_scheme(Builtin::Not).to_string(), "Bool -> Bool");
        for builtin in Builtin::ALL {
            builtin_scheme(*builtin);
        }
    }

    #[test]
    fn prior_units_are_used() {
        let mut sources = SourceMap::new();
        let loaded = load_text(&mut sources, "<test>", "(f 1)");
        struct Scope;
        impl crate::resolve::GlobalScope for Scope {
            fn lookup(&self, name: &str) -> Option<GlobalId> {
                (name == "f").then_some(GlobalId(0))
            }
            fn names(&self) -> Vec<Rc<str>> {
                vec![]
            }
            fn next_id(&self) -> u32 {
                1
            }
        }
        let resolved = resolve(loaded.items, &Scope);
        let prior = [Scheme::poly(
            1,
            Type::fun(Type::Gen(0), Type::list(Type::Gen(0))),
        )];
        let typed = check_unit(&resolved.unit, &prior, &TypeEnv::new(), TopLevel::Any);
        assert_eq!(typed.entry_schemes[0].to_string(), "List<Number>");
    }

    const OPTION: &str = "type Option<a> = None | Some a\n";

    #[test]
    fn constructors_are_curried_polymorphic_functions() {
        assert_eq!(
            types_of(&format!(
                "{OPTION}type Pair<a, b> = Pair a b\nsome = Some\nnone = None\npair = Pair\nboxed = (Pair 1)"
            )),
            [
                "some : a -> Option<a>",
                "none : Option<a>",
                "pair : a -> b -> Pair<a, b>",
                "boxed : a -> Pair<Number, a>",
            ]
        );
        assert_eq!(
            type_of_entry(&format!("{OPTION}(map Some [1])")),
            "List<Option<Number>>"
        );
        assert_eq!(type_of_entry("[True, False]"), "List<Bool>");
    }

    #[test]
    fn declared_types_can_be_recursive_and_used_in_any_order() {
        assert_eq!(
            types_of(
                "depth : Tree<a> -> Number = \\t. match t\n\
                 | Node l _ r -> (add 1 (max (depth l) (depth r)))\n\
                 | _ -> 0\n\
                 type Tree<a> = Leaf | Node Tree<a> a Tree<a> | Branches Forest<a>\n\
                 type Forest<a> = Forest List<Tree<a>>\n\
                 f = \\t. match t | Node _ x _ -> x | _ -> 0"
            ),
            ["depth : Tree<a> -> Number", "f : Tree<Number> -> Number"]
        );
    }

    #[test]
    fn match_infers_pattern_and_result_types() {
        assert_eq!(
            types_of(&format!(
                "{OPTION}withDefault = \\d. \\o. match o | Some x -> x | None -> d\n\
                 isZero = \\n. match n | 0 -> True | _ -> False"
            )),
            [
                "withDefault : a -> Option<a> -> a",
                "isZero : Number -> Bool"
            ]
        );
    }

    #[test]
    fn pattern_type_errors() {
        let errors = errors_of(&format!(
            "{OPTION}type Shape = Circle Number\nf = \\o. match o | Some x -> x | Circle r -> r"
        ));
        assert_eq!(errors[0].message, "mismatched types");
        assert_eq!(
            errors[0].label.as_deref(),
            Some("this pattern has type `Shape`, but the value matched has type `Option<a>`")
        );
        let errors = errors_of(&format!(
            "{OPTION}f = \\o. match o | Some x y -> x | None -> 0"
        ));
        assert_eq!(
            errors[0].message,
            "the constructor `Some` has 1 field, but this pattern has 2"
        );
        assert_eq!(
            errors[0].help.as_deref(),
            Some("write `Some _`, with `_` for each field you do not need")
        );
        let errors = errors_of("f = \\b. match b | True -> 1 | False -> \"no\"");
        assert_eq!(
            errors[0].label.as_deref(),
            Some(
                "the arms of a `match` must all have the same type: expected `Number`, found `String`"
            )
        );
    }

    #[test]
    fn non_exhaustive_matches_are_errors() {
        let errors = errors_of(&format!("{OPTION}f = \\o. match o | Some x -> x"));
        assert_eq!(
            errors[0].message,
            "this `match` does not cover every value of type `Option<a>`"
        );
        assert_eq!(errors[0].label.as_deref(), Some("`None` is not covered"));
        assert_eq!(
            errors[0].help.as_deref(),
            Some("add an arm `| None -> ...`, or `| _ -> ...` to cover every other value")
        );
    }

    #[test]
    fn type_declaration_errors() {
        let errors = errors_of("type Box = Box a");
        assert_eq!(
            errors[0].message,
            "the type variable `a` is not a parameter of `Box`"
        );
        assert_eq!(
            errors[0].help.as_deref(),
            Some("declare it after the type's name: `type Box<a>`")
        );
        let errors = errors_of("type Box<a, a> = Box a");
        assert_eq!(
            errors[0].message,
            "the type parameter `a` is declared more than once"
        );
        let errors = errors_of("type T = T Shape");
        assert_eq!(errors[0].message, "unknown type `Shape`");
        let errors = errors_of(&format!("{OPTION}type T = T Option"));
        assert_eq!(
            errors[0].message,
            "`Option` takes 1 type argument, but 0 were given"
        );
        let errors = errors_of("type P<a, b> = P a b\nx : P<Number> = (P 1 2)");
        assert_eq!(errors[0].help.as_deref(), Some("write `P<a, b>`"));
    }
}
