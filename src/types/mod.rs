//! The type system: type representation and Hindley-Milner inference.

pub mod infer;
pub mod ty;

pub use infer::{Typed, builtin_scheme, check_expr, check_unit};
pub use ty::{Scheme, Type};
