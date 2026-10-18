//! The type environment: every type constructor and data constructor a
//! session knows, built-in or declared with `type`.

use std::collections::HashMap;
use std::rc::Rc;

use crate::hir::{ConId, TyConId};
use crate::source::Span;

use super::ty::{CLASS_COUNT, Class, Scheme, TyCon, Type};

/// A type's instance of one class: `None` when the type does not belong to
/// it, otherwise the indices of the parameters that must belong to it too
/// (`List<a>` is in `Eq` when `a` is, so its `Eq` instance is `Some([0])`).
pub type Instance = Option<Vec<usize>>;

/// A type that exists without a declaration.
pub struct BuiltinType {
    pub name: &'static str,
    pub arity: usize,
    /// Its constructors, in order; empty for primitive types.
    pub constructors: &'static [&'static str],
    /// The classes it belongs to when its parameters do. `IO` belongs to
    /// none: actions cannot be compared or shown.
    pub classes: &'static [Class],
}

const NUMERIC: &[Class] = &[Class::Eq, Class::Ord, Class::Num, Class::Show];

/// The built-in types, indexed by [`TyConId`]. Their constructors take the
/// first [`ConId`]s, in order.
pub const BUILTIN_TYPES: &[BuiltinType] = &[
    BuiltinType {
        name: "Int",
        arity: 0,
        constructors: &[],
        classes: NUMERIC,
    },
    BuiltinType {
        name: "Float",
        arity: 0,
        constructors: &[],
        classes: NUMERIC,
    },
    BuiltinType {
        name: "String",
        arity: 0,
        constructors: &[],
        classes: &Class::STRUCTURAL,
    },
    BuiltinType {
        name: "Bool",
        arity: 0,
        constructors: &["False", "True"],
        classes: &Class::STRUCTURAL,
    },
    BuiltinType {
        name: "Unit",
        arity: 0,
        constructors: &[],
        classes: &Class::STRUCTURAL,
    },
    BuiltinType {
        name: "List",
        arity: 1,
        constructors: &[],
        classes: &Class::STRUCTURAL,
    },
    BuiltinType {
        name: "IO",
        arity: 1,
        constructors: &[],
        classes: &[],
    },
];

/// The constructors of the built-in types, in [`ConId`] order.
pub fn builtin_constructors() -> impl Iterator<Item = (ConId, &'static str)> {
    BUILTIN_TYPES
        .iter()
        .flat_map(|ty| ty.constructors.iter().copied())
        .enumerate()
        .map(|(i, name)| (ConId(i as u32), name))
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub con: TyCon,
    /// Names of the type parameters (`a` in `Option<a>`).
    pub params: Vec<Rc<str>>,
    /// The data constructors in declaration order; empty for primitive types.
    pub constructors: Vec<ConId>,
    /// Where it was declared; `None` for built-in types.
    pub span: Option<Span>,
    /// Its instance of each class, indexed by [`Class::index`].
    pub instances: [Instance; CLASS_COUNT],
}

impl TypeInfo {
    pub fn arity(&self) -> usize {
        self.params.len()
    }
}

#[derive(Debug, Clone)]
pub struct ConInfo {
    pub id: ConId,
    pub name: Rc<str>,
    /// The type it constructs.
    pub ty: TyConId,
    /// `forall params. field -> ... -> Type<params>`.
    pub scheme: Scheme,
    pub arity: usize,
    pub span: Option<Span>,
}

/// The types and constructors visible to a unit. Ids index the tables;
/// names map to the latest definition, so a REPL redefinition shadows an
/// older type without changing it.
#[derive(Debug, Clone)]
pub struct TypeEnv {
    types: Vec<TypeInfo>,
    constructors: Vec<ConInfo>,
    type_names: HashMap<Rc<str>, TyConId>,
    constructor_names: HashMap<Rc<str>, ConId>,
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeEnv {
    /// The built-in types and their constructors.
    pub fn new() -> Self {
        let mut env = TypeEnv {
            types: Vec::new(),
            constructors: Vec::new(),
            type_names: HashMap::new(),
            constructor_names: HashMap::new(),
        };
        let mut next_con = 0;
        for (index, builtin) in BUILTIN_TYPES.iter().enumerate() {
            let con = TyCon::builtin(TyConId(index as u32));
            let params: Vec<Rc<str>> = (0..builtin.arity)
                .map(|i| Rc::from(((b'a' + i as u8) as char).to_string()))
                .collect();
            let constructors: Vec<ConId> = (0..builtin.constructors.len())
                .map(|i| ConId(next_con + i as u32))
                .collect();
            next_con += constructors.len() as u32;
            for (&id, &name) in constructors.iter().zip(builtin.constructors) {
                env.add_constructor(ConInfo {
                    id,
                    name: Rc::from(name),
                    ty: con.id,
                    scheme: Scheme::mono(Type::Con(con.clone(), Vec::new())),
                    arity: 0,
                    span: None,
                });
            }
            let instances = Class::ALL.map(|class| {
                builtin
                    .classes
                    .contains(&class)
                    .then(|| (0..builtin.arity).collect())
            });
            env.add_type(TypeInfo {
                con,
                params,
                constructors,
                span: None,
                instances,
            });
        }
        env
    }

    /// Adds a type; its id must be the next one.
    pub fn add_type(&mut self, info: TypeInfo) {
        assert_eq!(
            info.con.id.index(),
            self.types.len(),
            "type ids are consecutive"
        );
        self.type_names.insert(info.con.name.clone(), info.con.id);
        self.types.push(info);
    }

    /// Adds a constructor; its id must be the next one.
    pub fn add_constructor(&mut self, info: ConInfo) {
        assert_eq!(
            info.id.index(),
            self.constructors.len(),
            "constructor ids are consecutive"
        );
        self.constructor_names.insert(info.name.clone(), info.id);
        self.constructors.push(info);
    }

    pub fn ty(&self, id: TyConId) -> &TypeInfo {
        &self.types[id.index()]
    }

    /// The parameters of `id` that must belong to `class` for `id` to, or
    /// `None` if `id` never belongs to it.
    pub fn instance(&self, id: TyConId, class: Class) -> Option<&[usize]> {
        self.ty(id).instances[class.index()].as_deref()
    }

    pub fn set_instances(&mut self, id: TyConId, instances: [Instance; CLASS_COUNT]) {
        self.types[id.index()].instances = instances;
    }

    pub fn constructor(&self, id: ConId) -> &ConInfo {
        &self.constructors[id.index()]
    }

    /// The current type with this name.
    pub fn lookup_type(&self, name: &str) -> Option<&TypeInfo> {
        self.type_names.get(name).map(|&id| self.ty(id))
    }

    /// The current constructor with this name.
    pub fn lookup_constructor(&self, name: &str) -> Option<ConId> {
        self.constructor_names.get(name).copied()
    }

    /// Names of the current types, unordered.
    pub fn type_names(&self) -> impl Iterator<Item = &Rc<str>> {
        self.type_names.keys()
    }

    /// Names of the current constructors, unordered.
    pub fn constructor_names(&self) -> impl Iterator<Item = &Rc<str>> {
        self.constructor_names.keys()
    }

    pub fn next_type_id(&self) -> u32 {
        self.types.len() as u32
    }

    pub fn next_constructor_id(&self) -> u32 {
        self.constructors.len() as u32
    }

    /// A type's declaration as it would be written:
    /// `type Option<a> = None | Some a`, or just `type Int`.
    pub fn describe(&self, id: TyConId) -> String {
        let info = self.ty(id);
        let mut out = format!("type {}", info.con.name);
        if !info.params.is_empty() {
            let params: Vec<&str> = info.params.iter().map(|p| p.as_ref()).collect();
            out.push_str(&format!("<{}>", params.join(", ")));
        }
        let names: Vec<Type> = info.params.iter().map(|p| Type::Rigid(p.clone())).collect();
        for (i, &con) in info.constructors.iter().enumerate() {
            let con = self.constructor(con);
            out.push_str(if i == 0 { " = " } else { " | " });
            out.push_str(&con.name);
            let mut ty = name_params(&con.scheme.ty, &names);
            while let Type::Fun(field, rest) = ty {
                match *field {
                    Type::Fun(..) => out.push_str(&format!(" ({field})")),
                    _ => out.push_str(&format!(" {field}")),
                }
                ty = *rest;
            }
        }
        out
    }

    /// The constructors of the type a constructor belongs to.
    pub fn siblings(&self, id: ConId) -> &[ConId] {
        &self.ty(self.constructor(id).ty).constructors
    }
}

/// Replaces quantified variables by the parameter names they stand for.
fn name_params(ty: &Type, names: &[Type]) -> Type {
    match ty {
        Type::Gen(i) => names
            .get(*i as usize)
            .cloned()
            .unwrap_or_else(|| Type::Rigid(Rc::from("?"))),
        Type::Con(con, args) => Type::Con(
            con.clone(),
            args.iter().map(|a| name_params(a, names)).collect(),
        ),
        Type::Fun(from, to) => Type::fun(name_params(from, names), name_params(to, names)),
        Type::Var(_) | Type::Rigid(_) => ty.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_types_and_constructors() {
        let env = TypeEnv::new();
        assert_eq!(env.lookup_type("List").unwrap().arity(), 1);
        assert_eq!(env.lookup_type("Int").unwrap().con.id, TyConId::INT);
        assert_eq!(env.lookup_type("Float").unwrap().con.id, TyConId::FLOAT);
        assert!(env.lookup_type("Map").is_none());
        assert_eq!(env.lookup_constructor("False"), Some(ConId::FALSE));
        assert_eq!(env.lookup_constructor("True"), Some(ConId::TRUE));
        assert_eq!(env.siblings(ConId::TRUE), [ConId::FALSE, ConId::TRUE]);
        assert_eq!(env.constructor(ConId::TRUE).scheme.to_string(), "Bool");
        for (index, builtin) in BUILTIN_TYPES.iter().enumerate() {
            assert_eq!(
                env.ty(TyConId(index as u32)).con.name.as_ref(),
                builtin.name
            );
        }
        assert_eq!(env.describe(TyConId::BOOL), "type Bool = False | True");
        assert_eq!(env.describe(TyConId::LIST), "type List<a>");
        assert_eq!(env.instance(TyConId::LIST, Class::Eq), Some(&[0][..]));
        assert_eq!(env.instance(TyConId::INT, Class::Ord), Some(&[][..]));
        assert_eq!(env.instance(TyConId::FLOAT, Class::Num), Some(&[][..]));
        assert_eq!(env.instance(TyConId::LIST, Class::Num), None);
        assert_eq!(env.instance(TyConId::IO, Class::Show), None);
        assert_eq!(
            builtin_constructors().collect::<Vec<_>>(),
            [(ConId::FALSE, "False"), (ConId::TRUE, "True")]
        );
    }
}
