//! High-level IR: the AST after name resolution.
//!
//! Every variable is resolved to a local binder, a global definition or a
//! builtin, so later stages never look names up again.

use std::rc::Rc;

use crate::builtins::Builtin;
use crate::source::Span;
use crate::syntax::ast::TypeExpr;

/// Identifies a global definition for the lifetime of a session. In the
/// REPL, redefining a name allocates a new id; old references keep theirs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GlobalId(pub u32);

impl GlobalId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A local binder (lambda parameter or `let`), unique within one [`Body`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalId(pub u32);

impl LocalId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Number(f64),
    String(Rc<str>),
    Unit,
    Local(LocalId),
    Global(GlobalId),
    Builtin(Builtin),
    Lambda {
        param: LocalId,
        ann: Option<TypeExpr>,
        body: Box<Expr>,
    },
    App {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    List(Vec<Expr>),
    /// A recursive `let`: `value` may refer to `local`.
    Let {
        local: LocalId,
        ann: Option<TypeExpr>,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    /// Stands in for an expression that failed to resolve or parse.
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalInfo {
    pub name: Rc<str>,
    pub span: Span,
}

/// An expression together with the table of its local binders.
#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub expr: Expr,
    pub locals: Vec<LocalInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalDef {
    pub id: GlobalId,
    pub name: Rc<str>,
    pub name_span: Span,
    pub ann: Option<TypeExpr>,
    pub body: Body,
}

/// One compilation unit: a file with its `use`s, or one REPL input.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Unit {
    pub defs: Vec<GlobalDef>,
    /// Top-level expressions, in source order.
    pub entries: Vec<Body>,
}

impl Expr {
    /// Calls `f` on every global this expression refers to.
    pub fn for_each_global(&self, f: &mut impl FnMut(GlobalId)) {
        match &self.kind {
            ExprKind::Global(id) => f(*id),
            ExprKind::Lambda { body, .. } => body.for_each_global(f),
            ExprKind::App { func, args } => {
                func.for_each_global(f);
                for arg in args {
                    arg.for_each_global(f);
                }
            }
            ExprKind::List(items) => {
                for item in items {
                    item.for_each_global(f);
                }
            }
            ExprKind::Let { value, body, .. } => {
                value.for_each_global(f);
                body.for_each_global(f);
            }
            ExprKind::Number(_)
            | ExprKind::String(_)
            | ExprKind::Unit
            | ExprKind::Local(_)
            | ExprKind::Builtin(_)
            | ExprKind::Error => {}
        }
    }
}
