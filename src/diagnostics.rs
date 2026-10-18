//! Compiler and runtime diagnostics, rendered with `ariadne`.
//!
//! Every stage reports problems as [`Diagnostic`] values instead of printing
//! or exiting, so the CLI, REPL and tests decide what to do with them.

use std::collections::HashMap;
use std::fmt;
use std::io::{self, Write};
use std::rc::Rc;

use ariadne::{Color, Config, IndexType, Label, Report, ReportKind, Source};

use crate::source::{FileId, SourceMap, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    /// The location the diagnostic is about. The primary label's text is `label`.
    pub span: Option<Span>,
    pub label: Option<String>,
    /// Extra locations, e.g. a previous definition.
    pub secondary: Vec<(Span, String)>,
    pub notes: Vec<String>,
    pub help: Option<String>,
}

pub type Diagnostics = Vec<Diagnostic>;

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span: None,
            label: None,
            secondary: Vec::new(),
            notes: Vec::new(),
            help: None,
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            ..Self::error(message)
        }
    }

    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_secondary(mut self, span: Span, label: impl Into<String>) -> Self {
        self.secondary.push((span, label.into()));
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Writes the diagnostic as a source-annotated report.
    pub fn render(&self, sources: &SourceMap, out: &mut dyn Write, color: bool) -> io::Result<()> {
        let Some(span) = self.span else {
            return self.render_plain(out);
        };
        let kind = match self.severity {
            Severity::Error => ReportKind::Error,
            Severity::Warning => ReportKind::Warning,
        };
        let primary = match self.severity {
            Severity::Error => Color::Red,
            Severity::Warning => Color::Yellow,
        };
        let config = Config::default()
            .with_color(color)
            .with_index_type(IndexType::Byte);
        let mut label = Label::new(AriadneSpan(span)).with_color(primary);
        if let Some(text) = &self.label {
            label = label.with_message(text);
        }
        let mut report = Report::build(kind, AriadneSpan(span))
            .with_config(config)
            .with_message(&self.message)
            .with_label(label);
        for (span, text) in &self.secondary {
            report = report.with_label(
                Label::new(AriadneSpan(*span))
                    .with_message(text)
                    .with_color(Color::Blue),
            );
        }
        for note in &self.notes {
            report = report.with_note(note);
        }
        if let Some(help) = &self.help {
            report = report.with_help(help);
        }
        report.finish().write(Cache::new(sources), out)
    }

    fn render_plain(&self, out: &mut dyn Write) -> io::Result<()> {
        writeln!(out, "{}: {}", self.severity, self.message)?;
        for note in &self.notes {
            writeln!(out, "  = note: {note}")?;
        }
        if let Some(help) = &self.help {
            writeln!(out, "  = help: {help}")?;
        }
        Ok(())
    }

    /// Renders to a string without colors; handy in tests and the REPL.
    pub fn to_plain_string(&self, sources: &SourceMap) -> String {
        let mut buffer = Vec::new();
        self.render(sources, &mut buffer, false)
            .expect("writing to a Vec cannot fail");
        String::from_utf8_lossy(&buffer).into_owned()
    }
}

/// Renders every diagnostic in order.
pub fn render_all(
    diagnostics: &[Diagnostic],
    sources: &SourceMap,
    out: &mut dyn Write,
    color: bool,
) -> io::Result<()> {
    for diagnostic in diagnostics {
        diagnostic.render(sources, out, color)?;
    }
    Ok(())
}

/// Whether stderr should be colored: a terminal, and `NO_COLOR` unset.
pub fn stderr_supports_color() -> bool {
    use std::io::IsTerminal;
    std::env::var_os("NO_COLOR").is_none() && io::stderr().is_terminal()
}

/// Adapts [`Span`] to ariadne's span trait.
#[derive(Debug, Clone, Copy)]
struct AriadneSpan(Span);

impl ariadne::Span for AriadneSpan {
    type SourceId = FileId;

    fn source(&self) -> &FileId {
        &self.0.file
    }

    fn start(&self) -> usize {
        self.0.start as usize
    }

    fn end(&self) -> usize {
        self.0.end as usize
    }
}

/// Lazily converts [`SourceMap`] files into ariadne sources.
struct Cache<'a> {
    map: &'a SourceMap,
    sources: HashMap<FileId, Source<Rc<str>>>,
}

impl<'a> Cache<'a> {
    fn new(map: &'a SourceMap) -> Self {
        Self {
            map,
            sources: HashMap::new(),
        }
    }
}

impl ariadne::Cache<FileId> for Cache<'_> {
    type Storage = Rc<str>;

    fn fetch(&mut self, id: &FileId) -> Result<&Source<Rc<str>>, impl fmt::Debug> {
        let map = self.map;
        Ok::<_, ()>(
            self.sources
                .entry(*id)
                .or_insert_with(|| Source::from(Rc::from(map.get(*id).text()))),
        )
    }

    fn display<'b>(&self, id: &'b FileId) -> Option<impl fmt::Display + 'b> {
        Some(self.map.get(*id).name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> (SourceMap, Span) {
        let mut map = SourceMap::new();
        let id = map.add_virtual("demo.lbd", "x = (add 1 \"two\")\n");
        (map, Span::new(id, 11..16))
    }

    #[test]
    fn renders_message_location_and_label() {
        let (map, span) = sample();
        let text = Diagnostic::error("type mismatch")
            .at(span)
            .with_label("expected `Int`, found `String`")
            .with_note("in a call to `add`")
            .to_plain_string(&map);
        assert!(text.contains("Error: type mismatch"), "{text}");
        assert!(text.contains("demo.lbd:1:12"), "{text}");
        assert!(text.contains("x = (add 1 \"two\")"), "{text}");
        assert!(text.contains("expected `Int`, found `String`"), "{text}");
        assert!(text.contains("in a call to `add`"), "{text}");
    }

    #[test]
    fn plain_diagnostic_without_span() {
        let map = SourceMap::new();
        let text = Diagnostic::error("cannot open `x.lbd`")
            .with_help("check the path")
            .to_plain_string(&map);
        assert_eq!(
            text,
            "error: cannot open `x.lbd`\n  = help: check the path\n"
        );
    }

    #[test]
    fn warnings_render_as_warnings() {
        let (map, span) = sample();
        let text = Diagnostic::warning("unused").at(span).to_plain_string(&map);
        assert!(text.contains("Warning: unused"), "{text}");
    }

    #[test]
    fn secondary_labels_are_rendered() {
        let (map, span) = sample();
        let first = Span::new(span.file, 0..1);
        let text = Diagnostic::error("duplicate")
            .at(span)
            .with_secondary(first, "first defined here")
            .to_plain_string(&map);
        assert!(text.contains("first defined here"), "{text}");
    }
}
