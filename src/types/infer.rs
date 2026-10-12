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

use std::collections::HashMap;
use std::rc::Rc;

use crate::builtins::Builtin;
use crate::diagnostics::Diagnostic;
use crate::hir::{self, Body, ExprKind, GlobalDef, GlobalId, LocalId, Unit};
use crate::source::{SourceMap, Span};
use crate::syntax::ast::{TypeExpr, TypeExprKind};

use super::ty::{Scheme, TyVar, Type, constructor_arity, display_together};

/// The result of checking one unit.
#[derive(Debug, Default)]
pub struct Typed {
    /// Schemes of the unit's definitions, indexed like `Unit::defs`.
    pub def_schemes: Vec<Scheme>,
    /// Types of the unit's top-level expressions, in order.
    pub entry_schemes: Vec<Scheme>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Type-checks a unit. `prior` holds the schemes of globals from earlier
/// units, indexed by [`GlobalId`]; the unit's own ids start at `prior.len()`.
pub fn check_unit(unit: &Unit, prior: &[Scheme]) -> Typed {
    let mut checker = Checker::new(prior);
    let first = prior.len();

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
            _ => Scheme {
                arity: 1,
                ty: Type::Gen(0),
            },
        })
        .collect();

    let entry_schemes = unit
        .entries
        .iter()
        .map(|body| checker.check_entry(body))
        .collect();

    Typed {
        def_schemes,
        entry_schemes,
        diagnostics: checker.diagnostics,
    }
}

/// Infers the type of a single expression body (the REPL's `:type`).
pub fn check_expr(body: &Body, prior: &[Scheme]) -> (Scheme, Vec<Diagnostic>) {
    let mut checker = Checker::new(prior);
    let scheme = checker.check_entry(body);
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
    let ann = crate::syntax::parse_type(file, builtin.signature())
        .unwrap_or_else(|_| panic!("malformed signature for `{builtin}`"));
    let mut checker = Checker::new(&[]);
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
    Unbound { level: u32 },
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
    globals: HashMap<GlobalId, GlobalType>,
    vars: Vec<VarState>,
    level: u32,
    rigid_levels: HashMap<Rc<str>, u32>,
    diagnostics: Vec<Diagnostic>,
}

struct Mismatch {
    expected: Type,
    found: Type,
    reason: MismatchReason,
}

enum MismatchReason {
    Different,
    Infinite,
    Escape(Rc<str>),
}

impl<'a> Checker<'a> {
    fn new(prior: &'a [Scheme]) -> Self {
        Self {
            prior,
            globals: HashMap::new(),
            vars: Vec::new(),
            level: 0,
            rigid_levels: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    // ---- variables, levels and unification -------------------------------

    fn fresh(&mut self) -> Type {
        let var = TyVar(self.vars.len() as u32);
        self.vars.push(VarState::Unbound { level: self.level });
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
        let VarState::Unbound { level } = self.vars[var.0 as usize] else {
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
                expected: expected.clone(),
                found: found.clone(),
                reason: MismatchReason::Infinite,
            });
        }
        self.vars[var.0 as usize] = VarState::Link(ty.clone());
        Ok(())
    }

    /// Occurs check; also lowers the levels of variables in `ty` to `level`
    /// so they are not generalized too early.
    fn occurs_and_adjust(&mut self, var: TyVar, level: u32, ty: &Type) -> bool {
        match self.shallow(ty) {
            Type::Var(v) if v == var => true,
            Type::Var(v) => {
                if let VarState::Unbound { level: other } = self.vars[v.0 as usize] {
                    if other > level {
                        self.vars[v.0 as usize] = VarState::Unbound { level };
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

    /// Quantifies over variables created at a deeper level than the current one.
    fn generalize(&mut self, ty: &Type) -> Scheme {
        let ty = self.zonk(ty);
        let mut mapping: HashMap<TyVar, u32> = HashMap::new();
        let ty = self.generalize_in(&ty, &mut mapping, &HashMap::new());
        Scheme {
            arity: mapping.len() as u32,
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
                VarState::Unbound { level } if level > self.level => {
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
        let fresh: Vec<Type> = (0..scheme.arity).map(|_| self.fresh()).collect();
        substitute_gens(&scheme.ty, &fresh)
    }

    // ---- annotations --------------------------------------------------------

    /// Converts an annotation whose free variables are all quantified.
    /// Reports ill-formed types and returns `None` for them.
    fn annotation_scheme(&mut self, ann: &TypeExpr) -> Option<Rigid> {
        let mut names: Vec<Rc<str>> = Vec::new();
        let mut ok = true;
        let ty = self.convert(
            ann,
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
        let rigids: HashMap<Rc<str>, u32> = names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i as u32))
            .collect();
        let scheme = Scheme {
            arity: names.len() as u32,
            ty: self.generalize_in(&ty, &mut HashMap::new(), &rigids),
        };
        Some(Rigid { scheme, ty, names })
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
                match constructor_arity(&name.name) {
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
                    Some(arity) if arity != args.len() => {
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
                            .with_help(if arity == 1 {
                                format!("write `{}<a>`", name.name)
                            } else {
                                format!("write `{}`", name.name)
                            }),
                        );
                        return None;
                    }
                    Some(_) => Type::Con(
                        name.name.clone(),
                        converted.into_iter().collect::<Option<_>>()?,
                    ),
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
                    for name in &rigid.names {
                        self.rigid_levels.insert(name.clone(), self.level);
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

    fn check_entry(&mut self, body: &Body) -> Scheme {
        self.enter();
        let mut ctx = BodyCtx::default();
        let ty = self.infer(&body.expr, &mut ctx);
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
                Some(scheme) => self.instantiate(&scheme),
                None => self.fresh(),
            },
            ExprKind::Global(id) => match self.global_type(*id) {
                GlobalType::Known(scheme) => self.instantiate(&scheme),
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
                self.instantiate(&scheme)
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
        ann: &TypeExpr,
        ctx: &mut BodyCtx,
    ) -> Option<(Scheme, Type, Vec<Rc<str>>)> {
        let mut ok = true;
        let mut new_names: Vec<Rc<str>> = Vec::new();
        let level = self.level;
        let ty = self.convert(
            ann,
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
        let ty = match ty {
            Some(ty) if ok => ty,
            _ => {
                ctx.failed = true;
                return None;
            }
        };
        for name in &new_names {
            self.rigid_levels.insert(name.clone(), level);
        }
        let rigids: HashMap<Rc<str>, u32> = new_names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i as u32))
            .collect();
        let zonked = self.zonk(&ty);
        let scheme = Scheme {
            arity: new_names.len() as u32,
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
                self.infer(arg, ctx);
            }
            return Type::unit();
        }
        let mut current = self.infer(func, ctx);
        for (index, arg) in args.iter().enumerate() {
            let (param, result) = match self.shallow(&current) {
                Type::Fun(param, result) => (*param, *result),
                Type::Var(_) => {
                    let param = self.fresh();
                    let result = self.fresh();
                    self.unify(&current, &Type::fun(param.clone(), result.clone()))
                        .unwrap_or_else(|_| {
                            unreachable!("a fresh variable unifies with a function type")
                        });
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
        };
        self.error(ctx, diagnostic);
    }
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
        let typed = check_unit(&resolved.unit, &[]);
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
        assert_eq!(type_of_entry(r#"(map show [true])"#), "List<String>");
        assert_eq!(type_of_entry("(foldr cons [] [1])"), "List<Number>");
    }

    #[test]
    fn partial_application() {
        assert_eq!(type_of_entry("(add 1)"), "Number -> Number");
        assert_eq!(type_of_entry("(if true)"), "a -> a -> a");
    }

    #[test]
    fn recursion_and_mutual_recursion() {
        assert_eq!(
            types_of(r"fact = \n. (if (eq n 0) 1 (mul n (fact (sub n 1))))"),
            ["fact : Number -> Number"]
        );
        assert_eq!(
            types_of(
                r"even = \n. (if (eq n 0) true (odd (sub n 1)))  odd = \n. (if (eq n 0) false (even (sub n 1)))"
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
        assert_eq!(type_of_entry(r#"(print 1 "a" [true])"#), "Unit");
        let errors = errors_of("(map print [1])");
        assert_eq!(errors[0].message, "`print` must be called directly");
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
        assert_eq!(builtin_scheme(Builtin::True).to_string(), "Bool");
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
        let prior = [Scheme {
            arity: 1,
            ty: Type::fun(Type::Gen(0), Type::list(Type::Gen(0))),
        }];
        let typed = check_unit(&resolved.unit, &prior);
        assert_eq!(typed.entry_schemes[0].to_string(), "List<Number>");
    }
}
