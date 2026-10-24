//! Compiler and runtime diagnostics, rendered with `ariadne`.
//!
//! Every stage reports problems as [`Diagnostic`] values instead of printing
//! or exiting, so the CLI, REPL and tests decide what to do with them.

use std::collections::HashMap;
use std::fmt;
use std::io::{self, Write};
use std::ops::Range;
use std::rc::Rc;

use ariadne::{Color, Config, IndexType, Label, Report, ReportKind, Source};

use crate::index::to_u32;
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
        let mut out = io::BufWriter::new(out);
        self.render_with(&mut Cache::new(sources), &mut out, color)?;
        out.flush()
    }

    /// [`render`](Self::render) with ariadne sources shared between diagnostics.
    fn render_with(
        &self,
        cache: &mut Cache<'_>,
        out: &mut dyn Write,
        color: bool,
    ) -> io::Result<()> {
        let Some(span) = self.span else {
            return self.render_plain(out);
        };
        let spans: Vec<Span> = std::iter::once(span)
            .chain(self.secondary.iter().map(|(span, _)| *span))
            .collect();
        let Some(windows) = Windows::new(cache.map, &spans) else {
            return self.report(&spans, color).write(cache, out);
        };
        let shown: Vec<Span> = spans.iter().map(|&span| windows.map(span)).collect();
        let mut buffer = Vec::new();
        self.report(&shown, color)
            .write(windows.cache(cache.map), &mut buffer)?;
        let text = String::from_utf8_lossy(&buffer);
        let text = windows.fix_locations(cache.map, &self.message, &spans, &shown, &text);
        out.write_all(text.as_bytes())
    }

    /// Builds the ariadne report for the primary span `spans[0]` and the
    /// secondary spans after it.
    fn report(&self, spans: &[Span], color: bool) -> Report<'static, AriadneSpan> {
        let span = spans[0];
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
        for ((_, text), span) in self.secondary.iter().zip(&spans[1..]) {
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
        report.finish()
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
    let mut out = io::BufWriter::new(out);
    let mut cache = Cache::new(sources);
    for diagnostic in diagnostics {
        diagnostic.render_with(&mut cache, &mut out, color)?;
    }
    out.flush()
}

/// Lines longer than this many bytes are shown as windows around the labels
/// on them, so a diagnostic on a long line renders as fast as one on a short
/// line instead of printing the whole line.
const LONG_LINE: usize = 160;

/// Bytes of context kept on each side of a label's ends on a long line.
const CONTEXT: usize = 40;

/// Stands for text left out of a long line.
const ELLIPSIS: &str = "…";

/// Shortened copies of the files one diagnostic points into, used when one of
/// its labels is on a long line.
struct Windows {
    /// In the order the files first appear among the labels, which is the
    /// order ariadne shows them in.
    files: Vec<Window>,
}

/// The part of one file a diagnostic shows: the lines from its first label to
/// its last, with long lines cut down to context around the label ends.
struct Window {
    file: FileId,
    /// Lines before the first label are kept as empty lines, so line numbers
    /// stay the same.
    text: String,
    /// `(original offset, offset in text, length)` of each piece of the file
    /// copied into `text`, in order.
    pieces: Vec<(usize, usize, usize)>,
}

impl Windows {
    /// Returns `None` when no label is near a long line, so the diagnostic can
    /// be shown against the whole files.
    fn new(map: &SourceMap, spans: &[Span]) -> Option<Self> {
        let mut files: Vec<FileId> = Vec::new();
        for span in spans {
            if !files.contains(&span.file) {
                files.push(span.file);
            }
        }
        let points = |file: FileId| -> Vec<usize> {
            spans
                .iter()
                .filter(|span| span.file == file)
                .flat_map(|span| [span.start as usize, span.end as usize])
                .collect()
        };
        let lines = |file: FileId| {
            let source = map.get(file);
            let points = points(file);
            let first = source.line_index(points.iter().copied().min().unwrap_or(0));
            let last = source.line_index(points.iter().copied().max().unwrap_or(0));
            (first..=last).map(move |line| (line, source.line_range(line)))
        };
        let text_len = |file: FileId, range: &Range<usize>| {
            content_end(map.get(file).text(), range.clone()) - range.start
        };
        if !files
            .iter()
            .any(|&file| lines(file).any(|(_, range)| text_len(file, &range) > LONG_LINE))
        {
            return None;
        }
        let files = files
            .iter()
            .map(|&file| Window::new(file, map.get(file).text(), lines(file), &points(file)))
            .collect();
        Some(Self { files })
    }

    fn map(&self, span: Span) -> Span {
        let window = self.window(span.file);
        Span {
            file: span.file,
            start: to_u32(window.map(span.start as usize)),
            end: to_u32(window.map(span.end as usize)),
        }
    }

    fn window(&self, file: FileId) -> &Window {
        self.files
            .iter()
            .find(|window| window.file == file)
            .expect("every label's file has a window")
    }

    fn cache<'a>(&self, map: &'a SourceMap) -> Cache<'a> {
        Cache {
            map,
            sources: self
                .files
                .iter()
                .map(|window| (window.file, Source::from(Rc::from(window.text.as_str()))))
                .collect(),
        }
    }

    /// ariadne computes the column in each file's `name:line:column` heading
    /// from the shortened line; puts back the column in the whole line.
    ///
    /// `spans` and `shown` are the labels' spans in the files and in the
    /// windows. Like ariadne, the heading for a file points at its first label.
    fn fix_locations(
        &self,
        map: &SourceMap,
        message: &str,
        spans: &[Span],
        shown: &[Span],
        rendered: &str,
    ) -> String {
        let mut rendered = rendered.to_owned();
        // Headings follow the message, which could contain the same text.
        let after_message = rendered.find(message).map_or(0, |i| i + message.len());
        for window in &self.files {
            let Some(label) = spans.iter().position(|span| span.file == window.file) else {
                continue;
            };
            let source = map.get(window.file);
            let (line, column) = source.line_col(spans[label].start as usize);
            let offset = shown[label].start as usize;
            let line_start = window.text[..offset].rfind('\n').map_or(0, |i| i + 1);
            let wrong = window.text[line_start..offset].chars().count() + 1;
            if wrong == column {
                continue;
            }
            let name = source.name();
            let from = format!("{name}:{line}:{wrong}");
            if let Some(i) = rendered[after_message..].find(&from) {
                let at = after_message + i;
                rendered.replace_range(at..at + from.len(), &format!("{name}:{line}:{column}"));
            }
        }
        rendered
    }
}

impl Window {
    /// `lines` are the line numbers and byte ranges to show; `points` are the
    /// label ends in this file.
    fn new(
        file: FileId,
        text: &str,
        mut lines: impl Iterator<Item = (usize, Range<usize>)>,
        points: &[usize],
    ) -> Self {
        let mut window = Self {
            file,
            text: String::new(),
            pieces: Vec::new(),
        };
        let Some((first, range)) = lines.next() else {
            return window;
        };
        window.text = "\n".repeat(first);
        for range in std::iter::once(range).chain(lines.map(|(_, range)| range)) {
            let end = content_end(text, range.clone());
            if end - range.start <= LONG_LINE {
                window.keep(text, range);
                continue;
            }
            let mut kept: Vec<Range<usize>> = points
                .iter()
                .filter(|&&point| (range.start..=end).contains(&point))
                .map(|&point| {
                    let from = point.saturating_sub(CONTEXT).max(range.start);
                    let to = (point + CONTEXT).min(end);
                    floor_boundary(text, from)..ceil_boundary(text, to)
                })
                .collect();
            if kept.is_empty() {
                // A line inside a multi-line label: its start is enough.
                kept.push(range.start..ceil_boundary(text, range.start + 2 * CONTEXT));
            }
            kept.sort_by_key(|kept| kept.start);
            let mut at = range.start;
            for part in merge(kept) {
                if part.start > at {
                    window.text.push_str(ELLIPSIS);
                }
                at = part.end;
                window.keep(text, part);
            }
            if at < end {
                window.text.push_str(ELLIPSIS);
            }
            window.keep(text, end..range.end);
        }
        window
    }

    fn keep(&mut self, text: &str, range: Range<usize>) {
        self.pieces
            .push((range.start, self.text.len(), range.end - range.start));
        self.text.push_str(&text[range]);
    }

    /// Where an offset in the file is in the window. Every label end is in a
    /// kept piece; a piece's end counts as inside it.
    fn map(&self, offset: usize) -> usize {
        self.pieces
            .iter()
            .find(|(from, _, len)| (*from..=from + len).contains(&offset))
            .map(|(from, to, _)| to + (offset - from))
            .expect("every label end is kept")
    }
}

/// The end of a line's text, before its line break.
fn content_end(text: &str, range: Range<usize>) -> usize {
    let line = &text[range.clone()];
    let line = line.strip_suffix('\n').unwrap_or(line);
    let line = line.strip_suffix('\r').unwrap_or(line);
    range.start + line.len()
}

/// Joins overlapping ranges of a list sorted by start.
fn merge(ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in ranges {
        match merged.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    merged
}

fn floor_boundary(text: &str, mut i: usize) -> usize {
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn ceil_boundary(text: &str, mut i: usize) -> usize {
    while !text.is_char_boundary(i) {
        i += 1;
    }
    i
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

    /// `x = a + a + … + a + y` on one line of about 2,000 bytes.
    fn long_line() -> String {
        format!("x = {}y\n", "a + ".repeat(500))
    }

    fn at(map: &SourceMap, id: FileId, needle: &str) -> Span {
        let start = map.get(id).text().find(needle).unwrap();
        Span::new(id, start..start + needle.len())
    }

    #[test]
    fn long_lines_are_shown_around_their_labels() {
        let mut map = SourceMap::new();
        let id = map.add_virtual("long.lbd", long_line());
        let span = at(&map, id, "y");
        let text = Diagnostic::error("unknown name `y`")
            .at(span)
            .with_label("not defined")
            .to_plain_string(&map);
        assert!(
            text.contains(&format!("long.lbd:1:{}", span.start + 1)),
            "{text}"
        );
        assert!(text.contains("…a + a + "), "{text}");
        assert!(text.contains("not defined"), "{text}");
        assert!(!text.contains("x = "), "{text}");
        assert!(text.len() < 1000, "{text}");
    }

    #[test]
    fn labels_far_apart_on_a_long_line_get_their_own_windows() {
        let mut map = SourceMap::new();
        let id = map.add_virtual("long.lbd", long_line());
        let text = Diagnostic::error("duplicate")
            .at(at(&map, id, "y"))
            .with_secondary(at(&map, id, "x"), "first here")
            .to_plain_string(&map);
        assert!(text.contains("long.lbd:1:2005"), "{text}");
        assert!(text.contains("x = a + "), "{text}");
        assert!(text.contains("a…a + "), "{text}");
        assert!(text.contains("first here"), "{text}");
        assert!(text.len() < 1500, "{text}");
    }

    #[test]
    fn columns_count_the_characters_left_out() {
        let mut map = SourceMap::new();
        let id = map.add_virtual("wide.lbd", format!("{} z\n", "λ".repeat(300)));
        let text = Diagnostic::error("here")
            .at(at(&map, id, "z"))
            .to_plain_string(&map);
        assert!(text.contains("wide.lbd:1:302"), "{text}");
        assert!(text.contains("…λλ"), "{text}");
    }

    #[test]
    fn lines_keep_their_numbers() {
        let mut map = SourceMap::new();
        let id = map.add_virtual("long.lbd", format!("a\nb\n{}c\n", long_line()));
        let text = Diagnostic::error("here")
            .at(at(&map, id, "y"))
            .with_secondary(at(&map, id, "c"), "and here")
            .to_plain_string(&map);
        assert!(text.contains("long.lbd:3:2005"), "{text}");
        assert!(text.contains(" 3 │ …"), "{text}");
        assert!(text.contains(" 4 │ c"), "{text}");
    }

    #[test]
    fn labels_at_the_end_or_across_a_long_line() {
        let mut map = SourceMap::new();
        let text = long_line();
        let id = map.add_virtual("long.lbd", format!("{text}z"));
        let end = Span::new(id, text.len() + 1..text.len() + 1);
        let rendered = Diagnostic::error("unexpected end of input")
            .at(end)
            .to_plain_string(&map);
        assert!(rendered.contains("long.lbd:2:2"), "{rendered}");
        let across = Span::new(id, text.len() - 3..text.len() + 1);
        let rendered = Diagnostic::error("across").at(across).to_plain_string(&map);
        assert!(rendered.contains("long.lbd:1:2004"), "{rendered}");
        assert!(rendered.len() < 1000, "{rendered}");
    }

    #[test]
    fn many_warnings_on_one_long_line_take_space_linear_in_their_number() {
        let mut map = SourceMap::new();
        let source: String = (0..2000).map(|i| format!("let a{i} = 1 in ")).collect();
        let id = map.add_virtual("many.lbd", format!("x = {source}a0\n"));
        let warnings: Vec<Diagnostic> = (1..2000)
            .map(|i| {
                Diagnostic::warning(format!("unused variable `a{i}`"))
                    .at(at(&map, id, &format!("a{i} ")))
                    .with_label("never used")
            })
            .collect();
        let mut out = Vec::new();
        render_all(&warnings, &map, &mut out, false).unwrap();
        assert!(out.len() < warnings.len() * 1000, "{} bytes", out.len());
    }
}
