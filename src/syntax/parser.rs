//! Hand-written recursive-descent parser with error recovery.
//!
//! Grammar (EBNF):
//!
//! ```text
//! module  = item* ;
//! item    = "use" STRING
//!         | IDENT [":" type] "=" expr
//!         | expr ;
//! expr    = "\" IDENT [":" type] "." expr
//!         | "let" IDENT [":" type] ("=" | "<-") expr "in" expr
//!         | NUMBER | STRING | IDENT
//!         | "(" ")"                       (* unit *)
//!         | "(" expr expr* ")"            (* grouping or application *)
//!         | "[" [expr ("," expr)* [","]] "]" ;
//! type    = btype ["->" type] ;
//! btype   = UPPER ["<" type ("," type)* ">"] | LOWER | "(" ")" | "(" type ")" ;
//! ```
//!
//! On a syntax error the parser records a diagnostic and skips to the next
//! line that starts a top-level item, so one run reports several errors.
//!
//! Every later stage recurses over the tree, so the parser rejects
//! expressions and types nested deeper than [`MAX_NESTING`] instead of letting
//! a later stage overflow the native stack.

use std::rc::Rc;

use crate::diagnostics::Diagnostic;
use crate::source::{FileId, Span};

use super::ast::{Def, Expr, ExprKind, Ident, Item, Module, TypeExpr, TypeExprKind};
use super::token::{Token, TokenKind, lex, unescape};

/// Marker for "a diagnostic was already recorded; unwind to recovery".
struct Failed;

type PResult<T> = Result<T, Failed>;

/// How deeply expressions, and the types written inside them, may nest.
pub const MAX_NESTING: u32 = 10_000;

/// Parses a whole file.
pub fn parse_module(file: FileId, text: &str) -> (Module, Vec<Diagnostic>) {
    let (tokens, mut diagnostics) = lex(file, text);
    let mut parser = Parser {
        text,
        tokens,
        pos: 0,
        depth: 0,
        diagnostics: Vec::new(),
    };
    let module = parser.module();
    diagnostics.append(&mut parser.diagnostics);
    (module, diagnostics)
}

/// Parses a single type (used for builtin signatures and the REPL).
pub fn parse_type(file: FileId, text: &str) -> Result<TypeExpr, Vec<Diagnostic>> {
    let (tokens, mut diagnostics) = lex(file, text);
    let mut parser = Parser {
        text,
        tokens,
        pos: 0,
        depth: 0,
        diagnostics: Vec::new(),
    };
    let result = parser
        .ty()
        .and_then(|t| parser.expect(TokenKind::Eof).map(|_| t));
    diagnostics.append(&mut parser.diagnostics);
    match result {
        Ok(t) if diagnostics.is_empty() => Ok(t),
        _ => Err(diagnostics),
    }
}

struct Parser<'a> {
    text: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    /// How many expressions and types enclose the current position.
    depth: u32,
    diagnostics: Vec<Diagnostic>,
}

impl Parser<'_> {
    fn peek(&self) -> Token {
        self.tokens[self.pos]
    }

    fn peek_kind(&self) -> TokenKind {
        self.peek().kind
    }

    fn nth_kind(&self, n: usize) -> TokenKind {
        self.tokens
            .get(self.pos + n)
            .map_or(TokenKind::Eof, |t| t.kind)
    }

    fn bump(&mut self) -> Token {
        let token = self.peek();
        if token.kind != TokenKind::Eof {
            self.pos += 1;
        }
        token
    }

    fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        (self.peek_kind() == kind).then(|| self.bump())
    }

    fn slice(&self, token: Token) -> &str {
        &self.text[token.span.range()]
    }

    fn expect(&mut self, kind: TokenKind) -> PResult<Token> {
        if let Some(token) = self.eat(kind) {
            return Ok(token);
        }
        let found = self.peek();
        Err(self.error(
            Diagnostic::error(format!("expected {}, found {}", kind, found.kind))
                .at(found.span)
                .with_label(format!("expected {kind}")),
        ))
    }

    /// Runs `parse` one nesting level deeper, failing past [`MAX_NESTING`].
    fn nested<T>(&mut self, what: &str, parse: impl FnOnce(&mut Self) -> PResult<T>) -> PResult<T> {
        if self.depth >= MAX_NESTING {
            let found = self.peek();
            return Err(self.error(
                Diagnostic::error(format!(
                    "{what} nested too deeply (the limit is {MAX_NESTING} levels)"
                ))
                .at(found.span)
                .with_label("nested too deeply here"),
            ));
        }
        self.depth += 1;
        let result = parse(self);
        self.depth -= 1;
        result
    }

    fn error(&mut self, diagnostic: Diagnostic) -> Failed {
        self.diagnostics.push(diagnostic);
        Failed
    }

    fn ident(&mut self) -> PResult<Ident> {
        let token = self.expect(TokenKind::Ident)?;
        Ok(Ident {
            name: Rc::from(self.slice(token)),
            span: token.span,
        })
    }

    fn module(&mut self) -> Module {
        let mut items = Vec::new();
        while self.peek_kind() != TokenKind::Eof {
            match self.item() {
                Ok(item) => items.push(item),
                Err(Failed) => self.recover(),
            }
        }
        Module { items }
    }

    /// Skips to the next token that begins a line and can start an item.
    fn recover(&mut self) {
        let start = self.pos;
        loop {
            let token = self.peek();
            if token.kind == TokenKind::Eof {
                return;
            }
            let at_line_start = token.span.start == 0
                || self.text.as_bytes()[token.span.start as usize - 1] == b'\n';
            let starts_item = matches!(
                token.kind,
                TokenKind::Ident | TokenKind::Use | TokenKind::LParen
            );
            if self.pos > start && at_line_start && starts_item {
                return;
            }
            self.bump();
        }
    }

    fn item(&mut self) -> PResult<Item> {
        match self.peek_kind() {
            TokenKind::Use => {
                let keyword = self.bump();
                let token = self.expect(TokenKind::String)?;
                let path = self.string_value(token)?;
                Ok(Item::Use {
                    path,
                    span: keyword.span.to(token.span),
                })
            }
            TokenKind::Ident
                if matches!(self.nth_kind(1), TokenKind::Colon | TokenKind::Equals) =>
            {
                let name = self.ident()?;
                let ann = match self.eat(TokenKind::Colon) {
                    Some(_) => Some(self.ty()?),
                    None => None,
                };
                self.expect(TokenKind::Equals)?;
                let value = self.expr()?;
                let span = name.span.to(value.span);
                Ok(Item::Def(Def {
                    name,
                    ann,
                    value,
                    span,
                }))
            }
            _ => Ok(Item::Expr(self.expr()?)),
        }
    }

    fn string_value(&mut self, token: Token) -> PResult<String> {
        unescape(self.slice(token)).map_err(|message| {
            self.error(
                Diagnostic::error(message)
                    .at(token.span)
                    .with_label("in this string literal"),
            )
        })
    }

    fn expr(&mut self) -> PResult<Expr> {
        self.nested("expression", Self::expr_inner)
    }

    fn expr_inner(&mut self) -> PResult<Expr> {
        let token = self.peek();
        let simple = |kind| {
            Ok(Expr {
                kind,
                span: token.span,
            })
        };
        match token.kind {
            TokenKind::Number => {
                self.bump();
                let text = self.slice(token);
                let value: f64 = text.parse().expect("the lexer only accepts valid numbers");
                simple(ExprKind::Number(value))
            }
            TokenKind::String => {
                self.bump();
                let value = self.string_value(token)?;
                simple(ExprKind::String(Rc::from(value)))
            }
            TokenKind::Ident => {
                let ident = self.ident()?;
                simple(ExprKind::Var(ident))
            }
            TokenKind::Lambda => self.lambda(),
            TokenKind::Let => self.let_expr(),
            TokenKind::LParen => self.paren(),
            TokenKind::LBracket => self.list(),
            _ => Err(self.error(
                Diagnostic::error(format!("expected an expression, found {}", token.kind))
                    .at(token.span)
                    .with_label("expected an expression"),
            )),
        }
    }

    fn lambda(&mut self) -> PResult<Expr> {
        let start = self.bump().span;
        let param = self.ident()?;
        let ann = match self.eat(TokenKind::Colon) {
            Some(_) => Some(self.ty()?),
            None => None,
        };
        self.expect(TokenKind::Dot)?;
        let body = self.expr()?;
        Ok(Expr {
            span: start.to(body.span),
            kind: ExprKind::Lambda {
                param,
                ann,
                body: Box::new(body),
            },
        })
    }

    fn let_expr(&mut self) -> PResult<Expr> {
        let start = self.bump().span;
        let name = self.ident()?;
        let ann = match self.eat(TokenKind::Colon) {
            Some(_) => Some(self.ty()?),
            None => None,
        };
        let binds = match self.eat(TokenKind::LeftArrow) {
            Some(_) => true,
            None if self.peek_kind() == TokenKind::Equals => {
                self.bump();
                false
            }
            None => {
                let found = self.peek();
                return Err(self.error(
                    Diagnostic::error(format!("expected `=` or `<-`, found {}", found.kind))
                        .at(found.span)
                        .with_label("expected `=` or `<-`")
                        .with_help("`let x = value in ...` names a value; `let x <- action in ...` performs an action and names its result"),
                ));
            }
        };
        let value = Box::new(self.expr()?);
        self.expect(TokenKind::In)?;
        let body = Box::new(self.expr()?);
        let span = start.to(body.span);
        let kind = if binds {
            ExprKind::Bind {
                name,
                ann,
                action: value,
                body,
            }
        } else {
            ExprKind::Let {
                name,
                ann,
                value,
                body,
            }
        };
        Ok(Expr { kind, span })
    }

    fn paren(&mut self) -> PResult<Expr> {
        let open = self.bump().span;
        if let Some(close) = self.eat(TokenKind::RParen) {
            return Ok(Expr {
                kind: ExprKind::Unit,
                span: open.to(close.span),
            });
        }
        let func = self.expr()?;
        let mut args = Vec::new();
        while !matches!(self.peek_kind(), TokenKind::RParen | TokenKind::Eof) {
            args.push(self.expr()?);
        }
        let close = self.expect_closing(TokenKind::RParen, open)?;
        let span = open.to(close.span);
        if args.is_empty() {
            // Plain grouping: `(e)` is `e`.
            return Ok(Expr { span, ..func });
        }
        Ok(Expr {
            kind: ExprKind::App {
                func: Box::new(func),
                args,
            },
            span,
        })
    }

    fn list(&mut self) -> PResult<Expr> {
        let open = self.bump().span;
        let mut items = Vec::new();
        while !matches!(self.peek_kind(), TokenKind::RBracket | TokenKind::Eof) {
            items.push(self.expr()?);
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        let close = self.expect_closing(TokenKind::RBracket, open)?;
        Ok(Expr {
            kind: ExprKind::List(items),
            span: open.to(close.span),
        })
    }

    /// Like [`Self::expect`], but points at the unmatched opening delimiter.
    fn expect_closing(&mut self, kind: TokenKind, open: Span) -> PResult<Token> {
        if let Some(token) = self.eat(kind) {
            return Ok(token);
        }
        let found = self.peek();
        Err(self.error(
            Diagnostic::error(format!("expected {}, found {}", kind, found.kind))
                .at(found.span)
                .with_label(format!("expected {kind}"))
                .with_secondary(open, "unclosed delimiter opened here"),
        ))
    }

    fn ty(&mut self) -> PResult<TypeExpr> {
        self.nested("type", Self::ty_inner)
    }

    fn ty_inner(&mut self) -> PResult<TypeExpr> {
        let from = self.btype()?;
        if self.eat(TokenKind::Arrow).is_none() {
            return Ok(from);
        }
        let to = self.ty()?;
        Ok(TypeExpr {
            span: from.span.to(to.span),
            kind: TypeExprKind::Fun(Box::new(from), Box::new(to)),
        })
    }

    fn btype(&mut self) -> PResult<TypeExpr> {
        if let Some(open) = self.eat(TokenKind::LParen) {
            if let Some(close) = self.eat(TokenKind::RParen) {
                return Ok(TypeExpr {
                    kind: TypeExprKind::Unit,
                    span: open.span.to(close.span),
                });
            }
            let inner = self.ty()?;
            let close = self.expect_closing(TokenKind::RParen, open.span)?;
            return Ok(TypeExpr {
                span: open.span.to(close.span),
                ..inner
            });
        }
        if self.peek_kind() != TokenKind::Ident {
            let found = self.peek();
            return Err(self.error(
                Diagnostic::error(format!("expected a type, found {}", found.kind))
                    .at(found.span)
                    .with_label("expected a type"),
            ));
        }
        let name = self.ident()?;
        if name
            .name
            .starts_with(|c: char| c.is_lowercase() || c == '_')
        {
            let span = name.span;
            return Ok(TypeExpr {
                kind: TypeExprKind::Var(name),
                span,
            });
        }
        let mut args = Vec::new();
        let mut span = name.span;
        if let Some(open) = self.eat(TokenKind::LAngle) {
            loop {
                args.push(self.ty()?);
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            span = span.to(self.expect_closing(TokenKind::RAngle, open.span)?.span);
        }
        Ok(TypeExpr {
            kind: TypeExprKind::Con { name, args },
            span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceMap;

    fn parse(text: &str) -> (Module, Vec<Diagnostic>) {
        let mut map = SourceMap::new();
        let file = map.add_virtual("t", text);
        parse_module(file, text)
    }

    fn parse_ok(text: &str) -> Module {
        let (module, diagnostics) = parse(text);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        module
    }

    fn roundtrip(text: &str) -> String {
        parse_ok(text).to_string()
    }

    #[test]
    fn definitions_with_and_without_annotations() {
        assert_eq!(roundtrip("x : Number = 1"), "x : Number = 1\n");
        assert_eq!(roundtrip("x = 1"), "x = 1\n");
        assert_eq!(
            roundtrip(r"apply : (a -> b) -> a -> b = \f. \x. (f x)"),
            "apply : (a -> b) -> a -> b = (\\f. (\\x. (f x)))\n"
        );
    }

    #[test]
    fn annotated_lambda_parameters() {
        assert_eq!(
            roundtrip(r"f = \x: List<Number>. x"),
            "f = (\\x: List<Number>. x)\n"
        );
    }

    #[test]
    fn applications_of_any_expression() {
        assert_eq!(roundtrip(r"((\x. x) 1)"), "((\\x. x) 1)\n");
        assert_eq!(roundtrip("((f 1) 2)"), "((f 1) 2)\n");
    }

    #[test]
    fn grouping_unit_and_lists() {
        assert_eq!(roundtrip("(x)"), "x\n");
        assert_eq!(roundtrip("()"), "()\n");
        assert_eq!(roundtrip("[]"), "[]\n");
        assert_eq!(roundtrip("[1, (f 2), [3],]"), "[1, (f 2), [3]]\n");
    }

    #[test]
    fn let_expressions() {
        assert_eq!(
            roundtrip("x = let y: Number = 2 in (add y y)"),
            "x = (let y: Number = 2 in (add y y))\n"
        );
    }

    #[test]
    fn let_bindings_of_action_results() {
        assert_eq!(
            roundtrip(r"x = let s <- (readFile f) in let n: Number <- (pure 1) in (println s n)"),
            "x = (let s <- (readFile f) in (let n: Number <- (pure 1) in (println s n)))\n"
        );
        let (_, diagnostics) = parse("x = let s 1 in s");
        assert_eq!(
            diagnostics[0].message,
            "expected `=` or `<-`, found a number"
        );
    }

    #[test]
    fn use_items_and_string_escapes() {
        let module = parse_ok("use \"std.lbd\"\n(print \"a\\tb\")");
        assert!(matches!(&module.items[0], Item::Use { path, .. } if path == "std.lbd"));
        assert_eq!(module.to_string(), "use \"std.lbd\"\n(print \"a\\tb\")\n");
    }

    #[test]
    fn function_types_are_right_associative() {
        let mut map = SourceMap::new();
        let file = map.add_virtual("t", "");
        let ty = parse_type(file, "(a -> b) -> List<a> -> List<b>").unwrap();
        assert_eq!(ty.to_string(), "(a -> b) -> List<a> -> List<b>");
        let TypeExprKind::Fun(_, rest) = ty.kind else {
            panic!()
        };
        assert!(matches!(rest.kind, TypeExprKind::Fun(..)));
        assert_eq!(parse_type(file, "()").unwrap().kind, TypeExprKind::Unit);
    }

    #[test]
    fn bad_type_is_an_error() {
        let mut map = SourceMap::new();
        let file = map.add_virtual("t", "");
        assert!(parse_type(file, "List<").is_err());
        assert!(parse_type(file, "a b").is_err());
    }

    #[test]
    fn spans_cover_whole_application() {
        let module = parse_ok("  (add 1 2)");
        let Item::Expr(expr) = &module.items[0] else {
            panic!()
        };
        assert_eq!(expr.span.range(), 2..11);
    }

    #[test]
    fn missing_close_paren_points_at_opener() {
        let (_, diagnostics) = parse("(add 1 2");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "expected `)`, found end of input");
        assert_eq!(diagnostics[0].secondary[0].0.range(), 0..1);
    }

    #[test]
    fn recovers_and_reports_multiple_errors() {
        let (module, diagnostics) = parse("a = \nb = 2\nc : = 3\nd = 4\n");
        // `a = b` parses, then `= 2` is not an expression, and `c : =` fails at `=`.
        assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
        let names: Vec<_> = module
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Def(d) => Some(d.name.name.to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(names, ["a", "d"]);
    }

    #[test]
    fn lexer_errors_are_included() {
        let (_, diagnostics) = parse("a : Number = 1;");
        assert_eq!(diagnostics[0].message, "unexpected character `;`");
    }

    /// Runs `f` on a thread with a stack like the one `lbd` gives the front end.
    fn with_big_stack(f: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(256 * 1024 * 1024)
            .spawn(f)
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn deep_nesting_is_an_error_not_a_stack_overflow() {
        with_big_stack(|| {
            let deep = 100_000;
            for (text, message) in [
                (
                    format!("{}1{}", "(".repeat(deep), ")".repeat(deep)),
                    "expression nested too deeply (the limit is 10000 levels)",
                ),
                (
                    format!("{}{}", "[".repeat(deep), "]".repeat(deep)),
                    "expression nested too deeply (the limit is 10000 levels)",
                ),
                (
                    format!("f = {}x", r"\x. ".repeat(deep)),
                    "expression nested too deeply (the limit is 10000 levels)",
                ),
                (
                    format!("f : {}Number = 1", "Number -> ".repeat(deep)),
                    "type nested too deeply (the limit is 10000 levels)",
                ),
            ] {
                let (_, diagnostics) = parse(&text);
                assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
                assert_eq!(diagnostics[0].message, message);
            }
        });
    }

    #[test]
    fn nesting_up_to_the_limit_is_accepted() {
        with_big_stack(|| {
            // The number inside the lists is one more level: exactly the limit.
            let depth = MAX_NESTING as usize - 1;
            let text = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
            parse_ok(&text);
            let text = format!("{}1{}", "[".repeat(depth + 1), "]".repeat(depth + 1));
            assert_eq!(parse(&text).1.len(), 1);
        });
    }

    #[test]
    fn stray_token_is_reported() {
        let (_, diagnostics) = parse(")");
        assert_eq!(diagnostics[0].message, "expected an expression, found `)`");
    }
}
