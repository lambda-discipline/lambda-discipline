//! Backends consume Core IR.
//!
//! Today there is one, the lazy [`interp::Interpreter`]. The [`Backend`]
//! trait is the seam for future ones: a transpiler or a native/uxn/LLVM code
//! generator would implement it with an `Output` such as generated source or
//! an object file, reusing the whole front end and the Core IR unchanged.

pub mod interp;

use thiserror::Error;

use crate::core::Module;
use crate::diagnostics::Diagnostic;
use crate::source::Span;

pub trait Backend {
    type Output;

    /// Processes one lowered unit. Units arrive in order and may refer to
    /// globals of earlier units (the REPL feeds one unit per input).
    fn run(&mut self, module: &Module) -> Result<Self::Output, RuntimeError>;
}

/// An error raised while a program runs.
#[derive(Debug, Clone, PartialEq, Error)]
#[error("{message}")]
pub struct RuntimeError {
    pub message: String,
    pub span: Option<Span>,
    pub label: Option<String>,
    pub notes: Vec<String>,
}

impl RuntimeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
            label: None,
            notes: Vec::new(),
        }
    }

    /// The error of an evaluation stopped by the user (Ctrl-C).
    pub fn interrupted() -> Self {
        Self::new("interrupted")
    }

    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn into_diagnostic(self) -> Diagnostic {
        let mut diagnostic = Diagnostic::error(self.message);
        diagnostic.span = self.span;
        diagnostic.label = self.label;
        diagnostic.notes = self.notes;
        diagnostic
    }
}
