//! Warnings: checks that run after type checking, on units without errors.
//!
//! A warning points at code that is accepted but probably not what was
//! meant: a binder that is never used, a `match` arm no value reaches, a
//! definition that hides a builtin, or an action whose result a program
//! throws away. Warnings never stop a program; `--deny-warnings` turns them
//! into errors.
//!
//! A binder whose name starts with `_` is never reported as unused.

use std::collections::HashMap;

use crate::builtins::Builtin;
use crate::diagnostics::Diagnostic;
use crate::hir::{self, Body, ExprKind, GlobalId, LocalId, MatchSource, TyConId, Unit};
use crate::source::FileId;
use crate::types::exhaustive::unreachable;
use crate::types::{Scheme, TopLevel, Type, TypeEnv};

/// What to check besides the checks every unit gets.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub top_level: TopLevel,
    /// The root file of a program, whose definitions are reported when
    /// nothing uses them. Definitions of `use`d files, and of REPL input,
    /// are meant to be used by code that is not there yet.
    pub program_root: Option<FileId>,
}

/// The warnings for a unit that type-checked, in source order.
/// `entry_schemes` holds the types of the unit's top-level expressions.
pub fn lint(
    unit: &Unit,
    entry_schemes: &[Scheme],
    types: &TypeEnv,
    options: Options,
) -> Vec<Diagnostic> {
    let mut warnings = Vec::new();
    for def in &unit.defs {
        if Builtin::lookup(&def.name).is_some() {
            warnings.push(
                Diagnostic::warning(format!("`{}` shadows the builtin `{}`", def.name, def.name))
                    .at(def.name_span)
                    .with_label("defined here")
                    .with_note(format!(
                        "every use of `{}` refers to this definition instead",
                        def.name
                    ))
                    .with_help("rename it to keep the builtin available"),
            );
        }
    }
    for body in unit.defs.iter().map(|def| &def.body).chain(&unit.entries) {
        BodyLinter::new(body, types, &mut warnings).run();
    }
    if let Some(root) = options.program_root {
        unused_definitions(unit, root, &mut warnings);
    }
    if options.top_level == TopLevel::Actions {
        for (body, scheme) in unit.entries.iter().zip(entry_schemes) {
            discarded_result(body, scheme, &mut warnings);
        }
    }
    warnings.sort_by_key(|warning| warning.span.map(|span| (span.file, span.start)));
    warnings
}

fn silenced(name: &str) -> bool {
    name.starts_with('_')
}

/// Reports the definitions of the root file that no top-level expression
/// reaches. A program without top-level expressions is a library being
/// checked, so nothing is reported for it.
fn unused_definitions(unit: &Unit, root: FileId, warnings: &mut Vec<Diagnostic>) {
    if unit.entries.is_empty() {
        return;
    }
    let index: HashMap<GlobalId, usize> = unit
        .defs
        .iter()
        .enumerate()
        .map(|(i, def)| (def.id, i))
        .collect();
    let mut reached = vec![false; unit.defs.len()];
    let mut pending: Vec<GlobalId> = Vec::new();
    for entry in &unit.entries {
        entry.expr.for_each_global(&mut |id| pending.push(id));
    }
    for def in &unit.defs {
        if silenced(&def.name) || def.name_span.file != root {
            pending.push(def.id);
        }
    }
    while let Some(id) = pending.pop() {
        let Some(&i) = index.get(&id) else { continue };
        if !std::mem::replace(&mut reached[i], true) {
            unit.defs[i]
                .body
                .expr
                .for_each_global(&mut |id| pending.push(id));
        }
    }
    for (def, reached) in unit.defs.iter().zip(reached) {
        if !reached {
            warnings.push(
                Diagnostic::warning(format!("`{}` is defined but never used", def.name))
                    .at(def.name_span)
                    .with_label("never used")
                    .with_help(format!(
                        "remove it, or rename it to `_{}` to keep it",
                        def.name
                    )),
            );
        }
    }
}

/// Reports a top-level action whose result means something, since a
/// program throws it away. Results made only of `()` carry nothing, so
/// `(sequence [(println "a"), (println "b")])` is fine.
fn discarded_result(body: &Body, scheme: &Scheme, warnings: &mut Vec<Diagnostic>) {
    let Some(result) = scheme.ty.io_result().filter(|ty| carries_information(ty)) else {
        return;
    };
    let result = Scheme {
        arity: scheme.arity,
        bounds: scheme.bounds.clone(),
        ty: result.clone(),
    };
    warnings.push(
        Diagnostic::warning("the result of this action is discarded")
            .at(body.expr.span)
            .with_label(format!("this produces a value of type `{result}`"))
            .with_note("a program performs its top-level actions and ignores what they produce")
            .with_help("use the result with `bind`, or write `(then ... (pure ()))` to drop it"),
    );
}

fn carries_information(ty: &Type) -> bool {
    match ty {
        Type::Con(con, _) if con.id == TyConId::UNIT => false,
        Type::Con(con, args) if con.id == TyConId::LIST || con.id.tuple_arity().is_some() => {
            args.iter().any(carries_information)
        }
        Type::Con(..) | Type::Fun(..) => true,
        Type::Var(_) | Type::Gen(_) | Type::Rigid(_) => false,
    }
}

/// Finds the unused binders and unreachable arms of one body.
struct BodyLinter<'a> {
    body: &'a Body,
    types: &'a TypeEnv,
    warnings: &'a mut Vec<Diagnostic>,
    used: Vec<bool>,
    /// The `let` binders whose values are being visited: a recursive use
    /// does not count as a use.
    defining: Vec<LocalId>,
}

impl<'a> BodyLinter<'a> {
    fn new(body: &'a Body, types: &'a TypeEnv, warnings: &'a mut Vec<Diagnostic>) -> Self {
        Self {
            body,
            types,
            warnings,
            used: vec![false; body.locals.len()],
            defining: Vec::new(),
        }
    }

    fn run(mut self) {
        let body = self.body;
        self.expr(&body.expr);
        for (local, used) in self.body.locals.iter().zip(&self.used) {
            if !used && !silenced(&local.name) {
                self.warnings.push(
                    Diagnostic::warning(format!("unused variable `{}`", local.name))
                        .at(local.span)
                        .with_label("never used")
                        .with_help(format!(
                            "if that is intended, rename it to `_{}`",
                            local.name
                        )),
                );
            }
        }
    }

    fn expr(&mut self, expr: &hir::Expr) {
        match &expr.kind {
            ExprKind::Local(id) => {
                if !self.defining.contains(id) {
                    self.used[id.index()] = true;
                }
            }
            ExprKind::Lambda { body, .. } => self.expr(body),
            ExprKind::App { func, args } => {
                self.expr(func);
                for arg in args {
                    self.expr(arg);
                }
            }
            ExprKind::List(items) => {
                for item in items {
                    self.expr(item);
                }
            }
            ExprKind::Let {
                local, value, body, ..
            } => {
                self.defining.push(*local);
                self.expr(value);
                self.defining.pop();
                self.expr(body);
            }
            ExprKind::Match {
                scrutinee,
                arms,
                source,
            } => {
                self.expr(scrutinee);
                if *source == MatchSource::Match {
                    let patterns: Vec<_> = arms.iter().map(|arm| &arm.pattern).collect();
                    for i in unreachable(self.types, &patterns) {
                        self.warnings.push(
                            Diagnostic::warning("unreachable `match` arm")
                                .at(arms[i].pattern.span)
                                .with_label("no value reaches this arm")
                                .with_note("the arms above it match every value it would")
                                .with_help(
                                    "remove the arm, or move it above the arms that cover it",
                                ),
                        );
                    }
                }
                for arm in arms {
                    self.expr(&arm.body);
                }
            }
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Unit
            | ExprKind::Global(_)
            | ExprKind::Builtin(_)
            | ExprKind::Con(_)
            | ExprKind::Error => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_text;
    use crate::resolve::{EmptyScope, resolve};
    use crate::source::SourceMap;
    use crate::types::check_unit;

    /// The warning messages for `text`, compiled as a program when
    /// `program`, and as REPL input otherwise.
    fn warnings(text: &str, program: bool) -> Vec<String> {
        let mut sources = SourceMap::new();
        let loaded = load_text(&mut sources, "<test>", text);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        let resolved = resolve(loaded.items, &EmptyScope);
        assert!(
            resolved.diagnostics.is_empty(),
            "{:?}",
            resolved.diagnostics
        );
        let top_level = if program {
            TopLevel::Actions
        } else {
            TopLevel::Any
        };
        let typed = check_unit(&resolved.unit, &[], &TypeEnv::new(), top_level);
        assert!(typed.diagnostics.is_empty(), "{:?}", typed.diagnostics);
        let options = Options {
            top_level,
            program_root: loaded.root.filter(|_| program),
        };
        lint(&resolved.unit, &typed.entry_schemes, &typed.types, options)
            .into_iter()
            .map(|warning| warning.message)
            .collect()
    }

    #[test]
    fn unused_binders_of_every_kind() {
        assert_eq!(
            warnings(
                r"f = \x. \y. match (Some x) | Some z -> 1 | None -> (let w = 2 in 3)",
                false
            ),
            [
                "unused variable `y`",
                "unused variable `z`",
                "unused variable `w`"
            ]
        );
        assert_eq!(
            warnings(r"f = \p. let (a, b) = p in a", false),
            ["unused variable `b`"]
        );
        assert_eq!(
            warnings(r"f = \_x. let _y = 1 in \_. 2", false),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_recursive_use_is_not_a_use() {
        assert_eq!(
            warnings(r"x = let loop = \n. (loop n) in 1", false),
            ["unused variable `loop`"]
        );
        assert_eq!(
            warnings(r"x = let loop = \n. (loop n) in (loop 1)", false),
            Vec::<String>::new()
        );
    }

    #[test]
    fn definitions_nothing_reaches_are_unused_in_programs() {
        let text = "a = b\nb = 1\nc = d\nd = c\ne = \\n. (e n)\n_f = g\ng = 2\n(println (show a))";
        assert_eq!(
            warnings(text, true),
            [
                "`c` is defined but never used",
                "`d` is defined but never used",
                "`e` is defined but never used"
            ]
        );
        // REPL input and libraries are used later.
        assert_eq!(warnings(text, false), Vec::<String>::new());
        assert_eq!(warnings("c = 1", true), Vec::<String>::new());
    }

    #[test]
    fn definitions_that_shadow_builtins() {
        assert_eq!(
            warnings("length = 1", false),
            ["`length` shadows the builtin `length`"]
        );
    }

    #[test]
    fn unreachable_arms() {
        assert_eq!(
            warnings(
                r"f = \n. match n | 1 -> 1 | _ -> 2 | 1 -> 3 | 3 -> 4",
                false
            ),
            ["unreachable `match` arm", "unreachable `match` arm"]
        );
        assert_eq!(
            warnings(
                r"f = \o. match o | Some True -> 1 | Some False -> 2 | Some _ -> 3 | None -> 4",
                false
            ),
            ["unreachable `match` arm"]
        );
        assert_eq!(
            warnings(
                r"f = \o. match o | Some 1 -> 1 | Some _ -> 2 | None -> 3",
                false
            ),
            Vec::<String>::new()
        );
    }

    #[test]
    fn discarded_results_of_top_level_actions() {
        assert_eq!(
            warnings("(pure 1)\n(sequence [(pure (1, ()))])", true),
            [
                "the result of this action is discarded",
                "the result of this action is discarded"
            ]
        );
        // Nothing of value is lost.
        assert_eq!(
            warnings(
                "(println \"a\")\n(sequence [(println \"a\")])\n(pure [])\n(pure ((), ()))",
                true
            ),
            Vec::<String>::new()
        );
        // The REPL shows the result.
        assert_eq!(warnings("(pure 1)", false), Vec::<String>::new());
    }
}
