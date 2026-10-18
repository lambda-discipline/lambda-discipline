//! High-level IR: the AST after name resolution.
//!
//! Every variable is resolved to a local binder, a global definition, a
//! constructor or a builtin, so later stages never look names up again.

use std::rc::Rc;

use crate::builtins::Builtin;
use crate::source::Span;
use crate::syntax::ast::{Ident, TypeExpr, TypeScheme};

/// Identifies a global definition for the lifetime of a session. In the
/// REPL, redefining a name allocates a new id; old references keep theirs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GlobalId(pub u32);

impl GlobalId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifies a type constructor (`Int`, `List`, a declared type) for the
/// lifetime of a session. Redefining a type in the REPL allocates a new id,
/// so values of the old type keep the old type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TyConId(pub u32);

impl TyConId {
    pub const INT: TyConId = TyConId(0);
    pub const FLOAT: TyConId = TyConId(1);
    pub const STRING: TyConId = TyConId(2);
    pub const BOOL: TyConId = TyConId(3);
    pub const UNIT: TyConId = TyConId(4);
    pub const LIST: TyConId = TyConId(5);
    pub const IO: TyConId = TyConId(6);
    pub const OPTION: TyConId = TyConId(7);
    pub const RESULT: TyConId = TyConId(8);

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifies a data constructor for the lifetime of a session. The
/// constructors of one type have consecutive ids in declaration order, so
/// comparing ids orders values by constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConId(pub u32);

impl ConId {
    pub const FALSE: ConId = ConId(0);
    pub const TRUE: ConId = ConId(1);
    pub const NONE: ConId = ConId(2);
    pub const SOME: ConId = ConId(3);
    pub const OK: ConId = ConId(4);
    pub const ERR: ConId = ConId(5);

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A local binder (lambda parameter, `let` or pattern variable), unique
/// within one [`Body`].
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
    Int(i64),
    Float(f64),
    String(Rc<str>),
    Unit,
    Local(LocalId),
    Global(GlobalId),
    Builtin(Builtin),
    /// A data constructor, used as a function (or a value if it has no fields).
    Con(ConId),
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
        ann: Option<TypeScheme>,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<Arm>,
    },
    /// Stands in for an expression that failed to resolve or parse.
    Error,
}

/// `| pattern -> body`; the pattern's variables are in scope in `body`.
#[derive(Debug, Clone, PartialEq)]
pub struct Arm {
    pub pattern: Pattern,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternKind {
    Wild,
    Bind(LocalId),
    Con {
        con: ConId,
        args: Vec<Pattern>,
    },
    Int(i64),
    Float(f64),
    String(Rc<str>),
    Unit,
    /// Stands in for a pattern that failed to resolve.
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
    pub ann: Option<TypeScheme>,
    pub body: Body,
}

/// A data type declared by a unit, with ids for it and its constructors.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDef {
    pub id: TyConId,
    pub name: Ident,
    pub params: Vec<Ident>,
    pub constructors: Vec<CtorDef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CtorDef {
    pub id: ConId,
    pub name: Ident,
    /// Field types as written; the type checker converts them.
    pub fields: Vec<TypeExpr>,
}

/// One compilation unit: a file with its `use`s, or one REPL input.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Unit {
    /// Declared types, in source order, with consecutive ids.
    pub types: Vec<TypeDef>,
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
            ExprKind::Match { scrutinee, arms } => {
                scrutinee.for_each_global(f);
                for arm in arms {
                    arm.body.for_each_global(f);
                }
            }
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Unit
            | ExprKind::Local(_)
            | ExprKind::Builtin(_)
            | ExprKind::Con(_)
            | ExprKind::Error => {}
        }
    }
}
