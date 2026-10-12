//! HIR to Core lowering: closure conversion and uncurrying.

use std::collections::HashMap;
use std::rc::Rc;

use crate::hir::{self, ExprKind, LocalId};

use super::{Body, Expr, GlobalBody, Lambda, Literal, Module};

/// Lowers a resolved, type-checked unit.
pub fn lower_unit(unit: &hir::Unit) -> Module {
    Module {
        globals: unit
            .defs
            .iter()
            .map(|def| GlobalBody {
                id: def.id,
                name: def.name.clone(),
                body: lower_body(&def.body),
            })
            .collect(),
        entries: unit.entries.iter().map(lower_body).collect(),
    }
}

pub fn lower_body(body: &hir::Body) -> Body {
    let mut lowerer = Lowerer {
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
        let slot = self.size;
        self.size += 1;
        self.slots.insert(local, slot);
        slot
    }
}

struct Lowerer {
    frames: Vec<Frame>,
}

impl Lowerer {
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
            ExprKind::Number(n) => Expr::Lit(Literal::Number(*n)),
            ExprKind::String(s) => Expr::Lit(Literal::String(s.clone())),
            ExprKind::Unit => Expr::Lit(Literal::Unit),
            ExprKind::Local(local) => Expr::Local(self.slot(self.frames.len() - 1, *local)),
            ExprKind::Global(id) => Expr::Global(*id),
            ExprKind::Builtin(builtin) => Expr::Builtin(*builtin),
            ExprKind::Lambda { .. } => Expr::Lambda(Rc::new(self.lambda(expr))),
            ExprKind::App { func, args } => Expr::App {
                func: self.expr(func),
                args: args.iter().map(|arg| self.expr(arg)).collect(),
                span: expr.span,
            },
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
        lower_unit(&resolved.unit)
    }

    fn global(text: &str) -> String {
        lower_text(text).globals[0].body.expr.to_string()
    }

    #[test]
    fn nested_lambdas_are_uncurried() {
        assert_eq!(global(r"k = \x. \y. x"), "(lambda/2 [] frame=2 $0)");
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
        let module = lower_text("a = 1\n(print a)\nb = 2\n(print b)");
        assert_eq!(module.globals.len(), 2);
        assert_eq!(module.entries.len(), 2);
        assert_eq!(
            module.to_string(),
            "@0 a [frame=0] = 1\n@1 b [frame=0] = 2\nentry [frame=0] = (print @0)\nentry [frame=0] = (print @1)\n"
        );
    }
}
