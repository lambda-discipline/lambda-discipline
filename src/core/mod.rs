//! Core IR: the small, explicit language every backend consumes.
//!
//! Lowering from [`crate::hir`] (see [`lower`]) performs the transformations
//! a compiler back end needs, so backends do not repeat them:
//!
//! * **Closure conversion.** Every lambda lists the enclosing slots it
//!   captures; a lambda's frame is `captures ++ parameters ++ let slots`, and
//!   variables are frame slot numbers instead of names.
//! * **Uncurrying.** `\x. \y. body` becomes one lambda of arity 2; partial
//!   and over-application are handled by the backend's apply.
//! * Names are gone; globals and builtins are referenced by id.
//! * `__FILE__`, `__DIR__` and `__LINE__` become the literals they stand for,
//!   since they are fixed by where they are written.
//! * **Saturated constructors.** A constructor applied to all its fields is a
//!   [`Expr::Con`] node; anywhere else it becomes a lambda that builds one.
//! * **Match compilation.** Nested patterns become a tree of flat
//!   [`Expr::Case`] nodes, each of which inspects one value and binds the
//!   fields of the constructor it finds to frame slots.
//!
//! The interpreter in [`crate::backend::interp`] executes this IR lazily. A
//! code generator (C, uxn, LLVM) would start from the same IR.

pub mod lower;

use std::fmt;
use std::rc::Rc;

use crate::builtins::Builtin;
use crate::hir::{ConId, GlobalId};
use crate::index::to_u32;
use crate::source::Span;
use crate::syntax::number::format_float;
use crate::types::env::builtin_constructors;

pub use lower::lower_unit;

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(Rc<str>),
    Unit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Lit(Literal),
    /// A slot of the current frame.
    Local(u32),
    Global(GlobalId),
    Builtin(Builtin),
    Lambda(Rc<Lambda>),
    App {
        func: Rc<Expr>,
        args: Vec<Rc<Expr>>,
        span: Span,
    },
    List {
        items: Vec<Rc<Expr>>,
        span: Span,
    },
    /// Binds `slot` (visible in `value`, so recursive) and continues with `body`.
    Let {
        slot: u32,
        value: Rc<Expr>,
        body: Rc<Expr>,
    },
    /// A constructor applied to exactly as many arguments as it has fields.
    Con {
        con: Rc<Constructor>,
        args: Vec<Rc<Expr>>,
        span: Span,
    },
    Case(Rc<Case>),
}

/// A data constructor, as a backend needs it.
#[derive(Debug, PartialEq, Eq)]
pub struct Constructor {
    /// Unique in a session. The constructors of a type have consecutive ids
    /// in declaration order.
    pub id: ConId,
    pub name: Rc<str>,
    pub arity: u32,
}

/// `True` or `False`, which builtins produce.
pub fn bool_constructor(value: bool) -> Rc<Constructor> {
    builtin_constructor(if value { ConId::TRUE } else { ConId::FALSE })
}

/// A constructor of a built-in type (`True`, `Some`, `Err`, ...), shared so
/// builtins can produce its values cheaply.
pub fn builtin_constructor(id: ConId) -> Rc<Constructor> {
    thread_local! {
        static CONSTRUCTORS: Vec<Rc<Constructor>> = builtin_constructors()
            .map(|(id, con)| {
                Rc::new(Constructor {
                    id,
                    name: Rc::from(con.name),
                    arity: to_u32(con.fields.len()),
                })
            })
            .collect();
    }
    CONSTRUCTORS.with(|constructors| constructors[id.index()].clone())
}

/// Evaluates `scrutinee` to its outermost constructor or literal and
/// continues with the first alternative that matches it, or `default`.
/// Type checking guarantees that one of them applies.
#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    pub scrutinee: Rc<Expr>,
    pub alts: Vec<Alt>,
    pub default: Option<Rc<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Alt {
    pub pat: AltPat,
    pub body: Rc<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AltPat {
    /// A constructor; its fields are stored in these slots, unevaluated.
    Con {
        con: Rc<Constructor>,
        fields: Vec<u32>,
    },
    Lit(Literal),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Lambda {
    /// Number of parameters, after uncurrying.
    pub arity: u32,
    /// Slots of the *enclosing* frame copied into this lambda's frame, in order.
    pub captures: Vec<u32>,
    /// Total slots in this lambda's frame.
    pub frame_size: u32,
    pub body: Rc<Expr>,
    pub span: Span,
}

/// Code run in a fresh frame with no parameters: a global's definition or a
/// top-level expression.
#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub frame_size: u32,
    pub expr: Rc<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalBody {
    pub id: GlobalId,
    pub name: Rc<str>,
    pub body: Body,
}

/// The lowered form of one compilation unit.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    pub globals: Vec<GlobalBody>,
    /// Top-level expressions, in source order. A backend performs the ones
    /// that are `IO` actions, in this order; in a program they all are.
    pub entries: Vec<Body>,
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Lit(Literal::Int(n)) => write!(f, "{n}"),
            Expr::Lit(Literal::Float(n)) => f.write_str(&format_float(*n)),
            Expr::Lit(Literal::String(s)) => write!(f, "{s:?}"),
            Expr::Lit(Literal::Unit) => f.write_str("()"),
            Expr::Local(slot) => write!(f, "${slot}"),
            Expr::Global(id) => write!(f, "@{}", id.0),
            Expr::Builtin(b) => write!(f, "{b}"),
            Expr::Lambda(lambda) => {
                write!(f, "(lambda/{} [", lambda.arity)?;
                for (i, capture) in lambda.captures.iter().enumerate() {
                    if i > 0 {
                        f.write_str(" ")?;
                    }
                    write!(f, "${capture}")?;
                }
                write!(f, "] frame={} {})", lambda.frame_size, lambda.body)
            }
            Expr::App { func, args, .. } => {
                write!(f, "({func}")?;
                for arg in args {
                    write!(f, " {arg}")?;
                }
                f.write_str(")")
            }
            Expr::List { items, .. } => {
                f.write_str("[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_str("]")
            }
            Expr::Let { slot, value, body } => write!(f, "(let ${slot} = {value} in {body})"),
            Expr::Con { con, args, .. } if args.is_empty() => f.write_str(&con.name),
            Expr::Con { con, args, .. } => {
                write!(f, "({}", con.name)?;
                for arg in args {
                    write!(f, " {arg}")?;
                }
                f.write_str(")")
            }
            Expr::Case(case) => {
                write!(f, "(case {}", case.scrutinee)?;
                for alt in &case.alts {
                    f.write_str(" | ")?;
                    match &alt.pat {
                        AltPat::Con { con, fields } => {
                            f.write_str(&con.name)?;
                            for field in fields {
                                write!(f, " ${field}")?;
                            }
                        }
                        AltPat::Lit(literal) => write!(f, "{}", Expr::Lit(literal.clone()))?,
                    }
                    write!(f, " -> {}", alt.body)?;
                }
                if let Some(default) = &case.default {
                    write!(f, " | _ -> {default}")?;
                }
                f.write_str(")")
            }
        }
    }
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for global in &self.globals {
            writeln!(
                f,
                "@{} {} [frame={}] = {}",
                global.id.0, global.name, global.body.frame_size, global.body.expr
            )?;
        }
        for entry in &self.entries {
            writeln!(f, "entry [frame={}] = {}", entry.frame_size, entry.expr)?;
        }
        Ok(())
    }
}
