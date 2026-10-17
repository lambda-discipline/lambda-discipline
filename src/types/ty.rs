//! Type representation shared by the checker and later stages.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

use crate::hir::TyConId;

/// A unification variable, owned by an inference context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TyVar(pub u32);

/// A type constructor: a built-in type such as `Number` or `List`, or a
/// declared data type. Two constructors are the same when their ids are,
/// whatever their names.
#[derive(Debug, Clone)]
pub struct TyCon {
    pub id: TyConId,
    pub name: Rc<str>,
}

impl PartialEq for TyCon {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TyCon {}

impl TyCon {
    /// One of the built-in type constructors (see [`super::env::BUILTIN_TYPES`]).
    pub fn builtin(id: TyConId) -> TyCon {
        TyCon {
            id,
            name: Rc::from(super::env::BUILTIN_TYPES[id.index()].name),
        }
    }
}

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
    Con(TyCon, Vec<Type>),
    /// A function type.
    Fun(Box<Type>, Box<Type>),
}

impl Type {
    fn builtin(id: TyConId, args: Vec<Type>) -> Type {
        Type::Con(TyCon::builtin(id), args)
    }

    pub fn number() -> Type {
        Type::builtin(TyConId::NUMBER, Vec::new())
    }

    pub fn string() -> Type {
        Type::builtin(TyConId::STRING, Vec::new())
    }

    pub fn bool() -> Type {
        Type::builtin(TyConId::BOOL, Vec::new())
    }

    pub fn unit() -> Type {
        Type::builtin(TyConId::UNIT, Vec::new())
    }

    pub fn list(element: Type) -> Type {
        Type::builtin(TyConId::LIST, vec![element])
    }

    /// `IO<result>`: an action producing a `result` when it runs.
    pub fn io(result: Type) -> Type {
        Type::builtin(TyConId::IO, vec![result])
    }

    /// The result type, if this is `IO<result>`.
    pub fn io_result(&self) -> Option<&Type> {
        match self {
            Type::Con(con, args) if con.id == TyConId::IO => args.first(),
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
/// Names that rigid variables already use are skipped, so an inferred
/// variable never prints like an annotation's `a`.
struct Namer {
    vars: HashMap<TyVar, String>,
    gens: HashMap<u32, String>,
    reserved: HashSet<Rc<str>>,
    next: usize,
}

impl Namer {
    /// A namer for printing `types` together, reserving their rigid names.
    fn new(types: &[&Type]) -> Self {
        let mut reserved = HashSet::new();
        for ty in types {
            ty.collect_rigid(&mut reserved);
        }
        Namer {
            vars: HashMap::new(),
            gens: HashMap::new(),
            reserved,
            next: 0,
        }
    }

    fn fresh(&mut self) -> String {
        loop {
            let n = self.next;
            self.next += 1;
            let name = if n < 26 {
                ((b'a' + n as u8) as char).to_string()
            } else {
                format!("t{n}")
            };
            if !self.reserved.contains(name.as_str()) {
                return name;
            }
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
            Type::Con(con, args) => {
                out.push_str(&con.name);
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

impl Type {
    fn collect_rigid(&self, names: &mut HashSet<Rc<str>>) {
        match self {
            Type::Rigid(name) => {
                names.insert(name.clone());
            }
            Type::Con(_, args) => args.iter().for_each(|arg| arg.collect_rigid(names)),
            Type::Fun(from, to) => {
                from.collect_rigid(names);
                to.collect_rigid(names);
            }
            Type::Var(_) | Type::Gen(_) => {}
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        Namer::new(&[self]).write(self, &mut out, false);
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
    let mut namer = Namer::new(types);
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
    fn inferred_variables_skip_rigid_names() {
        let annotation = Type::fun(Type::Rigid("a".into()), Type::number());
        let found = Type::fun(Type::list(Type::Var(TyVar(0))), Type::number());
        let shown = display_together(&[&annotation, &found]);
        assert_eq!(shown, ["a -> Number", "List<b> -> Number"]);
        // A rigid name that appears after the variable is reserved too.
        let ty = Type::fun(Type::Var(TyVar(0)), Type::Rigid("a".into()));
        assert_eq!(ty.to_string(), "b -> a");
    }

    #[test]
    fn display_together_shares_names() {
        let v = Type::Var(TyVar(1));
        let shown = display_together(&[&v, &Type::list(v.clone())]);
        assert_eq!(shown, ["a", "List<a>"]);
    }
}
