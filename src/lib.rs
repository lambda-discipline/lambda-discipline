//! Lambda Discipline: a lazy, statically typed functional language based on
//! the lambda calculus.

pub mod backend;
pub mod builtins;
pub mod core;
pub mod diagnostics;
pub mod hir;
pub mod loader;
pub mod output;
pub mod resolve;
pub mod session;
pub mod source;
pub mod syntax;
pub mod types;

pub use session::Session;
