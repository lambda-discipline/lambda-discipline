//! Surface syntax tree, as written by the user.
//!
//! Names are still plain strings here; [`crate::resolve`] turns them into
//! resolved references.

use std::fmt;
use std::rc::Rc;

use crate::source::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    pub name: Rc<str>,
    pub span: Span,
}

/// One parsed file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// `use "path"`
    Use { path: String, span: Span },
    /// `name : Type = expr` (the annotation is optional)
    Def(Def),
    /// A top-level expression. In a program it is an action, and running the
    /// program performs these in order.
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Def {
    pub name: Ident,
    pub ann: Option<TypeExpr>,
    pub value: Expr,
    pub span: Span,
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
    /// `()`
    Unit,
    Var(Ident),
    /// `\x: T. body`
    Lambda {
        param: Ident,
        ann: Option<TypeExpr>,
        body: Box<Expr>,
    },
    /// `(f a b ...)` with at least one argument.
    App {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    /// `[a, b, ...]`
    List(Vec<Expr>),
    /// `let x: T = value in body`
    Let {
        name: Ident,
        ann: Option<TypeExpr>,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    /// `let x: T <- action in body`: performs `action` and binds its result
    /// to `x` in `body`, which is an action too.
    Bind {
        name: Ident,
        ann: Option<TypeExpr>,
        action: Box<Expr>,
        body: Box<Expr>,
    },
    /// Placeholder produced after a syntax error so parsing can continue.
    Error,
}

/// A type as written in an annotation.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeExpr {
    pub kind: TypeExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExprKind {
    /// `Number`, `List<a>`: a capitalized constructor with arguments.
    Con { name: Ident, args: Vec<TypeExpr> },
    /// `a`: a lowercase type variable.
    Var(Ident),
    /// `a -> b`
    Fun(Box<TypeExpr>, Box<TypeExpr>),
    /// `()`
    Unit,
}

impl fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TypeExprKind::Con { name, args } if args.is_empty() => write!(f, "{}", name.name),
            TypeExprKind::Con { name, args } => {
                write!(f, "{}<", name.name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{arg}")?;
                }
                f.write_str(">")
            }
            TypeExprKind::Var(name) => write!(f, "{}", name.name),
            TypeExprKind::Fun(from, to) => match from.kind {
                TypeExprKind::Fun(..) => write!(f, "({from}) -> {to}"),
                _ => write!(f, "{from} -> {to}"),
            },
            TypeExprKind::Unit => f.write_str("()"),
        }
    }
}

/// Pretty-prints the tree in an S-expression form (used by `--emit ast`).
impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ExprKind::Number(n) => write!(f, "{n}"),
            ExprKind::String(s) => write!(f, "{s:?}"),
            ExprKind::Unit => f.write_str("()"),
            ExprKind::Var(id) => f.write_str(&id.name),
            ExprKind::Lambda { param, ann, body } => match ann {
                Some(t) => write!(f, "(\\{}: {}. {})", param.name, t, body),
                None => write!(f, "(\\{}. {})", param.name, body),
            },
            ExprKind::App { func, args } => {
                write!(f, "({func}")?;
                for arg in args {
                    write!(f, " {arg}")?;
                }
                f.write_str(")")
            }
            ExprKind::List(items) => {
                f.write_str("[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_str("]")
            }
            ExprKind::Let {
                name,
                ann,
                value,
                body,
            } => match ann {
                Some(t) => write!(f, "(let {}: {} = {} in {})", name.name, t, value, body),
                None => write!(f, "(let {} = {} in {})", name.name, value, body),
            },
            ExprKind::Bind {
                name,
                ann,
                action,
                body,
            } => match ann {
                Some(t) => write!(f, "(let {}: {} <- {} in {})", name.name, t, action, body),
                None => write!(f, "(let {} <- {} in {})", name.name, action, body),
            },
            ExprKind::Error => f.write_str("<error>"),
        }
    }
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for item in &self.items {
            match item {
                Item::Use { path, .. } => writeln!(f, "use {path:?}")?,
                Item::Def(def) => match &def.ann {
                    Some(t) => writeln!(f, "{} : {} = {}", def.name.name, t, def.value)?,
                    None => writeln!(f, "{} = {}", def.name.name, def.value)?,
                },
                Item::Expr(expr) => writeln!(f, "{expr}")?,
            }
        }
        Ok(())
    }
}
