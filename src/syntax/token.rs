//! Token definitions and the `logos`-generated lexer.

use std::fmt;

use logos::Logos;

use crate::diagnostics::Diagnostic;
use crate::source::{FileId, Span};

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip(r"--[^\n]*", allow_greedy = true))]
pub enum TokenKind {
    #[token("use")]
    Use,
    #[token("let")]
    Let,
    #[token("in")]
    In,
    #[token("type")]
    Type,
    #[token("match")]
    Match,

    #[regex(r"[A-Za-z_][A-Za-z0-9_']*")]
    Ident,
    #[regex(r"-?[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?")]
    Number,
    #[regex(r#""([^"\\]|\\.)*""#)]
    String,

    #[token("\\")]
    #[token("λ")]
    Lambda,
    #[token(".")]
    Dot,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token("=")]
    Equals,
    #[token("->")]
    Arrow,
    #[token("<-")]
    LeftArrow,
    #[token("|")]
    Bar,
    #[token("<")]
    LAngle,
    #[token(">")]
    RAngle,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,

    /// Synthesized by [`lex`] at the end of input; never produced by logos.
    Eof,
}

impl TokenKind {
    /// How the token is named in "expected ..." messages.
    pub fn describe(self) -> &'static str {
        match self {
            TokenKind::Use => "`use`",
            TokenKind::Let => "`let`",
            TokenKind::In => "`in`",
            TokenKind::Type => "`type`",
            TokenKind::Match => "`match`",
            TokenKind::Ident => "an identifier",
            TokenKind::Number => "a number",
            TokenKind::String => "a string",
            TokenKind::Lambda => "`\\`",
            TokenKind::Dot => "`.`",
            TokenKind::Colon => "`:`",
            TokenKind::Comma => "`,`",
            TokenKind::Equals => "`=`",
            TokenKind::Arrow => "`->`",
            TokenKind::LeftArrow => "`<-`",
            TokenKind::Bar => "`|`",
            TokenKind::LAngle => "`<`",
            TokenKind::RAngle => "`>`",
            TokenKind::LParen => "`(`",
            TokenKind::RParen => "`)`",
            TokenKind::LBracket => "`[`",
            TokenKind::RBracket => "`]`",
            TokenKind::Eof => "end of input",
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.describe())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// Tokenizes a whole file. Invalid input produces a diagnostic and is skipped,
/// so the parser still sees the rest of the file. The result always ends with
/// an [`TokenKind::Eof`] token.
pub fn lex(file: FileId, text: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();
    let mut lexer = TokenKind::lexer(text);
    while let Some(result) = lexer.next() {
        let span = Span::new(file, lexer.span());
        match result {
            Ok(kind) => tokens.push(Token { kind, span }),
            Err(()) => errors.push(lex_error(lexer.slice(), lexer.remainder(), span)),
        }
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        span: Span::new(file, text.len()..text.len()),
    });
    (tokens, errors)
}

fn lex_error(slice: &str, remainder: &str, span: Span) -> Diagnostic {
    if slice.starts_with('"') {
        let end = span.end as usize + remainder.len();
        return Diagnostic::error("unterminated string literal")
            .at(Span::new(span.file, span.start as usize..end))
            .with_label("this string is never closed")
            .with_help("add a closing `\"`");
    }
    let shown = slice
        .chars()
        .next()
        .map_or_else(String::new, |c| c.escape_debug().to_string());
    let diagnostic = Diagnostic::error(format!("unexpected character `{shown}`"))
        .at(span)
        .with_label("not valid here");
    match slice {
        ";" => diagnostic.with_help("statements are not terminated by `;`"),
        "-" => diagnostic.with_help("use `(sub a b)` or `(neg x)`; `-` only starts a negative number literal or `--` comment"),
        _ => diagnostic,
    }
}

/// Decodes the escapes in a string literal's source text (including quotes).
pub fn unescape(literal: &str) -> Result<String, String> {
    let inner = &literal[1..literal.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('0') => out.push('\0'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('u') => {
                let rest = chars.as_str();
                let decoded = rest
                    .strip_prefix('{')
                    .and_then(|r| r.split_once('}'))
                    .and_then(|(hex, _)| u32::from_str_radix(hex, 16).ok().map(|v| (hex.len(), v)))
                    .and_then(|(len, v)| char::from_u32(v).map(|c| (len, c)));
                match decoded {
                    Some((len, c)) => {
                        out.push(c);
                        chars = rest[len + 2..].chars();
                    }
                    None => return Err("invalid unicode escape, expected `\\u{XXXX}`".into()),
                }
            }
            Some(other) => return Err(format!("unknown escape sequence `\\{other}`")),
            None => return Err("dangling `\\` at end of string".into()),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceMap;

    fn kinds(text: &str) -> Vec<TokenKind> {
        let mut map = SourceMap::new();
        let file = map.add_virtual("t", text);
        let (tokens, errors) = lex(file, text);
        assert!(errors.is_empty(), "{errors:?}");
        tokens.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn lexes_definition() {
        use TokenKind::*;
        assert_eq!(
            kinds(r"id : a -> a = \x. x"),
            vec![
                Ident, Colon, Ident, Arrow, Ident, Equals, Lambda, Ident, Dot, Ident, Eof
            ]
        );
    }

    #[test]
    fn keywords_are_not_identifiers_but_prefixes_are() {
        use TokenKind::*;
        assert_eq!(
            kinds("use let in type match user letter inner types matches"),
            vec![
                Use, Let, In, Type, Match, Ident, Ident, Ident, Ident, Ident, Eof
            ]
        );
    }

    #[test]
    fn numbers_including_negative_and_exponent() {
        use TokenKind::*;
        assert_eq!(
            kinds("1 -2 3.25 -0.5 1e3 2.5E-2"),
            vec![Number; 6].into_iter().chain([Eof]).collect::<Vec<_>>()
        );
    }

    #[test]
    fn comments_arrows_and_negative_numbers_are_distinguished() {
        use TokenKind::*;
        assert_eq!(
            kinds("a -> -1 -- comment -> 2\nb"),
            vec![Ident, Arrow, Number, Ident, Eof]
        );
    }

    #[test]
    fn lists_and_unicode_lambda() {
        use TokenKind::*;
        assert_eq!(
            kinds("[1, 2] λx. x"),
            vec![
                LBracket, Number, Comma, Number, RBracket, Lambda, Ident, Dot, Ident, Eof
            ]
        );
    }

    #[test]
    fn arrows() {
        use TokenKind::*;
        assert_eq!(
            kinds("let x <- a in b -> c"),
            vec![Let, Ident, LeftArrow, Ident, In, Ident, Arrow, Ident, Eof]
        );
    }

    #[test]
    fn match_arms() {
        use TokenKind::*;
        assert_eq!(
            kinds("match o | Some x -> x"),
            vec![Match, Ident, Bar, Ident, Ident, Arrow, Ident, Eof]
        );
    }

    #[test]
    fn strings_with_escaped_quotes() {
        use TokenKind::*;
        assert_eq!(kinds(r#""a \"b\" c" "x""#), vec![String, String, Eof]);
    }

    #[test]
    fn unexpected_character_reports_and_continues() {
        let mut map = SourceMap::new();
        let text = "a = 1;";
        let file = map.add_virtual("t", text);
        let (tokens, errors) = lex(file, text);
        assert_eq!(tokens.len(), 4);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "unexpected character `;`");
        assert_eq!(errors[0].span.unwrap().range(), 5..6);
    }

    #[test]
    fn unterminated_string_is_reported() {
        let mut map = SourceMap::new();
        let text = "x = \"abc";
        let file = map.add_virtual("t", text);
        let (_, errors) = lex(file, text);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "unterminated string literal");
        assert_eq!(errors[0].span.unwrap().range(), 4..8);
    }

    #[test]
    fn eof_span_is_at_end() {
        let mut map = SourceMap::new();
        let file = map.add_virtual("t", "ab");
        let (tokens, _) = lex(file, "ab");
        assert_eq!(tokens.last().unwrap().span.range(), 2..2);
    }

    #[test]
    fn unescape_handles_all_escapes() {
        assert_eq!(unescape(r#""a\nb\t\\\"\u{3bb}""#).unwrap(), "a\nb\t\\\"λ");
        assert_eq!(unescape(r#""plain""#).unwrap(), "plain");
        assert!(unescape(r#""bad \q""#).is_err());
        assert!(unescape(r#""bad \u{zz}""#).is_err());
    }
}
