//! Random Lambda Discipline programs for fuzzing and property tests, in the
//! spirit of `csmith` and `wasm-smith`.
//!
//! [`program`] builds a [`Program`] from an [`arbitrary::Unstructured`]
//! byte source, so the same generator drives `cargo fuzz` targets (which
//! supply the bytes) and property tests (which use `arbtest` to supply
//! random ones). By default every program it builds is well-typed, and
//! records the type it meant each definition and top-level expression to
//! have; [`Config::syntactic`] relaxes that to programs that only parse.
//! [`mutate`] turns a well-typed program into one the type checker must
//! reject.
//!
//! Programs print as source text with `Display`. The generator has its own
//! small model of the language instead of using the compiler's AST, so it
//! checks the parser rather than sharing its assumptions, and its failures
//! are ordinary programs that can be pasted into a test.

mod generate;
mod model;
mod mutate;

pub use generate::Config;
pub use model::{DataDecl, Def, Expr, Pat, Program, Ty, float_literal};
pub use mutate::mutate;

use arbitrary::{Result, Unstructured};

/// Generates a program within the limits of `config`.
pub fn program(u: &mut Unstructured, config: &Config) -> Result<Program> {
    generate::Gen::new(u, config.clone()).program()
}
