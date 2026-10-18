//! Name resolution: surface AST to [`hir`].
//!
//! Scoping, innermost first: local binders, definitions of the current unit
//! (order-independent, so definitions may be mutually recursive), globals
//! from earlier units (REPL inputs), then builtins. A definition may shadow a
//! builtin of the same name.
//!
//! The case of a name's first letter decides what it can be: lowercase
//! names (and names starting with `_`) are values, uppercase names are
//! constructors. Constructors live in their own namespace: those of the
//! current unit, then those of earlier units, then the built-in ones.

use std::collections::HashMap;
use std::rc::Rc;

use crate::builtins::Builtin;
use crate::diagnostics::Diagnostic;
use crate::hir::{
    self, Body, ConId, CtorDef, GlobalDef, GlobalId, LocalId, LocalInfo, MatchSource, TyConId,
    TypeDef, Unit,
};
use crate::source::Span;
use crate::syntax::ast::{self, Item};
use crate::syntax::parser::is_lower;
use crate::types::env::{BUILTIN_TYPES, builtin_constructors};

/// Globals, types and constructors visible from earlier compilation units.
pub trait GlobalScope {
    fn lookup(&self, name: &str) -> Option<GlobalId>;
    fn names(&self) -> Vec<Rc<str>>;
    /// The id the next new global will receive.
    fn next_id(&self) -> u32;

    /// The current constructor with this name, including built-in ones.
    fn lookup_constructor(&self, name: &str) -> Option<ConId> {
        builtin_constructors().find_map(|(id, con)| (con.name == name).then_some(id))
    }
    fn constructor_names(&self) -> Vec<Rc<str>> {
        builtin_constructors()
            .filter(|(id, _)| id.tuple_arity().is_none())
            .map(|(_, con)| Rc::from(con.name))
            .collect()
    }
    /// The id the next new type will receive.
    fn next_type_id(&self) -> u32 {
        BUILTIN_TYPES.len() as u32
    }
    /// The id the next new constructor will receive.
    fn next_constructor_id(&self) -> u32 {
        builtin_constructors().count() as u32
    }
}

/// No earlier units: only builtins.
pub struct EmptyScope;

impl GlobalScope for EmptyScope {
    fn lookup(&self, _: &str) -> Option<GlobalId> {
        None
    }
    fn names(&self) -> Vec<Rc<str>> {
        Vec::new()
    }
    fn next_id(&self) -> u32 {
        0
    }
}

pub struct Resolved {
    pub unit: Unit,
    pub diagnostics: Vec<Diagnostic>,
}

/// Names a unit declares, with the ids allocated for them.
#[derive(Default)]
struct UnitNames {
    globals: HashMap<Rc<str>, (GlobalId, Span)>,
    constructors: HashMap<Rc<str>, (ConId, Span)>,
}

pub fn resolve(items: Vec<Item>, scope: &dyn GlobalScope) -> Resolved {
    let mut diagnostics = Vec::new();
    let mut names = UnitNames::default();
    let mut next = scope.next_id();
    let mut defs = Vec::new();
    let mut entries = Vec::new();

    // First pass: allocate ids so definitions can refer to each other in any order.
    for item in &items {
        if let Item::Def(def) = item {
            if !is_lower(&def.name.name) {
                diagnostics.push(uppercase_binder(&def.name));
                continue;
            }
            if let Some(&(_, first)) = names.globals.get(&def.name.name) {
                diagnostics.push(defined_twice(&def.name, first));
                continue;
            }
            names
                .globals
                .insert(def.name.name.clone(), (GlobalId(next), def.name.span));
            next += 1;
        }
    }
    let types = declare_types(&items, scope, &mut names, &mut diagnostics);

    for item in items {
        match item {
            Item::Def(def) => {
                let Some(&(id, span)) = names.globals.get(&def.name.name) else {
                    continue; // an uppercase name, already reported
                };
                if span != def.name.span {
                    continue; // a duplicate, already reported
                }
                let mut resolver = BodyResolver::new(scope, &names, &mut diagnostics);
                let expr = resolver.expr(def.value);
                let locals = resolver.locals;
                defs.push(GlobalDef {
                    id,
                    name: def.name.name,
                    name_span: def.name.span,
                    ann: def.ann,
                    body: Body { expr, locals },
                });
            }
            Item::Expr(expr) => {
                let mut resolver = BodyResolver::new(scope, &names, &mut diagnostics);
                let expr = resolver.expr(expr);
                let locals = resolver.locals;
                entries.push(Body { expr, locals });
            }
            Item::Type(_) => {}
            Item::Use { .. } => unreachable!("`use` items are expanded by the loader"),
        }
    }

    Resolved {
        unit: Unit {
            types,
            defs,
            entries,
        },
        diagnostics,
    }
}

/// Allocates ids for the unit's types and constructors, in source order.
fn declare_types(
    items: &[Item],
    scope: &dyn GlobalScope,
    names: &mut UnitNames,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<TypeDef> {
    let mut type_names: HashMap<Rc<str>, Span> = HashMap::new();
    let mut next_type = scope.next_type_id();
    let mut next_con = scope.next_constructor_id();
    let mut types = Vec::new();
    for item in items {
        let Item::Type(decl) = item else { continue };
        if let Some(builtin) = BUILTIN_TYPES.iter().find(|b| *b.name == *decl.name.name) {
            diagnostics.push(
                Diagnostic::error(format!("`{}` is a built-in type", builtin.name))
                    .at(decl.name.span)
                    .with_label("cannot be redefined"),
            );
            continue;
        }
        if let Some(&first) = type_names.get(&decl.name.name) {
            diagnostics.push(
                Diagnostic::error(format!(
                    "the type `{}` is defined more than once",
                    decl.name.name
                ))
                .at(decl.name.span)
                .with_label("redefined here")
                .with_secondary(first, "first defined here"),
            );
            continue;
        }
        type_names.insert(decl.name.name.clone(), decl.name.span);
        let mut constructors = Vec::new();
        for ctor in &decl.constructors {
            let name = &ctor.name;
            if let Some(&(_, first)) = names.constructors.get(&name.name) {
                diagnostics.push(
                    Diagnostic::error(format!(
                        "the constructor `{}` is defined more than once",
                        name.name
                    ))
                    .at(name.span)
                    .with_label("redefined here")
                    .with_secondary(first, "first defined here"),
                );
                continue;
            }
            if let Some((_, builtin)) =
                builtin_constructors().find(|(_, con)| con.name == &*name.name)
            {
                diagnostics.push(
                    Diagnostic::error(format!("`{}` is a built-in constructor", builtin.name))
                        .at(name.span)
                        .with_label("cannot be redefined"),
                );
                continue;
            }
            let id = ConId(next_con);
            next_con += 1;
            names
                .constructors
                .insert(name.name.clone(), (id, name.span));
            constructors.push(CtorDef {
                id,
                name: name.clone(),
                fields: ctor.fields.clone(),
            });
        }
        types.push(TypeDef {
            id: TyConId(next_type),
            name: decl.name.clone(),
            params: decl.params.clone(),
            constructors,
        });
        next_type += 1;
    }
    types
}

fn defined_twice(name: &ast::Ident, first: Span) -> Diagnostic {
    Diagnostic::error(format!("`{}` is defined more than once", name.name))
        .at(name.span)
        .with_label("redefined here")
        .with_secondary(first, "first defined here")
}

fn uppercase_binder(name: &ast::Ident) -> Diagnostic {
    let lower: String = name
        .name
        .chars()
        .enumerate()
        .map(|(i, c)| if i == 0 { c.to_ascii_lowercase() } else { c })
        .collect();
    Diagnostic::error(format!(
        "`{}` starts with an uppercase letter, but it names a value",
        name.name
    ))
    .at(name.span)
    .with_label("value names start with a lowercase letter")
    .with_help(format!(
        "uppercase names are for types and constructors; rename it to `{lower}`"
    ))
}

struct BodyResolver<'a> {
    scope: &'a dyn GlobalScope,
    names: &'a UnitNames,
    diagnostics: &'a mut Vec<Diagnostic>,
    locals: Vec<LocalInfo>,
    /// Binders currently in scope, innermost last.
    stack: Vec<(Rc<str>, LocalId)>,
}

impl<'a> BodyResolver<'a> {
    fn new(
        scope: &'a dyn GlobalScope,
        names: &'a UnitNames,
        diagnostics: &'a mut Vec<Diagnostic>,
    ) -> Self {
        Self {
            scope,
            names,
            diagnostics,
            locals: Vec::new(),
            stack: Vec::new(),
        }
    }

    fn bind(&mut self, ident: &ast::Ident) -> LocalId {
        if !is_lower(&ident.name) {
            self.diagnostics.push(uppercase_binder(ident));
        }
        let id = self.local(ident.name.clone(), ident.span);
        self.stack.push((ident.name.clone(), id));
        id
    }

    /// A new local binder, not yet in scope.
    fn local(&mut self, name: Rc<str>, span: Span) -> LocalId {
        let id = LocalId(self.locals.len() as u32);
        self.locals.push(LocalInfo { name, span });
        id
    }

    /// Resolves the parameter of a lambda and its body. A pattern other than
    /// a name becomes a parameter without a name that the body matches
    /// against the pattern: `\(x, y). body` is
    /// `\p. match p | (x, y) -> body`, whose `source` says what the lambda
    /// was written as.
    fn lambda(
        &mut self,
        param: ast::Pattern,
        body: ast::Expr,
        source: MatchSource,
    ) -> (LocalId, hir::Expr) {
        if let ast::PatternKind::Var(name) = &param.kind {
            let local = self.bind(name);
            let body = self.expr(body);
            self.stack.pop();
            return (local, body);
        }
        let span = param.span;
        let local = self.local(Rc::from(param.to_string()), span);
        let scrutinee = hir::Expr {
            kind: hir::ExprKind::Local(local),
            span,
        };
        let body_span = body.span;
        let kind = self.single_arm(scrutinee, param, body, source);
        let body = hir::Expr {
            kind,
            span: span.to(body_span),
        };
        (local, body)
    }

    /// `match scrutinee | pattern -> body`.
    fn single_arm(
        &mut self,
        scrutinee: hir::Expr,
        pattern: ast::Pattern,
        body: ast::Expr,
        source: MatchSource,
    ) -> hir::ExprKind {
        let depth = self.stack.len();
        let pattern = self.pattern(pattern, &mut Vec::new());
        let body = self.expr(body);
        self.stack.truncate(depth);
        hir::ExprKind::Match {
            scrutinee: Box::new(scrutinee),
            arms: vec![hir::Arm { pattern, body }],
            source,
        }
    }

    fn lookup(&mut self, ident: &ast::Ident) -> hir::ExprKind {
        if !is_lower(&ident.name) {
            return match self.lookup_constructor(ident) {
                Some(id) => hir::ExprKind::Con(id),
                None => hir::ExprKind::Error,
            };
        }
        if let Some((_, id)) = self
            .stack
            .iter()
            .rev()
            .find(|(name, _)| *name == ident.name)
        {
            return hir::ExprKind::Local(*id);
        }
        if let Some(&(id, _)) = self.names.globals.get(&ident.name) {
            return hir::ExprKind::Global(id);
        }
        if let Some(id) = self.scope.lookup(&ident.name) {
            return hir::ExprKind::Global(id);
        }
        if let Some(builtin) = Builtin::lookup(&ident.name) {
            return hir::ExprKind::Builtin(builtin);
        }
        let mut diagnostic =
            Diagnostic::error(format!("cannot find `{}` in this scope", ident.name))
                .at(ident.span)
                .with_label("not defined");
        if matches!(ident.name.as_ref(), "true" | "false") {
            diagnostic =
                diagnostic.with_help("the booleans are the constructors `True` and `False`");
        } else if let Some(suggestion) = self.suggest(&ident.name) {
            diagnostic = diagnostic.with_help(format!("a similar name exists: `{suggestion}`"));
        }
        self.diagnostics.push(diagnostic);
        hir::ExprKind::Error
    }

    /// Resolves a constructor name, reporting it if it is unknown.
    fn lookup_constructor(&mut self, ident: &ast::Ident) -> Option<ConId> {
        if let Some(&(id, _)) = self.names.constructors.get(&ident.name) {
            return Some(id);
        }
        if let Some(id) = self.scope.lookup_constructor(&ident.name) {
            return Some(id);
        }
        let mut diagnostic = Diagnostic::error(format!("cannot find constructor `{}`", ident.name))
            .at(ident.span)
            .with_label("not defined");
        let candidates = self
            .names
            .constructors
            .keys()
            .cloned()
            .chain(self.scope.constructor_names());
        if let Some(suggestion) = closest(&ident.name, candidates) {
            diagnostic = diagnostic.with_help(format!("a similar name exists: `{suggestion}`"));
        }
        self.diagnostics.push(diagnostic);
        None
    }

    fn suggest(&self, name: &str) -> Option<Rc<str>> {
        let candidates = self
            .stack
            .iter()
            .map(|(n, _)| n.clone())
            .chain(self.names.globals.keys().cloned())
            .chain(self.scope.names())
            .chain(Builtin::ALL.iter().map(|b| Rc::from(b.name())));
        closest(name, candidates)
    }

    fn expr(&mut self, expr: ast::Expr) -> hir::Expr {
        let span = expr.span;
        let kind = match expr.kind {
            ast::ExprKind::Int(n) => hir::ExprKind::Int(n),
            ast::ExprKind::Float(n) => hir::ExprKind::Float(n),
            ast::ExprKind::String(s) => hir::ExprKind::String(s),
            ast::ExprKind::Unit => hir::ExprKind::Unit,
            ast::ExprKind::Var(ident) => self.lookup(&ident),
            ast::ExprKind::Lambda { param, ann, body } => {
                let (param, body) = self.lambda(param, *body, MatchSource::Lambda);
                hir::ExprKind::Lambda {
                    param,
                    ann,
                    body: Box::new(body),
                }
            }
            ast::ExprKind::App { func, args } => hir::ExprKind::App {
                func: Box::new(self.expr(*func)),
                args: args.into_iter().map(|arg| self.expr(arg)).collect(),
            },
            ast::ExprKind::List(items) => {
                hir::ExprKind::List(items.into_iter().map(|item| self.expr(item)).collect())
            }
            // A tuple is its constructor applied to the components.
            ast::ExprKind::Tuple(items) => hir::ExprKind::App {
                func: Box::new(hir::Expr {
                    kind: hir::ExprKind::Con(ConId::tuple(items.len())),
                    span,
                }),
                args: items.into_iter().map(|item| self.expr(item)).collect(),
            },
            ast::ExprKind::Let {
                name,
                ann,
                value,
                body,
            } => {
                let local = self.bind(&name);
                let value = self.expr(*value);
                let body = self.expr(*body);
                self.stack.pop();
                hir::ExprKind::Let {
                    local,
                    ann,
                    value: Box::new(value),
                    body: Box::new(body),
                }
            }
            // Unlike in a plain `let`, the pattern's variables are not in
            // scope in `value`.
            ast::ExprKind::LetPattern {
                pattern,
                value,
                body,
            } => {
                let value = self.expr(*value);
                self.single_arm(value, pattern, *body, MatchSource::Let)
            }
            // `let x <- action in body` is `(bind action \x. body)`. The
            // builtin is used directly, so a definition named `bind` cannot
            // change it. Unlike `let`, `x` is not in scope in `action`.
            ast::ExprKind::Bind {
                pattern,
                ann,
                action,
                body,
            } => {
                let action = self.expr(*action);
                let pattern_span = pattern.span;
                let (param, body) = self.lambda(pattern, *body, MatchSource::Let);
                let continuation = hir::Expr {
                    span: pattern_span.to(body.span),
                    kind: hir::ExprKind::Lambda {
                        param,
                        ann,
                        body: Box::new(body),
                    },
                };
                hir::ExprKind::App {
                    func: Box::new(hir::Expr {
                        kind: hir::ExprKind::Builtin(Builtin::Bind),
                        span,
                    }),
                    args: vec![action, continuation],
                }
            }
            ast::ExprKind::Match { scrutinee, arms } => {
                let scrutinee = self.expr(*scrutinee);
                let arms = arms
                    .into_iter()
                    .map(|arm| {
                        let depth = self.stack.len();
                        let mut bound = Vec::new();
                        let pattern = self.pattern(arm.pattern, &mut bound);
                        let body = self.expr(arm.body);
                        self.stack.truncate(depth);
                        hir::Arm { pattern, body }
                    })
                    .collect();
                hir::ExprKind::Match {
                    scrutinee: Box::new(scrutinee),
                    arms,
                    source: MatchSource::Match,
                }
            }
            ast::ExprKind::Error => hir::ExprKind::Error,
        };
        hir::Expr { kind, span }
    }

    /// Resolves a pattern, binding its variables. `bound` collects the
    /// variables of the whole pattern, which must be distinct.
    fn pattern(&mut self, pattern: ast::Pattern, bound: &mut Vec<ast::Ident>) -> hir::Pattern {
        let kind = match pattern.kind {
            ast::PatternKind::Wild => hir::PatternKind::Wild,
            ast::PatternKind::Var(name) => {
                if let Some(first) = bound.iter().find(|b| b.name == name.name) {
                    self.diagnostics.push(
                        Diagnostic::error(format!(
                            "`{}` is bound more than once in this pattern",
                            name.name
                        ))
                        .at(name.span)
                        .with_label("bound again here")
                        .with_secondary(first.span, "first bound here")
                        .with_help("use `_` for a field you do not need"),
                    );
                }
                bound.push(name.clone());
                hir::PatternKind::Bind(self.bind(&name))
            }
            ast::PatternKind::Con { name, args } => {
                let con = self.lookup_constructor(&name);
                let args = args
                    .into_iter()
                    .map(|arg| self.pattern(arg, bound))
                    .collect();
                match con {
                    Some(con) => hir::PatternKind::Con { con, args },
                    None => hir::PatternKind::Error,
                }
            }
            ast::PatternKind::Int(n) => hir::PatternKind::Int(n),
            ast::PatternKind::Float(n) => hir::PatternKind::Float(n),
            ast::PatternKind::String(s) => hir::PatternKind::String(s),
            ast::PatternKind::Unit => hir::PatternKind::Unit,
            ast::PatternKind::Tuple(items) => hir::PatternKind::Con {
                con: ConId::tuple(items.len()),
                args: items
                    .into_iter()
                    .map(|item| self.pattern(item, bound))
                    .collect(),
            },
        };
        hir::Pattern {
            kind,
            span: pattern.span,
        }
    }
}

/// The candidate closest to `name`, if any is close enough to be a typo.
fn closest(name: &str, candidates: impl Iterator<Item = Rc<str>>) -> Option<Rc<str>> {
    candidates
        .map(|candidate| (strsim::damerau_levenshtein(name, &candidate), candidate))
        .filter(|(distance, candidate)| *distance <= (candidate.len().max(name.len()) / 3).max(1))
        .min_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)))
        .map(|(_, candidate)| candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::ExprKind;
    use crate::loader::load_text;
    use crate::source::SourceMap;

    fn resolve_text(text: &str) -> Resolved {
        let mut sources = SourceMap::new();
        let loaded = load_text(&mut sources, "<test>", text);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        resolve(loaded.items, &EmptyScope)
    }

    #[test]
    fn resolves_locals_globals_and_builtins() {
        let resolved = resolve_text(r"f = \x. (add x g)  g = 1");
        assert!(resolved.diagnostics.is_empty());
        let def = &resolved.unit.defs[0];
        assert_eq!(def.id, GlobalId(0));
        let ExprKind::Lambda { param, body, .. } = &def.body.expr.kind else {
            panic!()
        };
        let ExprKind::App { func, args } = &body.kind else {
            panic!()
        };
        assert_eq!(func.kind, ExprKind::Builtin(Builtin::Add));
        assert_eq!(args[0].kind, ExprKind::Local(*param));
        assert_eq!(args[1].kind, ExprKind::Global(GlobalId(1)));
        assert_eq!(def.body.locals[0].name.as_ref(), "x");
    }

    #[test]
    fn inner_binders_shadow_outer_ones() {
        let resolved = resolve_text(r"f = \x. \x. x");
        let ExprKind::Lambda { body, .. } = &resolved.unit.defs[0].body.expr.kind else {
            panic!()
        };
        let ExprKind::Lambda { param, body, .. } = &body.kind else {
            panic!()
        };
        assert_eq!(body.kind, ExprKind::Local(*param));
        assert_eq!(*param, LocalId(1));
    }

    #[test]
    fn definitions_shadow_builtins() {
        let resolved = resolve_text("add = 1\n(add)");
        assert_eq!(
            resolved.unit.entries[0].expr.kind,
            ExprKind::Global(GlobalId(0))
        );
    }

    #[test]
    fn let_is_recursive() {
        let resolved = resolve_text(r"x = let f = \n. (f n) in f");
        let ExprKind::Let { local, value, .. } = &resolved.unit.defs[0].body.expr.kind else {
            panic!()
        };
        let ExprKind::Lambda { body, .. } = &value.kind else {
            panic!()
        };
        let ExprKind::App { func, .. } = &body.kind else {
            panic!()
        };
        assert_eq!(func.kind, ExprKind::Local(*local));
    }

    #[test]
    fn undefined_names_are_reported_with_suggestions() {
        let resolved = resolve_text("fibonacci = 1\n(fibonaci)\n(zzzzzz)");
        assert_eq!(resolved.diagnostics.len(), 2);
        assert_eq!(
            resolved.diagnostics[0].message,
            "cannot find `fibonaci` in this scope"
        );
        assert_eq!(
            resolved.diagnostics[0].help.as_deref(),
            Some("a similar name exists: `fibonacci`")
        );
        assert!(resolved.diagnostics[1].help.is_none());
        assert_eq!(resolved.unit.entries[1].expr.kind, ExprKind::Error);
    }

    #[test]
    fn builtin_names_are_suggested() {
        let resolved = resolve_text("(lenght [1])");
        assert_eq!(
            resolved.diagnostics[0].help.as_deref(),
            Some("a similar name exists: `length`")
        );
    }

    #[test]
    fn duplicate_definitions_are_errors() {
        let resolved = resolve_text("a = 1\na = 2");
        assert_eq!(resolved.diagnostics.len(), 1);
        assert_eq!(
            resolved.diagnostics[0].message,
            "`a` is defined more than once"
        );
        assert_eq!(resolved.unit.defs.len(), 1);
    }

    #[test]
    fn earlier_units_are_visible_and_ids_continue() {
        struct Previous;
        impl GlobalScope for Previous {
            fn lookup(&self, name: &str) -> Option<GlobalId> {
                (name == "old").then_some(GlobalId(0))
            }
            fn names(&self) -> Vec<Rc<str>> {
                vec![Rc::from("old")]
            }
            fn next_id(&self) -> u32 {
                1
            }
        }
        let mut sources = SourceMap::new();
        let loaded = load_text(&mut sources, "<test>", "new = old");
        let resolved = resolve(loaded.items, &Previous);
        let def = &resolved.unit.defs[0];
        assert_eq!(def.id, GlobalId(1));
        assert_eq!(def.body.expr.kind, ExprKind::Global(GlobalId(0)));
    }

    #[test]
    fn types_and_constructors_get_consecutive_ids() {
        let resolved = resolve_text(
            "type Shape = Circle Int | Rect Int Int
type Unit2 = U
c = (Circle 1)
b = True",
        );
        assert!(
            resolved.diagnostics.is_empty(),
            "{:?}",
            resolved.diagnostics
        );
        let types = &resolved.unit.types;
        assert_eq!(types[0].id, TyConId(15));
        assert_eq!(types[1].id, TyConId(16));
        let ids: Vec<ConId> = types
            .iter()
            .flat_map(|t| t.constructors.iter().map(|c| c.id))
            .collect();
        assert_eq!(ids, [ConId(12), ConId(13), ConId(14)]);
        let ExprKind::App { func, .. } = &resolved.unit.defs[0].body.expr.kind else {
            panic!()
        };
        assert_eq!(func.kind, ExprKind::Con(ConId(12)));
        assert_eq!(
            resolved.unit.defs[1].body.expr.kind,
            ExprKind::Con(ConId::TRUE)
        );
    }

    #[test]
    fn unknown_constructors_are_reported_with_suggestions() {
        let resolved = resolve_text(
            "type Shape = Circle Int
c = (Circel 1)
d = Square",
        );
        assert_eq!(resolved.diagnostics.len(), 2);
        assert_eq!(
            resolved.diagnostics[0].message,
            "cannot find constructor `Circel`"
        );
        assert_eq!(
            resolved.diagnostics[0].help.as_deref(),
            Some("a similar name exists: `Circle`")
        );
        let resolved = resolve_text("x = true");
        assert_eq!(
            resolved.diagnostics[0].help.as_deref(),
            Some("the booleans are the constructors `True` and `False`")
        );
    }

    #[test]
    fn values_must_have_lowercase_names() {
        let resolved = resolve_text(
            "Foo = 1
f = \\X. X",
        );
        let messages: Vec<&str> = resolved
            .diagnostics
            .iter()
            .map(|d| d.message.as_str())
            .collect();
        assert_eq!(
            messages,
            [
                "`Foo` starts with an uppercase letter, but it names a value",
                "`X` starts with an uppercase letter, but it names a value",
                "cannot find constructor `X`",
            ]
        );
    }

    #[test]
    fn duplicate_types_and_constructors_are_errors() {
        let resolved = resolve_text(
            "type A = X
type A = Y
type B = X | Z
type C = True
type List = L",
        );
        let messages: Vec<&str> = resolved
            .diagnostics
            .iter()
            .map(|d| d.message.as_str())
            .collect();
        assert_eq!(
            messages,
            [
                "the type `A` is defined more than once",
                "the constructor `X` is defined more than once",
                "`True` is a built-in constructor",
                "`List` is a built-in type",
            ]
        );
    }

    #[test]
    fn pattern_variables_are_scoped_to_their_arm() {
        let resolved = resolve_text(
            "type P = P Int Int
f = \\p. match p | P x y -> (add x y) | q -> x",
        );
        assert_eq!(resolved.diagnostics.len(), 1);
        assert_eq!(
            resolved.diagnostics[0].message,
            "cannot find `x` in this scope"
        );
        let resolved = resolve_text(
            "type P = P Int Int
f = \\p. match p | P x x -> x",
        );
        assert_eq!(
            resolved.diagnostics[0].message,
            "`x` is bound more than once in this pattern"
        );
    }
}
