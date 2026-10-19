//! HIR to Core lowering: closure conversion, uncurrying, source locations,
//! saturated constructors and match compilation.
//!
//! A `match` is compiled into a decision tree of flat [`Case`]s. Each `Case`
//! evaluates one value, so the order in which the tree inspects values is
//! the order in which a lazy program forces them. The tree follows the
//! semantics of Haskell's pattern matching: arms are tried from top to
//! bottom and the parts of a pattern from left to right, and a value is
//! only evaluated when the first arm that is still possible needs it. So a
//! `match` never evaluates a field that no arm needed to decide.

use std::collections::HashMap;
use std::rc::Rc;

use crate::builtins::Builtin;
use crate::hir::{self, ConId, ExprKind, LocalId, PatternKind};
use crate::source::{SourceMap, Span};
use crate::types::TypeEnv;
use crate::types::env::builtin_constructors;

use super::{
    Alt, AltPat, Body, Case, Constructor, Expr, GlobalBody, Lambda, Literal, Module,
    builtin_constructor,
};

/// Lowers a resolved, type-checked unit. `sources` holds the files its spans
/// point into, and `types` its constructors.
pub fn lower_unit(unit: &hir::Unit, sources: &SourceMap, types: &TypeEnv) -> Module {
    let mut constructors = HashMap::new();
    let mut lower = |body| lower_body(body, sources, types, &mut constructors);
    Module {
        globals: unit
            .defs
            .iter()
            .map(|def| GlobalBody {
                id: def.id,
                name: def.name.clone(),
                body: lower(&def.body),
            })
            .collect(),
        entries: unit.entries.iter().map(&mut lower).collect(),
    }
}

fn lower_body(
    body: &hir::Body,
    sources: &SourceMap,
    types: &TypeEnv,
    constructors: &mut HashMap<ConId, Rc<Constructor>>,
) -> Body {
    let mut lowerer = Lowerer {
        sources,
        types,
        constructors,
        frames: vec![Frame::default()],
    };
    let expr = lowerer.expr(&body.expr);
    let frame = lowerer.frames.pop().expect("the body frame");
    debug_assert!(frame.captures.is_empty(), "a body has no enclosing frame");
    Body {
        frame_size: frame.size,
        expr,
    }
}

/// Slot assignment for one lambda (or body) being lowered.
#[derive(Default)]
struct Frame {
    slots: HashMap<LocalId, u32>,
    /// Enclosing-frame slots captured so far, in capture order.
    captures: Vec<u32>,
    size: u32,
}

impl Frame {
    fn alloc(&mut self, local: LocalId) -> u32 {
        let slot = self.fresh();
        self.slots.insert(local, slot);
        slot
    }

    /// A slot for a value that has no name, such as a constructor's field.
    fn fresh(&mut self) -> u32 {
        let slot = self.size;
        self.size += 1;
        slot
    }
}

struct Lowerer<'a> {
    sources: &'a SourceMap,
    types: &'a TypeEnv,
    /// One shared [`Constructor`] per constructor used.
    constructors: &'a mut HashMap<ConId, Rc<Constructor>>,
    frames: Vec<Frame>,
}

/// One arm of a `match` during compilation: the patterns still to test,
/// one per value in the occurrence list (`None` matches anything), and the
/// variables already bound to slots.
#[derive(Clone)]
struct Row<'h> {
    patterns: Vec<Option<&'h hir::Pattern>>,
    binds: Vec<(LocalId, u32)>,
    body: &'h hir::Expr,
}

/// Whether matching a pattern needs to look at the value.
fn refutable(pattern: &hir::Pattern) -> bool {
    matches!(
        pattern.kind,
        PatternKind::Con { .. }
            | PatternKind::Int(_)
            | PatternKind::Float(_)
            | PatternKind::String(_)
    )
}

fn literal_of(pattern: &hir::Pattern) -> Option<Literal> {
    match &pattern.kind {
        PatternKind::Int(n) => Some(Literal::Int(*n)),
        PatternKind::Float(n) => Some(Literal::Float(*n)),
        PatternKind::String(s) => Some(Literal::String(s.clone())),
        _ => None,
    }
}

impl Lowerer<'_> {
    /// The slot of `local` in frame `depth`, capturing it through every
    /// intermediate lambda if it lives further out.
    fn slot(&mut self, depth: usize, local: LocalId) -> u32 {
        if let Some(&slot) = self.frames[depth].slots.get(&local) {
            return slot;
        }
        assert!(
            depth > 0,
            "local {local:?} is not bound in any enclosing frame"
        );
        let outer = self.slot(depth - 1, local);
        let frame = &mut self.frames[depth];
        // Captures occupy the first slots, so they are allocated before
        // anything else; `lambda` reserves them after lowering the body.
        frame.captures.push(outer);
        let slot = CAPTURE_TAG | (frame.captures.len() as u32 - 1);
        frame.slots.insert(local, slot);
        slot
    }

    fn expr(&mut self, expr: &hir::Expr) -> Rc<Expr> {
        Rc::new(match &expr.kind {
            ExprKind::Int(n) => Expr::Lit(Literal::Int(*n)),
            ExprKind::Float(n) => Expr::Lit(Literal::Float(*n)),
            ExprKind::String(s) => Expr::Lit(Literal::String(s.clone())),
            ExprKind::Unit => Expr::Lit(Literal::Unit),
            ExprKind::Local(local) => Expr::Local(self.slot(self.frames.len() - 1, *local)),
            ExprKind::Global(id) => Expr::Global(*id),
            // Constants of the place they are written, like literals.
            ExprKind::Builtin(Builtin::File) => Expr::Lit(Literal::String(
                self.sources.get(expr.span.file).name().into(),
            )),
            ExprKind::Builtin(Builtin::Dir) => Expr::Lit(Literal::String(
                self.sources.get(expr.span.file).dir_name().into(),
            )),
            ExprKind::Builtin(Builtin::Line) => {
                let file = self.sources.get(expr.span.file);
                Expr::Lit(Literal::Int(
                    file.line_col(expr.span.start as usize).0 as i64,
                ))
            }
            ExprKind::Builtin(builtin) => Expr::Builtin(*builtin),
            ExprKind::Lambda { .. } => Expr::Lambda(Rc::new(self.lambda(expr))),
            ExprKind::Con(con) => return self.constructor_value(*con, expr.span),
            ExprKind::App { func, args } => match func.kind {
                ExprKind::Con(con) if args.len() >= self.constructor(con).arity as usize => {
                    let constructor = self.constructor(con);
                    let (fields, rest) = args.split_at(constructor.arity as usize);
                    let built = Rc::new(Expr::Con {
                        con: constructor,
                        args: fields.iter().map(|arg| self.expr(arg)).collect(),
                        span: expr.span,
                    });
                    if rest.is_empty() {
                        return built;
                    }
                    Expr::App {
                        func: built,
                        args: rest.iter().map(|arg| self.expr(arg)).collect(),
                        span: expr.span,
                    }
                }
                _ => Expr::App {
                    func: self.expr(func),
                    args: args.iter().map(|arg| self.expr(arg)).collect(),
                    span: expr.span,
                },
            },
            ExprKind::Match {
                scrutinee, arms, ..
            } => {
                return self.lower_match(scrutinee, arms, expr.span);
            }
            ExprKind::List(items) => Expr::List {
                items: items.iter().map(|item| self.expr(item)).collect(),
                span: expr.span,
            },
            ExprKind::Let {
                local, value, body, ..
            } => {
                let slot = self.frames.last_mut().expect("a frame").alloc(*local);
                Expr::Let {
                    slot,
                    value: self.expr(value),
                    body: self.expr(body),
                }
            }
            ExprKind::Error => unreachable!("bodies with errors are never lowered"),
        })
    }

    fn constructor(&mut self, id: ConId) -> Rc<Constructor> {
        if id.index() < builtin_constructors().count() {
            return builtin_constructor(id);
        }
        let types = self.types;
        self.constructors
            .entry(id)
            .or_insert_with(|| {
                let info = types.constructor(id);
                Rc::new(Constructor {
                    id,
                    name: info.name.clone(),
                    arity: info.arity as u32,
                })
            })
            .clone()
    }

    /// A constructor used as a value: the value itself if it has no fields,
    /// otherwise a function that builds it.
    fn constructor_value(&mut self, id: ConId, span: Span) -> Rc<Expr> {
        let con = self.constructor(id);
        if con.arity == 0 {
            return Rc::new(Expr::Con {
                con,
                args: Vec::new(),
                span,
            });
        }
        let arity = con.arity;
        let body = Rc::new(Expr::Con {
            con,
            args: (0..arity).map(|slot| Rc::new(Expr::Local(slot))).collect(),
            span,
        });
        Rc::new(Expr::Lambda(Rc::new(Lambda {
            arity,
            captures: Vec::new(),
            frame_size: arity,
            body,
            span,
        })))
    }

    fn current(&mut self) -> &mut Frame {
        self.frames.last_mut().expect("a frame")
    }

    fn lower_match(&mut self, scrutinee: &hir::Expr, arms: &[hir::Arm], span: Span) -> Rc<Expr> {
        let value = self.expr(scrutinee);
        // The tree refers to the scrutinee by its slot; it gets one unless
        // it already is one.
        let (slot, value) = match value.as_ref() {
            Expr::Local(slot) => (*slot, None),
            _ => (self.current().fresh(), Some(value)),
        };
        let rows = arms
            .iter()
            .map(|arm| Row {
                patterns: vec![Some(&arm.pattern)],
                binds: Vec::new(),
                body: &arm.body,
            })
            .collect();
        let tree = self.compile(&[slot], rows, span);
        match value {
            Some(value) => Rc::new(Expr::Let {
                slot,
                value,
                body: tree,
            }),
            None => tree,
        }
    }

    /// Compiles rows whose patterns test the values in `occurrences` (frame
    /// slots) into a decision tree.
    fn compile(&mut self, occurrences: &[u32], rows: Vec<Row<'_>>, span: Span) -> Rc<Expr> {
        let Some(first) = rows.first() else {
            // Exhaustiveness checking rules this out.
            return Rc::new(Expr::App {
                func: Rc::new(Expr::Builtin(Builtin::Error)),
                args: vec![Rc::new(Expr::Lit(Literal::String(
                    "internal error: no arm of this `match` applies".into(),
                )))],
                span,
            });
        };
        // The first arm still possible decides which value to look at next:
        // the leftmost one its pattern needs.
        let Some(column) = first.patterns.iter().position(|p| p.is_some_and(refutable)) else {
            let row = first.clone();
            for (pattern, &slot) in row.patterns.iter().zip(occurrences) {
                if let Some(PatternKind::Bind(local)) = pattern.map(|p| &p.kind) {
                    self.current().slots.insert(*local, slot);
                }
            }
            for &(local, slot) in &row.binds {
                self.current().slots.insert(local, slot);
            }
            return self.expr(row.body);
        };
        let slot = occurrences[column];
        let head = first.patterns[column].expect("a refutable pattern");
        let mut alts = Vec::new();
        let complete = if let PatternKind::Con { con, .. } = &head.kind {
            let mut heads: Vec<ConId> = Vec::new();
            for row in &rows {
                if let Some(PatternKind::Con { con, .. }) = row.patterns[column].map(|p| &p.kind) {
                    if !heads.contains(con) {
                        heads.push(*con);
                    }
                }
            }
            for &head in &heads {
                let constructor = self.constructor(head);
                let fields: Vec<u32> = (0..constructor.arity)
                    .map(|_| self.current().fresh())
                    .collect();
                let mut inner = occurrences[..column].to_vec();
                inner.extend(&fields);
                inner.extend(&occurrences[column + 1..]);
                let specialized = rows
                    .iter()
                    .filter_map(|row| {
                        let mut row = row.clone();
                        let replacement = match row.patterns[column].map(|p| &p.kind) {
                            Some(PatternKind::Con { con, args }) if *con == head => {
                                args.iter().map(Some).collect()
                            }
                            Some(PatternKind::Con { .. }) => return None,
                            other => {
                                if let Some(PatternKind::Bind(local)) = other {
                                    row.binds.push((*local, slot));
                                }
                                vec![None; fields.len()]
                            }
                        };
                        row.patterns.splice(column..=column, replacement);
                        Some(row)
                    })
                    .collect();
                let body = self.compile(&inner, specialized, span);
                alts.push(Alt {
                    pat: AltPat::Con {
                        con: constructor,
                        fields,
                    },
                    body,
                });
            }
            self.types.siblings(*con).len() == heads.len()
        } else {
            let mut heads: Vec<Literal> = Vec::new();
            for row in &rows {
                if let Some(literal) = row.patterns[column].and_then(literal_of) {
                    if !heads.contains(&literal) {
                        heads.push(literal);
                    }
                }
            }
            let mut rest = occurrences.to_vec();
            rest.remove(column);
            for head in heads {
                let specialized = rows
                    .iter()
                    .filter_map(|row| {
                        let mut row = row.clone();
                        let pattern = row.patterns.remove(column);
                        match pattern.and_then(literal_of) {
                            Some(literal) if literal == head => {}
                            Some(_) => return None,
                            None => {
                                if let Some(PatternKind::Bind(local)) = pattern.map(|p| &p.kind) {
                                    row.binds.push((*local, slot));
                                }
                            }
                        }
                        Some(row)
                    })
                    .collect();
                let body = self.compile(&rest, specialized, span);
                alts.push(Alt {
                    pat: AltPat::Lit(head),
                    body,
                });
            }
            false
        };
        let default = (!complete).then(|| {
            let mut rest = occurrences.to_vec();
            rest.remove(column);
            let defaults = rows
                .iter()
                .filter(|row| !row.patterns[column].is_some_and(refutable))
                .map(|row| {
                    let mut row = row.clone();
                    if let Some(PatternKind::Bind(local)) =
                        row.patterns.remove(column).map(|p| &p.kind)
                    {
                        row.binds.push((*local, slot));
                    }
                    row
                })
                .collect();
            self.compile(&rest, defaults, span)
        });
        Rc::new(Expr::Case(Rc::new(Case {
            scrutinee: Rc::new(Expr::Local(slot)),
            alts,
            default,
            span,
        })))
    }

    /// Lowers a chain of nested lambdas into one multi-parameter lambda.
    fn lambda(&mut self, expr: &hir::Expr) -> Lambda {
        self.frames.push(Frame::default());
        let mut arity = 0;
        let mut body = expr;
        while let ExprKind::Lambda {
            param, body: inner, ..
        } = &body.kind
        {
            self.frames
                .last_mut()
                .expect("the lambda frame")
                .alloc(*param);
            arity += 1;
            body = inner;
        }
        let lowered = self.expr(body);
        let frame = self.frames.pop().expect("the lambda frame");
        // Captured slots were tagged during lowering; move them in front.
        let captured = frame.captures.len() as u32;
        let body = shift(&lowered, captured);
        Lambda {
            arity,
            captures: frame.captures,
            frame_size: frame.size + captured,
            body,
            span: expr.span,
        }
    }
}

/// Marks a slot number as "capture index" until the lambda is finished.
const CAPTURE_TAG: u32 = 1 << 31;

/// Rewrites a finished lambda body so captures take slots `0..captured` and
/// parameters and lets follow them.
fn shift(expr: &Rc<Expr>, captured: u32) -> Rc<Expr> {
    let fix = |slot: u32| {
        if slot & CAPTURE_TAG != 0 {
            slot & !CAPTURE_TAG
        } else {
            slot + captured
        }
    };
    Rc::new(match expr.as_ref() {
        Expr::Local(slot) => Expr::Local(fix(*slot)),
        Expr::Lambda(lambda) => Expr::Lambda(Rc::new(Lambda {
            // Inner bodies are already final; only their capture list refers to us.
            captures: lambda.captures.iter().map(|&s| fix(s)).collect(),
            ..lambda.as_ref().clone()
        })),
        Expr::App { func, args, span } => Expr::App {
            func: shift(func, captured),
            args: args.iter().map(|a| shift(a, captured)).collect(),
            span: *span,
        },
        Expr::List { items, span } => Expr::List {
            items: items.iter().map(|i| shift(i, captured)).collect(),
            span: *span,
        },
        Expr::Let { slot, value, body } => Expr::Let {
            slot: fix(*slot),
            value: shift(value, captured),
            body: shift(body, captured),
        },
        Expr::Con { con, args, span } => Expr::Con {
            con: con.clone(),
            args: args.iter().map(|a| shift(a, captured)).collect(),
            span: *span,
        },
        Expr::Case(case) => Expr::Case(Rc::new(Case {
            scrutinee: shift(&case.scrutinee, captured),
            alts: case
                .alts
                .iter()
                .map(|alt| Alt {
                    pat: match &alt.pat {
                        AltPat::Con { con, fields } => AltPat::Con {
                            con: con.clone(),
                            fields: fields.iter().map(|&s| fix(s)).collect(),
                        },
                        AltPat::Lit(literal) => AltPat::Lit(literal.clone()),
                    },
                    body: shift(&alt.body, captured),
                })
                .collect(),
            default: case.default.as_ref().map(|d| shift(d, captured)),
            span: case.span,
        })),
        Expr::Lit(_) | Expr::Global(_) | Expr::Builtin(_) => return expr.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_text;
    use crate::resolve::{EmptyScope, resolve};
    use crate::source::SourceMap;

    fn lower_text(text: &str) -> Module {
        let mut sources = SourceMap::new();
        let loaded = load_text(&mut sources, "<test>", text);
        let resolved = resolve(loaded.items, &EmptyScope);
        assert!(resolved.diagnostics.is_empty());
        let typed = crate::types::check_unit(
            &resolved.unit,
            &[],
            &TypeEnv::new(),
            crate::types::TopLevel::Any,
        );
        assert!(typed.diagnostics.is_empty(), "{:?}", typed.diagnostics);
        lower_unit(&resolved.unit, &sources, &typed.types)
    }

    fn global(text: &str) -> String {
        lower_text(text).globals[0].body.expr.to_string()
    }

    #[test]
    fn nested_lambdas_are_uncurried() {
        assert_eq!(global(r"k = \x. \y. x"), "(lambda/2 [] frame=2 $0)");
    }

    #[test]
    fn source_locations_are_literals_unless_shadowed() {
        // In-memory text has no directory of its own: paths in it are
        // relative to the working directory, like its `use` paths.
        assert_eq!(global("f = __FILE__"), r#""<test>""#);
        assert_eq!(global("d = __DIR__"), r#"".""#);
        assert_eq!(global("l =\n\n    __LINE__"), "3");
        assert_eq!(global(r"f = \__DIR__. __DIR__"), "(lambda/1 [] frame=1 $0)");
    }

    #[test]
    fn free_variables_are_captured_first() {
        // The inner lambda is not directly nested (it is an argument), so it
        // captures `x` and `y` from the outer frame.
        assert_eq!(
            global(r"f = \x. \y. (map \z. (add x (add y z)) [1])"),
            "(lambda/2 [] frame=2 (map (lambda/1 [$0 $1] frame=3 (add $0 (add $1 $2))) [1]))"
        );
    }

    #[test]
    fn captures_pass_through_intermediate_lambdas() {
        // `x` is captured by the middle lambda only so the innermost can capture it.
        assert_eq!(
            global("f = \\x. (g \\y. (g \\z. x))\ng = \\h. h"),
            "(lambda/1 [] frame=1 (@1 (lambda/1 [$0] frame=2 (@1 (lambda/1 [$0] frame=2 $0)))))"
        );
    }

    #[test]
    fn let_slots_follow_parameters_and_captures() {
        assert_eq!(
            global("f = \\x. (g \\y. let z = (add x y) in z)\ng = \\h. h"),
            "(lambda/1 [] frame=1 (@1 (lambda/1 [$0] frame=3 (let $2 = (add $0 $1) in $2))))"
        );
        assert_eq!(
            lower_text("v = let a = 1 in let b = a in b").globals[0]
                .body
                .frame_size,
            2
        );
    }

    #[test]
    fn entries_and_globals_are_kept_in_order() {
        let module = lower_text("a = \"x\"\n(print a)\nb = \"y\"\n(print b)");
        assert_eq!(module.globals.len(), 2);
        assert_eq!(module.entries.len(), 2);
        assert_eq!(
            module.to_string(),
            "@0 a [frame=0] = \"x\"\n@1 b [frame=0] = \"y\"\nentry [frame=0] = (print @0)\nentry [frame=0] = (print @1)\n"
        );
    }

    #[test]
    fn constructors_are_saturated_or_wrapped_in_lambdas() {
        let module = lower_text("a = (Some 1)\nb = Some\nc = None");
        let shown: Vec<String> = module
            .globals
            .iter()
            .map(|g| g.body.expr.to_string())
            .collect();
        assert_eq!(
            shown,
            ["(Some 1)", "(lambda/1 [] frame=1 (Some $0))", "None"]
        );
    }

    #[test]
    fn nested_patterns_become_a_tree_of_flat_cases() {
        assert_eq!(
            global("f = \\o. match o | Some 0 -> 1 | Some n -> n | None -> 2"),
            "(lambda/1 [] frame=2 (case $0 | Some $1 -> (case $1 | 0 -> 1 | _ -> $1) | None -> 2))"
        );
    }

    #[test]
    fn a_scrutinee_that_is_not_a_variable_gets_a_slot() {
        assert_eq!(
            global("h = \\x. match (Some x) | Some y -> y | _ -> x"),
            "(lambda/1 [] frame=3 (let $1 = (Some $0) in (case $1 | Some $2 -> $2 | _ -> $0)))"
        );
    }

    #[test]
    fn case_slots_follow_captures() {
        assert_eq!(
            global("k = \\o. \\y. (map \\z. (match o | Some v -> (add v z) | None -> y) [1])"),
            "(lambda/2 [] frame=2 (map (lambda/1 [$0 $1] frame=4 (case $0 | Some $3 -> (add $3 $2) | None -> $1)) [1]))"
        );
    }
}
