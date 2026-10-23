#![no_main]

use lambda_discipline::source::SourceMap;
use lambda_discipline::syntax::token::{TokenKind, lex};
use lbd_fuzz::{assert_diagnostics_valid, assert_span_valid};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let mut sources = SourceMap::new();
    let file = sources.add_virtual("<fuzz>", text);
    let (tokens, diagnostics) = lex(file, text);
    let (last, rest) = tokens.split_last().expect("lex always ends with Eof");
    assert_eq!(last.kind, TokenKind::Eof);
    assert_eq!(last.span.range(), text.len()..text.len());
    let mut end = 0;
    for token in rest {
        assert_ne!(token.kind, TokenKind::Eof);
        assert_span_valid(&sources, token.span);
        assert!(token.span.start < token.span.end, "empty token {token:?}");
        assert!(
            token.span.start as usize >= end,
            "overlapping token {token:?}"
        );
        end = token.span.end as usize;
    }
    assert_diagnostics_valid(&sources, &diagnostics);
});
