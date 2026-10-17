//! Type representation shared by the checker and later stages.

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

/// A unification variable, owned by an inference context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TyVar(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// A unification variable (only during inference).
    Var(TyVar),
    /// A quantified variable of a [`Scheme`], replaced on instantiation.
    Gen(u32),
    /// A rigid (skolem) variable from a user annotation, such as the `a` in
    /// `id : a -> a`; it only unifies with itself.
    Rigid(Rc<str>),
    /// A type constructor applied to arguments: `Number`, `List<a>`.
    Con(Rc<str>, Vec<Type>),
    /// A function type.
    Fun(Box<Type>, Box<Type>),
}

/// Built-in type constructors and their arities.
pub const CONSTRUCTORS: &[(&str, usize)] = &[
    ("Number", 0),
    ("String", 0),
    ("Bool", 0),
    ("Unit", 0),
    ("List", 1),
    ("IO", 1),
];

pub fn constructor_arity(name: &str) -> Option<usize> {
    CONSTRUCTORS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|&(_, arity)| arity)
}

impl Type {
    pub fn con(name: &str) -> Type {
        Type::Con(Rc::from(name), Vec::new())
    }

    pub fn number() -> Type {
        Type::con("Number")
    }

    pub fn string() -> Type {
        Type::con("String")
    }

    pub fn bool() -> Type {
        Type::con("Bool")
    }

    pub fn unit() -> Type {
        Type::con("Unit")
    }

    pub fn list(element: Type) -> Type {
        Type::Con(Rc::from("List"), vec![element])
    }

    /// `IO<result>`: an action producing a `result` when it runs.
    pub fn io(result: Type) -> Type {
        Type::Con(Rc::from("IO"), vec![result])
    }

    /// The result type, if this is `IO<result>`.
    pub fn io_result(&self) -> Option<&Type> {
        match self {
            Type::Con(name, args) if name.as_ref() == "IO" => args.first(),
            _ => None,
        }
    }

    pub fn fun(from: Type, to: Type) -> Type {
        Type::Fun(Box::new(from), Box::new(to))
    }

    /// `a -> b -> c` from `[a, b]` and `c`.
    pub fn curried(
        params: impl IntoIterator<Item = Type, IntoIter: DoubleEndedIterator>,
        ret: Type,
    ) -> Type {
        params
            .into_iter()
            .rev()
            .fold(ret, |acc, param| Type::fun(param, acc))
    }
}

/// A polymorphic type `forall a b. t`; quantified variables appear as
/// [`Type::Gen`] indices `0..arity`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scheme {
    pub arity: u32,
    pub ty: Type,
}

impl Scheme {
    pub fn mono(ty: Type) -> Self {
        Scheme { arity: 0, ty }
    }
}

/// Assigns readable names (`a`, `b`, ..., `z`, `t26`, ...) to variables in
/// order of first appearance, so the same type always prints the same way.
#[derive(Default)]
struct Namer {
    vars: HashMap<TyVar, String>,
    gens: HashMap<u32, String>,
    next: usize,
}

impl Namer {
    fn fresh(&mut self) -> String {
        let n = self.next;
        self.next += 1;
        if n < 26 {
            ((b'a' + n as u8) as char).to_string()
        } else {
            format!("t{n}")
        }
    }

    fn write(&mut self, ty: &Type, out: &mut String, in_fun_lhs: bool) {
        match ty {
            Type::Var(v) => {
                if !self.vars.contains_key(v) {
                    let name = self.fresh();
                    self.vars.insert(*v, name);
                }
                out.push_str(&self.vars[v]);
            }
            Type::Gen(g) => {
                if !self.gens.contains_key(g) {
                    let name = self.fresh();
                    self.gens.insert(*g, name);
                }
                out.push_str(&self.gens[g]);
            }
            Type::Rigid(name) => out.push_str(name),
            Type::Con(name, args) => {
                out.push_str(name);
                if !args.is_empty() {
                    out.push('<');
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        self.write(arg, out, false);
                    }
                    out.push('>');
                }
            }
            Type::Fun(from, to) => {
                if in_fun_lhs {
                    out.push('(');
                }
                self.write(from, out, true);
                out.push_str(" -> ");
                self.write(to, out, false);
                if in_fun_lhs {
                    out.push(')');
                }
            }
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        Namer::default().write(self, &mut out, false);
        f.write_str(&out)
    }
}

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ty.fmt(f)
    }
}

/// Prints several types with one consistent variable naming, so `a` means
/// the same variable in "expected `a`, found `List<a>`".
pub fn display_together(types: &[&Type]) -> Vec<String> {
    let mut namer = Namer::default();
    types
        .iter()
        .map(|ty| {
            let mut out = String::new();
            namer.write(ty, &mut out, false);
            out
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_nested_function_types_with_parentheses() {
        let a = Type::Gen(0);
        let b = Type::Gen(1);
        let map = Type::curried(
            [Type::fun(a.clone(), b.clone()), Type::list(a)],
            Type::list(b),
        );
        assert_eq!(map.to_string(), "(a -> b) -> List<a> -> List<b>");
    }

    #[test]
    fn variables_are_named_in_order_of_appearance() {
        let ty = Type::fun(Type::Var(TyVar(7)), Type::Var(TyVar(3)));
        assert_eq!(ty.to_string(), "a -> b");
        let rigid = Type::fun(Type::Rigid("x".into()), Type::number());
        assert_eq!(rigid.to_string(), "x -> Number");
    }

    #[test]
    fn display_together_shares_names() {
        let v = Type::Var(TyVar(1));
        let shown = display_together(&[&v, &Type::list(v.clone())]);
        assert_eq!(shown, ["a", "List<a>"]);
    }

    #[test]
    fn constructor_arities() {
        assert_eq!(constructor_arity("List"), Some(1));
        assert_eq!(constructor_arity("IO"), Some(1));
        assert_eq!(constructor_arity("Number"), Some(0));
        assert_eq!(constructor_arity("Map"), None);
    }
}
