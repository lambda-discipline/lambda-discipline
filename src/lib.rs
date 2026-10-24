//! Lambda Discipline: a lazy, statically typed functional language based on
//! the lambda calculus.
//!
//! The crate is organized as a compiler pipeline; see [`session`] for how
//! the stages fit together:
//!
//! * [`source`], [`diagnostics`]: files, spans and error reporting.
//! * [`syntax`]: `logos` lexer and recursive-descent parser to an AST.
//! * [`loader`]: expands `use` items.
//! * [`resolve`] to [`hir`]: name resolution.
//! * [`types`]: Hindley-Milner type inference.
//! * [`lint`]: warnings about code that type-checks.
//! * [`core`]: the closure-converted, uncurried IR that backends consume.
//! * [`backend`]: the [`backend::Backend`] trait and the lazy interpreter.
//! * [`builtins`]: the primitive operations, shared by every stage.
//! * [`repl`]: the interactive loop.

pub mod backend;
pub mod builtins;
pub mod cli;
pub mod core;
pub mod diagnostics;
pub mod hir;
mod index;
pub mod lint;
pub mod loader;
pub mod output;
pub mod repl;
pub mod resolve;
pub mod session;
pub mod source;
pub mod syntax;
pub mod types;

pub use session::Session;
