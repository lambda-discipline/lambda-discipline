//! Shared helpers for the fuzz targets.

use lambda_discipline::Session;
use lambda_discipline::diagnostics::Diagnostic;
use lambda_discipline::loader::{MemoryFiles, load_text_from};
use lambda_discipline::source::{SourceMap, Span};
use lambda_discipline::types::TopLevel;

/// The stack the `lbd` binary runs the compiler on (see `cli.rs`), so inputs
/// nested up to the parser's limit behave as they do for users.
const STACK_SIZE: usize = 256 * 1024 * 1024;

/// Runs `f` on a thread with the compiler's stack size, re-raising its panic.
pub fn on_big_stack(f: impl FnOnce() + Send + 'static) {
    let result = std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(f)
        .expect("spawning the fuzz thread")
        .join();
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

/// Asserts that `span` lies inside its file, on character boundaries.
pub fn assert_span_valid(sources: &SourceMap, span: Span) {
    let text = sources.get(span.file).text();
    let range = span.range();
    assert!(range.start <= range.end, "inverted span {range:?}");
    assert!(
        range.end <= text.len(),
        "span {range:?} past the end ({})",
        text.len()
    );
    assert!(
        text.is_char_boundary(range.start) && text.is_char_boundary(range.end),
        "span {range:?} splits a character"
    );
}

/// Asserts that every location a diagnostic points at is valid.
pub fn assert_diagnostics_valid(sources: &SourceMap, diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        let secondary = diagnostic.secondary.iter().map(|(span, _)| *span);
        for span in diagnostic.span.into_iter().chain(secondary) {
            assert_span_valid(sources, span);
        }
    }
}

/// Runs the whole compiler except evaluation on `text`, with `use` unable to
/// read any file, then renders every diagnostic it reports.
pub fn check(text: &str) {
    let mut session = Session::new(Box::new(std::io::sink()));
    let loaded = load_text_from(&mut session.sources, &MemoryFiles::new(), "<fuzz>", text);
    let mut diagnostics = match session.compile(loaded, TopLevel::Any) {
        Ok(_) => Vec::new(),
        Err(errors) => errors,
    };
    diagnostics.extend(session.take_warnings());
    assert_diagnostics_valid(&session.sources, &diagnostics);
    let mut rendered = Vec::new();
    session.render(&diagnostics, &mut rendered, false);
}
