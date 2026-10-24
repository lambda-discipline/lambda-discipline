//! The generator's own small model of Lambda Discipline programs, and how it
//! prints as source text.
//!
//! The model is independent of the compiler's AST on purpose: it only has to
//! agree with the language, not with the compiler's internals, so it checks
//! the parser instead of sharing its assumptions.

use std::fmt::{self, Write};

/// A type of the generated program. Every type is monomorphic, except that
/// the fields of a [`DataDecl`] may mention its parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Int,
    Float,
    String,
    Bool,
    Unit,
    List(Box<Ty>),
    Option(Box<Ty>),
    /// `Result<e, a>`.
    Result(Box<Ty>, Box<Ty>),
    Tuple(Vec<Ty>),
    Fun(Box<Ty>, Box<Ty>),
    Io(Box<Ty>),
    /// The program's data type with this index, applied to arguments.
    Data(usize, Vec<Ty>),
    /// A parameter of the enclosing data declaration.
    Param(usize),
}

impl Ty {
    pub fn list(item: Ty) -> Ty {
        Ty::List(Box::new(item))
    }

    pub fn option(item: Ty) -> Ty {
        Ty::Option(Box::new(item))
    }

    pub fn result(error: Ty, ok: Ty) -> Ty {
        Ty::Result(Box::new(error), Box::new(ok))
    }

    pub fn fun(from: Ty, to: Ty) -> Ty {
        Ty::Fun(Box::new(from), Box::new(to))
    }

    pub fn io(result: Ty) -> Ty {
        Ty::Io(Box::new(result))
    }

    /// Replaces [`Ty::Param`]s by `args`.
    pub fn subst(&self, args: &[Ty]) -> Ty {
        match self {
            Ty::Param(i) => args[*i].clone(),
            Ty::List(t) => Ty::list(t.subst(args)),
            Ty::Option(t) => Ty::option(t.subst(args)),
            Ty::Result(e, a) => Ty::result(e.subst(args), a.subst(args)),
            Ty::Tuple(items) => Ty::Tuple(items.iter().map(|t| t.subst(args)).collect()),
            Ty::Fun(a, b) => Ty::fun(a.subst(args), b.subst(args)),
            Ty::Io(t) => Ty::io(t.subst(args)),
            Ty::Data(d, xs) => Ty::Data(*d, xs.iter().map(|t| t.subst(args)).collect()),
            other => other.clone(),
        }
    }

    /// Whether the outermost type constructors of `self` and `other` differ,
    /// so that no instantiation can make them equal.
    pub fn clashes_with(&self, other: &Ty) -> bool {
        std::mem::discriminant(self) != std::mem::discriminant(other)
            || match (self, other) {
                (Ty::Tuple(a), Ty::Tuple(b)) => a.len() != b.len(),
                (Ty::Data(a, _), Ty::Data(b, _)) => a != b,
                _ => false,
            }
    }
}

/// `type T<a, b> = C1 ... | C2 ...`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataDecl {
    pub name: String,
    pub params: usize,
    /// Each constructor's name and field types. The first constructor's
    /// fields never mention the type itself, so every type has a finite value.
    pub ctors: Vec<(String, Vec<Ty>)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pat {
    Wild,
    Var(String),
    /// A constructor (`None`, `Some p`, a data constructor) with its fields.
    Con(String, Vec<Pat>),
    Int(i64),
    Float(f64),
    String(String),
    Unit,
    Tuple(Vec<Pat>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Var(String),
    /// A constructor or builtin, which renaming leaves alone.
    Global(String),
    Int(i64),
    Float(f64),
    String(String),
    Unit,
    App(Box<Expr>, Vec<Expr>),
    Lambda {
        param: Pat,
        ann: Option<Ty>,
        body: Box<Expr>,
    },
    Let {
        pat: Pat,
        ann: Option<Ty>,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    /// `let p <- action in body`.
    Bind {
        pat: Pat,
        ann: Option<Ty>,
        action: Box<Expr>,
        body: Box<Expr>,
    },
    List(Vec<Expr>),
    Tuple(Vec<Expr>),
    Match(Box<Expr>, Vec<(Pat, Expr)>),
}

impl Expr {
    pub fn app(func: &str, args: Vec<Expr>) -> Expr {
        Expr::App(Box::new(Expr::Global(func.to_owned())), args)
    }

    /// Whether the expression extends as far to the right as it can, and so
    /// needs parentheses anywhere but at the end of an enclosing expression.
    fn is_open(&self) -> bool {
        matches!(
            self,
            Expr::Lambda { .. } | Expr::Let { .. } | Expr::Bind { .. } | Expr::Match(..)
        )
    }

    /// Calls `f` on every variable name (bound or used) in the expression.
    fn rename(&mut self, f: &impl Fn(&str) -> String) {
        match self {
            Expr::Var(name) => *name = f(name),
            Expr::App(func, args) => {
                func.rename(f);
                for a in args.iter_mut() {
                    a.rename(f);
                }
            }
            Expr::Lambda { param, body, .. } => {
                param.rename(f);
                body.rename(f);
            }
            Expr::Let {
                pat, value, body, ..
            } => {
                pat.rename(f);
                value.rename(f);
                body.rename(f);
            }
            Expr::Bind {
                pat, action, body, ..
            } => {
                pat.rename(f);
                action.rename(f);
                body.rename(f);
            }
            Expr::List(items) | Expr::Tuple(items) => items.iter_mut().for_each(|e| e.rename(f)),
            Expr::Match(scrutinee, arms) => {
                scrutinee.rename(f);
                for (pat, body) in arms {
                    pat.rename(f);
                    body.rename(f);
                }
            }
            Expr::Global(_) | Expr::Int(_) | Expr::Float(_) | Expr::String(_) | Expr::Unit => {}
        }
    }
}

impl Pat {
    fn rename(&mut self, f: &impl Fn(&str) -> String) {
        match self {
            Pat::Var(name) => *name = f(name),
            Pat::Con(_, items) | Pat::Tuple(items) => items.iter_mut().for_each(|p| p.rename(f)),
            _ => {}
        }
    }
}

/// A top-level definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Def {
    pub name: String,
    /// The type the generator meant the body to have.
    pub ty: Ty,
    /// The annotation written in the source, if any (as text, so that a
    /// test can write an inferred scheme back).
    pub ann: Option<String>,
    pub body: Expr,
}

/// A generated program.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub data: Vec<DataDecl>,
    pub defs: Vec<Def>,
    /// Top-level expressions, with the types the generator meant.
    pub entries: Vec<(Expr, Ty)>,
}

impl Program {
    /// Annotates every definition with the type the generator meant.
    pub fn annotate(&mut self) {
        for i in 0..self.defs.len() {
            let ann = self.ty(&self.defs[i].ty);
            self.defs[i].ann = Some(ann);
        }
    }

    /// Removes every top-level annotation.
    pub fn strip_annotations(&mut self) {
        for def in &mut self.defs {
            def.ann = None;
        }
    }

    /// Renames every generated variable and definition consistently.
    pub fn rename(&mut self, f: impl Fn(&str) -> String) {
        for def in &mut self.defs {
            def.name = f(&def.name);
            def.body.rename(&f);
        }
        for (entry, _) in &mut self.entries {
            entry.rename(&f);
        }
    }

    /// Prints a type in source syntax.
    pub fn ty(&self, ty: &Ty) -> String {
        let mut out = String::new();
        self.write_ty(&mut out, ty, false, &[]);
        out
    }

    /// `params` names the parameters of the enclosing data declaration;
    /// `atomic` asks for parentheses around a function type.
    fn write_ty(&self, out: &mut String, ty: &Ty, atomic: bool, params: &[String]) {
        let list = |out: &mut String, name: &str, items: &[&Ty]| {
            out.push_str(name);
            out.push('<');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                self.write_ty(out, item, false, params);
            }
            out.push('>');
        };
        match ty {
            Ty::Int => out.push_str("Int"),
            Ty::Float => out.push_str("Float"),
            Ty::String => out.push_str("String"),
            Ty::Bool => out.push_str("Bool"),
            Ty::Unit => out.push_str("Unit"),
            Ty::List(t) => list(out, "List", &[t]),
            Ty::Option(t) => list(out, "Option", &[t]),
            Ty::Result(e, a) => list(out, "Result", &[e, a]),
            Ty::Io(t) => list(out, "IO", &[t]),
            Ty::Data(d, args) if args.is_empty() => out.push_str(&self.data[*d].name),
            Ty::Data(d, args) => list(out, &self.data[*d].name, &args.iter().collect::<Vec<_>>()),
            Ty::Param(i) => out.push_str(&params[*i]),
            Ty::Tuple(items) => {
                out.push('(');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    self.write_ty(out, item, false, params);
                }
                out.push(')');
            }
            Ty::Fun(a, b) => {
                if atomic {
                    out.push('(');
                }
                self.write_ty(out, a, true, params);
                out.push_str(" -> ");
                self.write_ty(out, b, false, params);
                if atomic {
                    out.push(')');
                }
            }
        }
    }

    fn write_data(&self, out: &mut String, decl: &DataDecl) {
        let params: Vec<String> = (0..decl.params).map(param_name).collect();
        out.push_str("type ");
        out.push_str(&decl.name);
        if !params.is_empty() {
            let _ = write!(out, "<{}>", params.join(", "));
        }
        out.push_str(" =");
        for (i, (name, fields)) in decl.ctors.iter().enumerate() {
            out.push_str(if i == 0 { " " } else { " | " });
            out.push_str(name);
            for field in fields {
                out.push(' ');
                self.write_ty(out, field, true, &params);
            }
        }
        out.push('\n');
    }
}

/// The name of a data declaration's `i`th parameter.
fn param_name(i: usize) -> String {
    char::from(b"abcdefghijklmnopqrstuvwxyz"[i]).to_string()
}

/// Prints the program as source text.
impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        for decl in &self.data {
            self.write_data(&mut out, decl);
        }
        for def in &self.defs {
            out.push_str(&def.name);
            if let Some(ann) = &def.ann {
                out.push_str(" : ");
                out.push_str(ann);
            }
            out.push_str(" = ");
            self.write_expr(&mut out, &def.body, 1);
            out.push('\n');
        }
        for (entry, _) in &self.entries {
            self.write_expr(&mut out, entry, 0);
            out.push('\n');
        }
        f.write_str(&out)
    }
}

impl Program {
    /// Writes `e`, parenthesized if it is open and something follows it.
    fn write_closed(&self, out: &mut String, e: &Expr, indent: usize) {
        if e.is_open() {
            out.push('(');
            self.write_expr(out, e, indent);
            out.push(')');
        } else {
            self.write_expr(out, e, indent);
        }
    }

    fn write_expr(&self, out: &mut String, e: &Expr, indent: usize) {
        match e {
            Expr::Var(name) | Expr::Global(name) => out.push_str(name),
            Expr::Int(n) => {
                let _ = write!(out, "{n}");
            }
            Expr::Float(x) => out.push_str(&float_literal(*x)),
            Expr::String(s) => write_string(out, s),
            Expr::Unit => out.push_str("()"),
            Expr::App(func, args) => {
                out.push('(');
                self.write_closed(out, func, indent);
                for arg in args {
                    out.push(' ');
                    self.write_closed(out, arg, indent);
                }
                out.push(')');
            }
            Expr::Lambda { param, ann, body } => {
                out.push('\\');
                write_pat(out, param, true);
                if let Some(t) = ann {
                    out.push_str(": ");
                    out.push_str(&self.ty(t));
                }
                out.push_str(". ");
                self.write_expr(out, body, indent);
            }
            Expr::Let {
                pat,
                ann,
                value,
                body,
            }
            | Expr::Bind {
                pat,
                ann,
                action: value,
                body,
            } => {
                out.push_str("let ");
                write_pat(out, pat, true);
                if let Some(t) = ann {
                    out.push_str(": ");
                    out.push_str(&self.ty(t));
                }
                out.push_str(if matches!(e, Expr::Let { .. }) {
                    " = "
                } else {
                    " <- "
                });
                self.write_closed(out, value, indent);
                out.push_str(" in\n");
                push_indent(out, indent);
                self.write_expr(out, body, indent);
            }
            Expr::List(items) | Expr::Tuple(items) => {
                let (open, close) = if matches!(e, Expr::List(_)) {
                    ('[', ']')
                } else {
                    ('(', ')')
                };
                out.push(open);
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    self.write_closed(out, item, indent);
                }
                out.push(close);
            }
            Expr::Match(scrutinee, arms) => {
                out.push_str("match ");
                self.write_closed(out, scrutinee, indent);
                for (pat, body) in arms {
                    out.push('\n');
                    push_indent(out, indent + 1);
                    out.push_str("| ");
                    write_pat(out, pat, false);
                    out.push_str(" -> ");
                    self.write_closed(out, body, indent + 2);
                }
            }
        }
    }
}

fn push_indent(out: &mut String, indent: usize) {
    // Never column 0, which would start a new top-level item.
    for _ in 0..indent.max(1) {
        out.push_str("    ");
    }
}

/// `atomic` asks for parentheses around a constructor with fields.
fn write_pat(out: &mut String, pat: &Pat, atomic: bool) {
    match pat {
        Pat::Wild => out.push('_'),
        Pat::Var(name) => out.push_str(name),
        Pat::Con(name, fields) if fields.is_empty() => out.push_str(name),
        Pat::Con(name, fields) => {
            if atomic {
                out.push('(');
            }
            out.push_str(name);
            for field in fields {
                out.push(' ');
                write_pat(out, field, true);
            }
            if atomic {
                out.push(')');
            }
        }
        Pat::Int(n) => {
            let _ = write!(out, "{n}");
        }
        Pat::Float(x) => out.push_str(&float_literal(*x)),
        Pat::String(s) => write_string(out, s),
        Pat::Unit => out.push_str("()"),
        Pat::Tuple(items) => {
            out.push('(');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_pat(out, item, false);
            }
            out.push(')');
        }
    }
}

/// A float literal that reads back as exactly `x`. Rust's `Debug` writes the
/// shortest round-trip digits, always with a `.` or an exponent.
pub fn float_literal(x: f64) -> String {
    assert!(x.is_finite(), "no literal for {x}");
    format!("{x:?}")
}

/// Writes a string literal, escaping what the lexer requires.
fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{{{:x}}}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}
