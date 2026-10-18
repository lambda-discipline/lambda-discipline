//! Surface syntax tree, as written by the user.
//!
//! Names are still plain strings here; [`crate::resolve`] turns them into
//! resolved references.

use std::fmt;
use std::rc::Rc;

use crate::source::Span;
use crate::syntax::number::format_number;

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
    /// `type Name<a> = Con field ... | ...`
    Type(TypeDecl),
    /// A top-level expression. In a program it is an action, and running the
    /// program performs these in order.
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Def {
    pub name: Ident,
    pub ann: Option<TypeScheme>,
    pub value: Expr,
    pub span: Span,
}

/// A data type declaration: `type Option<a> = None | Some a`.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDecl {
    pub name: Ident,
    pub params: Vec<Ident>,
    pub constructors: Vec<CtorDecl>,
    pub span: Span,
}

/// One constructor of a [`TypeDecl`] with the types of its fields.
#[derive(Debug, Clone, PartialEq)]
pub struct CtorDecl {
    pub name: Ident,
    pub fields: Vec<TypeExpr>,
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
        ann: Option<TypeScheme>,
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
    /// `match scrutinee | pattern -> body | ...`
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<Arm>,
    },
    /// Placeholder produced after a syntax error so parsing can continue.
    Error,
}

/// `| pattern -> body`
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
    /// `_`
    Wild,
    /// A lowercase name, bound to the matched value.
    Var(Ident),
    /// `Some x`: a constructor and patterns for its fields.
    Con {
        name: Ident,
        args: Vec<Pattern>,
    },
    Number(f64),
    String(Rc<str>),
    /// `()`
    Unit,
}

/// The annotation of a definition or `let`: a type with an optional
/// context, `(Eq<a>, Show<b>) => a -> b -> String`.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeScheme {
    pub context: Vec<Constraint>,
    pub ty: TypeExpr,
}

/// `Eq<a>`: the type argument must belong to the class. Only a type
/// variable is valid as the argument, which the type checker enforces.
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    pub class: Ident,
    pub arg: TypeExpr,
    pub span: Span,
}

impl fmt::Display for TypeScheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let context: Vec<String> = self
            .context
            .iter()
            .map(|c| format!("{}<{}>", c.class.name, c.arg))
            .collect();
        match context.len() {
            0 => write!(f, "{}", self.ty),
            1 => write!(f, "{} => {}", context[0], self.ty),
            _ => write!(f, "({}) => {}", context.join(", "), self.ty),
        }
    }
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
            ExprKind::Number(n) => f.write_str(&format_number(*n)),
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
            ExprKind::Match { scrutinee, arms } => {
                write!(f, "(match {scrutinee}")?;
                for arm in arms {
                    write!(f, " | ")?;
                    arm.pattern.write(f, false)?;
                    write!(f, " -> {}", arm.body)?;
                }
                f.write_str(")")
            }
            ExprKind::Error => f.write_str("<error>"),
        }
    }
}

impl Pattern {
    /// Writes the pattern; `nested` parenthesizes a constructor with fields.
    fn write(&self, f: &mut fmt::Formatter<'_>, nested: bool) -> fmt::Result {
        match &self.kind {
            PatternKind::Wild => f.write_str("_"),
            PatternKind::Var(name) => f.write_str(&name.name),
            PatternKind::Con { name, args } if args.is_empty() => f.write_str(&name.name),
            PatternKind::Con { name, args } => {
                if nested {
                    f.write_str("(")?;
                }
                f.write_str(&name.name)?;
                for arg in args {
                    f.write_str(" ")?;
                    arg.write(f, true)?;
                }
                if nested {
                    f.write_str(")")?;
                }
                Ok(())
            }
            PatternKind::Number(n) => f.write_str(&format_number(*n)),
            PatternKind::String(s) => write!(f, "{s:?}"),
            PatternKind::Unit => f.write_str("()"),
        }
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write(f, false)
    }
}

impl fmt::Display for TypeDecl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "type {}", self.name.name)?;
        if !self.params.is_empty() {
            let params: Vec<&str> = self.params.iter().map(|p| p.name.as_ref()).collect();
            write!(f, "<{}>", params.join(", "))?;
        }
        f.write_str(" =")?;
        for (i, ctor) in self.constructors.iter().enumerate() {
            if i > 0 {
                f.write_str(" |")?;
            }
            write!(f, " {}", ctor.name.name)?;
            for field in &ctor.fields {
                match field.kind {
                    TypeExprKind::Fun(..) => write!(f, " ({field})")?,
                    _ => write!(f, " {field}")?,
                }
            }
        }
        Ok(())
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
                Item::Type(decl) => writeln!(f, "{decl}")?,
                Item::Expr(expr) => writeln!(f, "{expr}")?,
            }
        }
        Ok(())
    }
}
