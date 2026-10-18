//! Type representation shared by the checker and later stages.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

use crate::hir::TyConId;

/// A unification variable, owned by an inference context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TyVar(pub u32);

/// A type constructor: a built-in type such as `Int` or `List`, or a
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
    /// A type constructor applied to arguments: `Int`, `List<a>`.
    Con(TyCon, Vec<Type>),
    /// A function type.
    Fun(Box<Type>, Box<Type>),
}

impl Type {
    fn builtin(id: TyConId, args: Vec<Type>) -> Type {
        Type::Con(TyCon::builtin(id), args)
    }

    pub fn int() -> Type {
        Type::builtin(TyConId::INT, Vec::new())
    }

    pub fn float() -> Type {
        Type::builtin(TyConId::FLOAT, Vec::new())
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

    /// `(a, b, ...)`, with 2 to [`crate::syntax::ast::MAX_TUPLE_ARITY`]
    /// components.
    pub fn tuple(components: Vec<Type>) -> Type {
        Type::builtin(TyConId::tuple(components.len()), components)
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

/// One of the built-in type classes. A type belongs to a class when the
/// class's builtins work on its values. Every type built without functions
/// or `IO` actions belongs to the structural classes `Eq`, `Ord` and `Show`;
/// only `Int` and `Float` belong to `Num`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Class {
    /// Equality: `eq`, `neq`.
    Eq,
    /// Ordering: `lt`, `compare`, `sort`, ...; implies `Eq`.
    Ord,
    /// Arithmetic: `add`, `sub`, `mul`, `neg`, `abs`, `pow`.
    Num,
    /// Conversion to source-like text: `show`, `print`.
    Show,
}

/// How many classes there are, for tables indexed by [`Class::index`].
pub const CLASS_COUNT: usize = Class::ALL.len();

impl Class {
    pub const ALL: [Class; 4] = [Class::Eq, Class::Ord, Class::Num, Class::Show];

    /// The classes every type without functions or `IO` actions belongs to.
    pub const STRUCTURAL: [Class; 3] = [Class::Eq, Class::Ord, Class::Show];

    pub fn name(self) -> &'static str {
        match self {
            Class::Eq => "Eq",
            Class::Ord => "Ord",
            Class::Num => "Num",
            Class::Show => "Show",
        }
    }

    pub fn lookup(name: &str) -> Option<Class> {
        Class::ALL.into_iter().find(|class| class.name() == name)
    }

    pub fn index(self) -> usize {
        self as usize
    }

    /// What the class's builtins do, for messages: "functions cannot be
    /// compared".
    pub fn verb(self) -> &'static str {
        match self {
            Class::Eq => "compared",
            Class::Ord => "ordered",
            Class::Num => "used in arithmetic",
            Class::Show => "shown",
        }
    }
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A set of classes, closed under superclasses (`Ord` implies `Eq`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Classes(u8);

impl Classes {
    pub const NONE: Classes = Classes(0);

    pub fn of(class: Class) -> Classes {
        let mut set = Classes(1 << class.index());
        if class == Class::Ord {
            set.0 |= 1 << Class::Eq.index();
        }
        set
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn contains(self, class: Class) -> bool {
        self.0 & (1 << class.index()) != 0
    }

    pub fn union(self, other: Classes) -> Classes {
        Classes(self.0 | other.0)
    }

    pub fn with(self, class: Class) -> Classes {
        self.union(Classes::of(class))
    }

    /// The classes in `self` that are not in `other`.
    pub fn minus(self, other: Classes) -> Classes {
        Classes(self.0 & !other.0)
    }

    pub fn iter(self) -> impl Iterator<Item = Class> {
        Class::ALL
            .into_iter()
            .filter(move |&class| self.contains(class))
    }

    /// The classes to write in a context: `Eq` is implied by `Ord`.
    pub fn shown(self) -> impl Iterator<Item = Class> {
        self.iter()
            .filter(move |&class| !(class == Class::Eq && self.contains(Class::Ord)))
    }
}

/// A polymorphic type `forall a b. C => t`; quantified variables appear as
/// [`Type::Gen`] indices `0..arity`, and `bounds[i]` holds the classes
/// variable `i` must belong to (`bounds` may be shorter than `arity` when
/// the trailing variables are unconstrained).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scheme {
    pub arity: u32,
    pub bounds: Vec<Classes>,
    pub ty: Type,
}

impl Scheme {
    pub fn mono(ty: Type) -> Self {
        Scheme::poly(0, ty)
    }

    /// A scheme whose variables are unconstrained.
    pub fn poly(arity: u32, ty: Type) -> Self {
        Scheme {
            arity,
            bounds: Vec::new(),
            ty,
        }
    }

    /// The classes quantified variable `index` must belong to.
    pub fn bound(&self, index: u32) -> Classes {
        self.bounds
            .get(index as usize)
            .copied()
            .unwrap_or(Classes::NONE)
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
            Type::Con(con, args) if con.id.tuple_arity().is_some() => {
                out.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    self.write(arg, out, false);
                }
                out.push(')');
            }
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

/// Prints `Eq<a> => a -> a -> Bool`, with several constraints in
/// parentheses, ordered by class and then by variable name.
impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut namer = Namer::new(&[&self.ty]);
        let mut ty = String::new();
        namer.write(&self.ty, &mut ty, false);
        let mut constraints: Vec<(Class, String)> = Vec::new();
        for index in 0..self.arity {
            let bound = self.bound(index);
            if bound.is_empty() {
                continue;
            }
            let mut name = String::new();
            namer.write(&Type::Gen(index), &mut name, false);
            constraints.extend(bound.shown().map(|class| (class, name.clone())));
        }
        constraints.sort();
        let context: Vec<String> = constraints
            .iter()
            .map(|(class, name)| format!("{class}<{name}>"))
            .collect();
        match context.len() {
            0 => f.write_str(&ty),
            1 => write!(f, "{} => {ty}", context[0]),
            _ => write!(f, "({}) => {ty}", context.join(", ")),
        }
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
        let rigid = Type::fun(Type::Rigid("x".into()), Type::int());
        assert_eq!(rigid.to_string(), "x -> Int");
    }

    #[test]
    fn inferred_variables_skip_rigid_names() {
        let annotation = Type::fun(Type::Rigid("a".into()), Type::int());
        let found = Type::fun(Type::list(Type::Var(TyVar(0))), Type::int());
        let shown = display_together(&[&annotation, &found]);
        assert_eq!(shown, ["a -> Int", "List<b> -> Int"]);
        // A rigid name that appears after the variable is reserved too.
        let ty = Type::fun(Type::Var(TyVar(0)), Type::Rigid("a".into()));
        assert_eq!(ty.to_string(), "b -> a");
    }

    #[test]
    fn schemes_show_their_context() {
        let a = Type::Gen(0);
        let b = Type::Gen(1);
        let eq = Scheme {
            arity: 1,
            bounds: vec![Classes::of(Class::Eq)],
            ty: Type::curried([a.clone(), a.clone()], Type::bool()),
        };
        assert_eq!(eq.to_string(), "Eq<a> => a -> a -> Bool");
        // `Ord` implies `Eq`, and constraints are ordered by class.
        let both = Scheme {
            arity: 2,
            bounds: vec![
                Classes::of(Class::Show),
                Classes::of(Class::Eq).with(Class::Ord),
            ],
            ty: Type::fun(a, b),
        };
        assert_eq!(both.to_string(), "(Ord<b>, Show<a>) => a -> b");
        assert_eq!(Scheme::poly(1, Type::Gen(0)).to_string(), "a");
    }

    #[test]
    fn class_sets_include_superclasses() {
        let ord = Classes::of(Class::Ord);
        assert!(ord.contains(Class::Eq));
        assert_eq!(ord.shown().collect::<Vec<_>>(), [Class::Ord]);
        assert_eq!(
            ord.with(Class::Show).minus(Classes::of(Class::Eq)),
            Classes::of(Class::Show).union(Classes(1 << Class::Ord.index()))
        );
        assert_eq!(Class::lookup("Show"), Some(Class::Show));
        assert_eq!(Class::lookup("Num"), Some(Class::Num));
        assert_eq!(Class::lookup("Monoid"), None);
    }

    #[test]
    fn display_together_shares_names() {
        let v = Type::Var(TyVar(1));
        let shown = display_together(&[&v, &Type::list(v.clone())]);
        assert_eq!(shown, ["a", "List<a>"]);
    }
}
