//! The type system: type representation, the type environment,
//! Hindley-Milner inference and pattern exhaustiveness.

pub mod env;
pub mod exhaustive;
pub mod infer;
pub mod ty;

pub use env::TypeEnv;
pub use infer::{TopLevel, Typed, builtin_scheme, check_expr, check_unit};
pub use ty::{Scheme, Type};
