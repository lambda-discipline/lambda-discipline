//! Name resolution: surface AST to [`hir`].
//!
//! Scoping, innermost first: local binders, definitions of the current unit
//! (order-independent, so definitions may be mutually recursive), globals
//! from earlier units (REPL inputs), then builtins. A definition may shadow a
//! builtin of the same name.

use std::collections::HashMap;
use std::rc::Rc;

use crate::builtins::Builtin;
use crate::diagnostics::Diagnostic;
use crate::hir::{self, Body, GlobalDef, GlobalId, LocalId, LocalInfo, Unit};
use crate::source::Span;
use crate::syntax::ast::{self, Item};

/// Globals visible from earlier compilation units.
pub trait GlobalScope {
    fn lookup(&self, name: &str) -> Option<GlobalId>;
    fn names(&self) -> Vec<Rc<str>>;
    /// The id the next new global will receive.
    fn next_id(&self) -> u32;
}

/// No earlier units.
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

pub fn resolve(items: Vec<Item>, scope: &dyn GlobalScope) -> Resolved {
    let mut diagnostics = Vec::new();
    let mut unit_names: HashMap<Rc<str>, (GlobalId, Span)> = HashMap::new();
    let mut next = scope.next_id();
    let mut defs = Vec::new();
    let mut entries = Vec::new();

    // First pass: allocate ids so definitions can refer to each other in any order.
    for item in &items {
        if let Item::Def(def) = item {
            if let Some(&(_, first)) = unit_names.get(&def.name.name) {
                diagnostics.push(
                    Diagnostic::error(format!("`{}` is defined more than once", def.name.name))
                        .at(def.name.span)
                        .with_label("redefined here")
                        .with_secondary(first, "first defined here"),
                );
                continue;
            }
            unit_names.insert(def.name.name.clone(), (GlobalId(next), def.name.span));
            next += 1;
        }
    }

    for item in items {
        match item {
            Item::Def(def) => {
                let (id, span) = unit_names[&def.name.name];
                if span != def.name.span {
                    continue; // a duplicate, already reported
                }
                let mut resolver = BodyResolver::new(scope, &unit_names, &mut diagnostics);
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
                let mut resolver = BodyResolver::new(scope, &unit_names, &mut diagnostics);
                let expr = resolver.expr(expr);
                let locals = resolver.locals;
                entries.push(Body { expr, locals });
            }
            Item::Use { .. } => unreachable!("`use` items are expanded by the loader"),
        }
    }

    Resolved {
        unit: Unit { defs, entries },
        diagnostics,
    }
}

struct BodyResolver<'a> {
    scope: &'a dyn GlobalScope,
    unit_names: &'a HashMap<Rc<str>, (GlobalId, Span)>,
    diagnostics: &'a mut Vec<Diagnostic>,
    locals: Vec<LocalInfo>,
    /// Binders currently in scope, innermost last.
    stack: Vec<(Rc<str>, LocalId)>,
}

impl<'a> BodyResolver<'a> {
    fn new(
        scope: &'a dyn GlobalScope,
        unit_names: &'a HashMap<Rc<str>, (GlobalId, Span)>,
        diagnostics: &'a mut Vec<Diagnostic>,
    ) -> Self {
        Self {
            scope,
            unit_names,
            diagnostics,
            locals: Vec::new(),
            stack: Vec::new(),
        }
    }

    fn bind(&mut self, ident: &ast::Ident) -> LocalId {
        let id = LocalId(self.locals.len() as u32);
        self.locals.push(LocalInfo {
            name: ident.name.clone(),
            span: ident.span,
        });
        self.stack.push((ident.name.clone(), id));
        id
    }

    fn lookup(&mut self, ident: &ast::Ident) -> hir::ExprKind {
        if let Some((_, id)) = self
            .stack
            .iter()
            .rev()
            .find(|(name, _)| *name == ident.name)
        {
            return hir::ExprKind::Local(*id);
        }
        if let Some(&(id, _)) = self.unit_names.get(&ident.name) {
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
        if let Some(suggestion) = self.suggest(&ident.name) {
            diagnostic = diagnostic.with_help(format!("a similar name exists: `{suggestion}`"));
        }
        self.diagnostics.push(diagnostic);
        hir::ExprKind::Error
    }

    fn suggest(&self, name: &str) -> Option<Rc<str>> {
        let candidates = self
            .stack
            .iter()
            .map(|(n, _)| n.clone())
            .chain(self.unit_names.keys().cloned())
            .chain(self.scope.names())
            .chain(Builtin::ALL.iter().map(|b| Rc::from(b.name())));
        candidates
            .map(|candidate| (strsim::damerau_levenshtein(name, &candidate), candidate))
            .filter(|(distance, candidate)| {
                *distance <= (candidate.len().max(name.len()) / 3).max(1)
            })
            .min_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)))
            .map(|(_, candidate)| candidate)
    }

    fn expr(&mut self, expr: ast::Expr) -> hir::Expr {
        let span = expr.span;
        let kind = match expr.kind {
            ast::ExprKind::Number(n) => hir::ExprKind::Number(n),
            ast::ExprKind::String(s) => hir::ExprKind::String(s),
            ast::ExprKind::Unit => hir::ExprKind::Unit,
            ast::ExprKind::Var(ident) => self.lookup(&ident),
            ast::ExprKind::Lambda { param, ann, body } => {
                let param = self.bind(&param);
                let body = self.expr(*body);
                self.stack.pop();
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
            // `let x <- action in body` is `(bind action \x. body)`. The
            // builtin is used directly, so a definition named `bind` cannot
            // change it. Unlike `let`, `x` is not in scope in `action`.
            ast::ExprKind::Bind {
                name,
                ann,
                action,
                body,
            } => {
                let action = self.expr(*action);
                let param = self.bind(&name);
                let body = self.expr(*body);
                self.stack.pop();
                let continuation = hir::Expr {
                    span: name.span.to(body.span),
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
            ast::ExprKind::Error => hir::ExprKind::Error,
        };
        hir::Expr { kind, span }
    }
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
}
