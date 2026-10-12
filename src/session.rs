//! The compiler driver: runs the pipeline and keeps state across units.
//!
//! ```text
//! text ─lex/parse─▶ AST ─use expansion─▶ items ─resolve─▶ HIR ─infer─▶ types
//!                                                         └──lower──▶ Core ─▶ Backend
//! ```
//!
//! A [`Session`] remembers every global defined so far, so the REPL can feed
//! it one unit per input and each unit sees the earlier ones.

use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;

use crate::backend::Backend;
use crate::backend::interp::{Interpreter, Value};
use crate::core::{self, lower_unit};
use crate::diagnostics::{Diagnostic, Diagnostics};
use crate::hir::{GlobalId, Unit};
use crate::loader::{self, Loaded};
use crate::resolve::{GlobalScope, resolve};
use crate::source::{SourceMap, Span};
use crate::syntax::{parse_module, token};
use crate::types::{Scheme, check_expr, check_unit};

/// A global known to the session.
#[derive(Debug, Clone)]
pub struct GlobalInfo {
    pub name: Rc<str>,
    pub scheme: Scheme,
    pub span: Span,
}

/// A unit that passed every check, ready for a backend.
#[derive(Debug)]
pub struct Compiled {
    pub unit: Unit,
    pub def_schemes: Vec<Scheme>,
    pub entry_schemes: Vec<Scheme>,
    pub module: core::Module,
}

/// What evaluating some REPL input produced.
#[derive(Debug)]
pub struct Evaluated {
    /// New definitions with their types.
    pub definitions: Vec<(Rc<str>, Scheme)>,
    /// Values of the top-level expressions, with their types.
    pub values: Vec<(Value, Scheme)>,
}

/// Intermediate representations `lbd check --emit` can print.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Stage {
    Tokens,
    Ast,
    Types,
    Core,
}

pub struct Session {
    pub sources: SourceMap,
    globals: Vec<GlobalInfo>,
    by_name: HashMap<Rc<str>, GlobalId>,
    interpreter: Interpreter,
}

impl Session {
    /// A session whose programs print to `out`.
    pub fn new(out: Box<dyn Write>) -> Self {
        Self {
            sources: SourceMap::new(),
            globals: Vec::new(),
            by_name: HashMap::new(),
            interpreter: Interpreter::new(out),
        }
    }

    /// Forgets every definition (the REPL's `:reset`). Sources are kept so
    /// old diagnostics can still be rendered.
    pub fn reset(&mut self) {
        self.globals.clear();
        self.by_name.clear();
        self.interpreter = Interpreter::new(self.take_output());
    }

    fn take_output(&mut self) -> Box<dyn Write> {
        let placeholder = Interpreter::new(Box::new(std::io::sink()));
        std::mem::replace(&mut self.interpreter, placeholder).into_output()
    }

    /// The latest binding of every user-defined name, sorted by name.
    pub fn bindings(&self) -> Vec<&GlobalInfo> {
        let mut bindings: Vec<_> = self
            .by_name
            .values()
            .map(|id| &self.globals[id.index()])
            .collect();
        bindings.sort_by(|a, b| a.name.cmp(&b.name));
        bindings
    }

    /// The current binding of a user-defined name.
    pub fn binding(&self, name: &str) -> Option<&GlobalInfo> {
        self.by_name.get(name).map(|id| &self.globals[id.index()])
    }

    /// Runs the pipeline up to Core and records the unit's globals.
    pub fn compile(&mut self, loaded: Loaded) -> Result<Compiled, Diagnostics> {
        if !loaded.diagnostics.is_empty() {
            return Err(loaded.diagnostics);
        }
        let resolved = resolve(loaded.items, self);
        let prior: Vec<Scheme> = self.globals.iter().map(|g| g.scheme.clone()).collect();
        let typed = check_unit(&resolved.unit, &prior);
        let mut diagnostics = resolved.diagnostics;
        diagnostics.extend(typed.diagnostics);
        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }
        let module = lower_unit(&resolved.unit);
        for (def, scheme) in resolved.unit.defs.iter().zip(&typed.def_schemes) {
            debug_assert_eq!(def.id.index(), self.globals.len());
            self.globals.push(GlobalInfo {
                name: def.name.clone(),
                scheme: scheme.clone(),
                span: def.name_span,
            });
            self.by_name.insert(def.name.clone(), def.id);
        }
        Ok(Compiled {
            unit: resolved.unit,
            def_schemes: typed.def_schemes,
            entry_schemes: typed.entry_schemes,
            module,
        })
    }

    /// Compiles and runs a unit, returning the values of its top-level expressions.
    pub fn execute(&mut self, loaded: Loaded) -> Result<(Compiled, Vec<Value>), Diagnostics> {
        let compiled = self.compile(loaded)?;
        let values = self
            .interpreter
            .run(&compiled.module)
            .map_err(|error| vec![error.into_diagnostic()])?;
        Ok((compiled, values))
    }

    /// Runs a program file.
    pub fn run_file(&mut self, path: &Path) -> Result<Vec<Value>, Diagnostics> {
        let loaded = loader::load_file(&mut self.sources, path);
        self.execute(loaded).map(|(_, values)| values)
    }

    /// Type-checks a program file without running it.
    pub fn check_file(&mut self, path: &Path) -> Result<Compiled, Diagnostics> {
        let loaded = loader::load_file(&mut self.sources, path);
        self.compile(loaded)
    }

    /// Evaluates source text as a new unit (REPL input). `use` paths are
    /// relative to the working directory.
    pub fn eval(&mut self, name: &str, text: &str) -> Result<Evaluated, Diagnostics> {
        let loaded = loader::load_text(&mut self.sources, name, text);
        self.evaluated(loaded)
    }

    /// Loads a file into the session (REPL `:load`), running its top-level expressions.
    pub fn load(&mut self, path: &Path) -> Result<Evaluated, Diagnostics> {
        let loaded = loader::load_file(&mut self.sources, path);
        self.evaluated(loaded)
    }

    fn evaluated(&mut self, loaded: Loaded) -> Result<Evaluated, Diagnostics> {
        let (compiled, values) = self.execute(loaded)?;
        Ok(Evaluated {
            definitions: compiled
                .unit
                .defs
                .iter()
                .map(|def| def.name.clone())
                .zip(compiled.def_schemes)
                .collect(),
            values: values.into_iter().zip(compiled.entry_schemes).collect(),
        })
    }

    /// The type of an expression, without evaluating it (REPL `:type`).
    pub fn type_of(&mut self, text: &str) -> Result<Scheme, Diagnostics> {
        let loaded = loader::load_text(&mut self.sources, "<type>", text);
        if !loaded.diagnostics.is_empty() {
            return Err(loaded.diagnostics);
        }
        let resolved = resolve(loaded.items, self);
        if !resolved.diagnostics.is_empty() {
            return Err(resolved.diagnostics);
        }
        let unit = resolved.unit;
        match (unit.entries.as_slice(), unit.defs.as_slice()) {
            ([body], []) => {
                let prior: Vec<Scheme> = self.globals.iter().map(|g| g.scheme.clone()).collect();
                let (scheme, diagnostics) = check_expr(body, &prior);
                if diagnostics.is_empty() {
                    Ok(scheme)
                } else {
                    Err(diagnostics)
                }
            }
            _ => Err(vec![Diagnostic::error(
                "`:type` expects exactly one expression",
            )]),
        }
    }

    /// Prints an intermediate representation of a file.
    pub fn emit(&mut self, path: &Path, stage: Stage) -> Result<String, Diagnostics> {
        match stage {
            Stage::Tokens | Stage::Ast => {
                let text = std::fs::read_to_string(path).map_err(|e| {
                    vec![Diagnostic::error(format!(
                        "cannot read `{}`: {e}",
                        path.display()
                    ))]
                })?;
                let file = self
                    .sources
                    .add_virtual(path.display().to_string(), text.as_str());
                if stage == Stage::Tokens {
                    let (tokens, diagnostics) = token::lex(file, &text);
                    if !diagnostics.is_empty() {
                        return Err(diagnostics);
                    }
                    Ok(tokens
                        .iter()
                        .map(|t| {
                            let (line, column) =
                                self.sources.get(file).line_col(t.span.start as usize);
                            format!(
                                "{line}:{column} {:?} {:?}\n",
                                t.kind,
                                self.sources.snippet(t.span)
                            )
                        })
                        .collect())
                } else {
                    let (module, diagnostics) = parse_module(file, &text);
                    if diagnostics.is_empty() {
                        Ok(module.to_string())
                    } else {
                        Err(diagnostics)
                    }
                }
            }
            Stage::Types => {
                let compiled = self.check_file(path)?;
                Ok(compiled
                    .unit
                    .defs
                    .iter()
                    .zip(&compiled.def_schemes)
                    .map(|(def, scheme)| format!("{} : {}\n", def.name, scheme))
                    .collect())
            }
            Stage::Core => Ok(self.check_file(path)?.module.to_string()),
        }
    }

    pub fn flush(&mut self) {
        self.interpreter.flush();
    }

    /// Limits the interpreter's evaluation stack (see [`Interpreter::set_max_frames`]).
    pub fn set_max_frames(&mut self, max_frames: usize) {
        self.interpreter.set_max_frames(max_frames);
    }

    /// Renders diagnostics against this session's sources.
    pub fn render(&self, diagnostics: &[Diagnostic], out: &mut dyn Write, color: bool) {
        let _ = crate::diagnostics::render_all(diagnostics, &self.sources, out, color);
    }
}

impl GlobalScope for Session {
    fn lookup(&self, name: &str) -> Option<GlobalId> {
        self.by_name.get(name).copied()
    }

    fn names(&self) -> Vec<Rc<str>> {
        self.by_name.keys().cloned().collect()
    }

    fn next_id(&self) -> u32 {
        self.globals.len() as u32
    }
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("globals", &self.globals.len())
            .finish_non_exhaustive()
    }
}
