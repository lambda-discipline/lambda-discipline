//! Hand-written recursive-descent parser with error recovery.
//!
//! Grammar (EBNF):
//!
//! ```text
//! module  = item* ;
//! item    = "use" STRING
//!         | "type" UPPER ["<" LOWER ("," LOWER)* ">"] "=" ["|"] ctor ("|" ctor)*
//!         | IDENT [":" type] "=" expr
//!         | expr ;
//! ctor    = UPPER btype* ;                (* fields end at a line's first column *)
//! expr    = "\" IDENT [":" type] "." expr
//!         | "let" IDENT [":" type] ("=" | "<-") expr "in" expr
//!         | "match" expr ("|" pattern "->" expr)+
//!         | INT | FLOAT | STRING | IDENT
//!         | "(" ")"                       (* unit *)
//!         | "(" expr expr* ")"            (* grouping or application *)
//!         | "[" [expr ("," expr)* [","]] "]" ;
//! type    = btype ["->" type] ;
//! btype   = UPPER ["<" type ("," type)* ">"] | LOWER | "(" ")" | "(" type ")" ;
//! pattern = UPPER apat* | apat ;
//! apat    = "_" | LOWER | UPPER | INT | FLOAT | STRING | "(" ")" | "(" pattern ")" ;
//! ```
//!
//! A `match` arm's body extends as far as it can, like a `let` body, so a
//! `match` nested in the last arm of another takes the arms that follow it
//! unless it is parenthesized. A type declaration ends at the first token in
//! the first column of a line, which starts the next item.
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

use super::ast::{
    Arm, Constraint, CtorDecl, Def, Expr, ExprKind, Ident, Item, Module, Pattern, PatternKind,
    TypeDecl, TypeExpr, TypeExprKind, TypeScheme,
};
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

/// Parses a single type (used for the REPL).
pub fn parse_type(file: FileId, text: &str) -> Result<TypeExpr, Vec<Diagnostic>> {
    parse_whole(file, text, Parser::ty)
}

/// Parses a type with an optional context, `Eq<a> => a -> a -> Bool` (used
/// for builtin signatures).
pub fn parse_scheme(file: FileId, text: &str) -> Result<TypeScheme, Vec<Diagnostic>> {
    parse_whole(file, text, Parser::scheme)
}

fn parse_whole<'t, T>(
    file: FileId,
    text: &'t str,
    parse: impl FnOnce(&mut Parser<'t>) -> PResult<T>,
) -> Result<T, Vec<Diagnostic>> {
    let (tokens, mut diagnostics) = lex(file, text);
    let mut parser = Parser {
        text,
        tokens,
        pos: 0,
        depth: 0,
        diagnostics: Vec::new(),
    };
    let result = parse(&mut parser).and_then(|t| parser.expect(TokenKind::Eof).map(|_| t));
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

    /// Whether `token` is in the first column of its line.
    fn at_line_start(&self, token: Token) -> bool {
        token.span.start == 0 || self.text.as_bytes()[token.span.start as usize - 1] == b'\n'
    }

    /// Skips to the next token that begins a line and can start an item.
    fn recover(&mut self) {
        let start = self.pos;
        loop {
            let token = self.peek();
            if token.kind == TokenKind::Eof {
                return;
            }
            let starts_item = matches!(
                token.kind,
                TokenKind::Ident | TokenKind::Use | TokenKind::Type | TokenKind::LParen
            );
            if self.pos > start && self.at_line_start(token) && starts_item {
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
            TokenKind::Type => self.type_decl().map(Item::Type),
            TokenKind::Ident
                if matches!(self.nth_kind(1), TokenKind::Colon | TokenKind::Equals) =>
            {
                let name = self.ident()?;
                let ann = match self.eat(TokenKind::Colon) {
                    Some(_) => Some(self.scheme()?),
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

    fn int_value(&mut self, token: Token) -> PResult<i64> {
        self.slice(token).parse().map_err(|_| {
            self.error(
                Diagnostic::error("integer literal is too large for an `Int`")
                    .at(token.span)
                    .with_label("does not fit in 64 bits")
                    .with_help(format!(
                        "an `Int` is between {} and {}; write a `Float` such as `1e20` for larger numbers",
                        i64::MIN,
                        i64::MAX
                    )),
            )
        })
    }

    fn float_value(&self, token: Token) -> f64 {
        self.slice(token)
            .parse()
            .expect("the lexer only accepts valid floats")
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
            TokenKind::Int => {
                self.bump();
                simple(ExprKind::Int(self.int_value(token)?))
            }
            TokenKind::Float => {
                self.bump();
                simple(ExprKind::Float(self.float_value(token)))
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
            TokenKind::Match => self.match_expr(),
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
            Some(_) => Some(self.scheme()?),
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
            if let Some(first) = ann.as_ref().and_then(|ann| ann.context.first()) {
                return Err(self.error(
                    Diagnostic::error("the result of an action cannot have a context")
                        .at(first.span)
                        .with_label("not allowed in `let ... <-`")
                        .with_note("an action's result is a single value, so it cannot be generic"),
                ));
            }
            ExprKind::Bind {
                name,
                ann: ann.map(|ann| ann.ty),
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

    /// `type Name<a, b> = Con field ... | ...`
    fn type_decl(&mut self) -> PResult<TypeDecl> {
        let start = self.bump().span;
        let name = self.upper_ident("a type name")?;
        let mut params = Vec::new();
        if let Some(open) = self.eat(TokenKind::LAngle) {
            loop {
                let param = self.ident()?;
                if !is_lower(&param.name) {
                    return Err(self.error(
                        Diagnostic::error(format!(
                            "type parameter `{}` must start with a lowercase letter",
                            param.name
                        ))
                        .at(param.span)
                        .with_label("not a type variable"),
                    ));
                }
                params.push(param);
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            self.expect_closing(TokenKind::RAngle, open.span)?;
        }
        self.expect(TokenKind::Equals)?;
        self.eat(TokenKind::Bar);
        let mut constructors = vec![self.ctor_decl()?];
        while self.eat(TokenKind::Bar).is_some() {
            constructors.push(self.ctor_decl()?);
        }
        let span = start.to(constructors.last().expect("at least one").span);
        Ok(TypeDecl {
            name,
            params,
            constructors,
            span,
        })
    }

    /// A constructor and its field types. The fields end at a token in the
    /// first column of a line, which starts the next item.
    fn ctor_decl(&mut self) -> PResult<CtorDecl> {
        let name = self.upper_ident("a constructor name")?;
        let mut span = name.span;
        let mut fields = Vec::new();
        while matches!(self.peek_kind(), TokenKind::Ident | TokenKind::LParen)
            && !self.at_line_start(self.peek())
        {
            let field = self.nested("type", Self::btype)?;
            span = span.to(field.span);
            fields.push(field);
        }
        Ok(CtorDecl { name, fields, span })
    }

    /// An identifier that must start with an uppercase letter.
    fn upper_ident(&mut self, what: &str) -> PResult<Ident> {
        let found = self.peek();
        if found.kind == TokenKind::Ident && !is_lower(self.slice(found)) {
            return self.ident();
        }
        let message = if found.kind == TokenKind::Ident {
            format!("{what} must start with an uppercase letter")
        } else {
            format!("expected {what}, found {}", found.kind)
        };
        Err(self.error(
            Diagnostic::error(message)
                .at(found.span)
                .with_label(format!("expected {what}")),
        ))
    }

    /// `match scrutinee | pattern -> body | ...`
    fn match_expr(&mut self) -> PResult<Expr> {
        let start = self.bump().span;
        let scrutinee = self.expr()?;
        if self.peek_kind() != TokenKind::Bar {
            let found = self.peek();
            return Err(self.error(
                Diagnostic::error(format!("expected `|`, found {}", found.kind))
                    .at(found.span)
                    .with_label("expected `|`")
                    .with_help(
                        "each arm of a `match` starts with `|`: `match x | pattern -> result`",
                    ),
            ));
        }
        let mut arms = Vec::new();
        let mut end = start;
        while self.eat(TokenKind::Bar).is_some() {
            let pattern = self.pattern()?;
            self.expect(TokenKind::Arrow)?;
            let body = self.expr()?;
            end = body.span;
            arms.push(Arm { pattern, body });
        }
        Ok(Expr {
            kind: ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
            span: start.to(end),
        })
    }

    fn pattern(&mut self) -> PResult<Pattern> {
        self.nested("pattern", Self::pattern_inner)
    }

    /// A constructor applied to field patterns, or an atomic pattern.
    fn pattern_inner(&mut self) -> PResult<Pattern> {
        let token = self.peek();
        if token.kind != TokenKind::Ident || is_lower(self.slice(token)) {
            return self.atomic_pattern();
        }
        let name = self.ident()?;
        let mut span = name.span;
        let mut args = Vec::new();
        while matches!(
            self.peek_kind(),
            TokenKind::Ident
                | TokenKind::Int
                | TokenKind::Float
                | TokenKind::String
                | TokenKind::LParen
        ) {
            let arg = self.nested("pattern", Self::atomic_pattern)?;
            span = span.to(arg.span);
            args.push(arg);
        }
        Ok(Pattern {
            kind: PatternKind::Con { name, args },
            span,
        })
    }

    fn atomic_pattern(&mut self) -> PResult<Pattern> {
        let token = self.peek();
        let simple = |kind| {
            Ok(Pattern {
                kind,
                span: token.span,
            })
        };
        match token.kind {
            TokenKind::Ident => {
                let name = self.ident()?;
                if name.name.as_ref() == "_" {
                    simple(PatternKind::Wild)
                } else if is_lower(&name.name) {
                    simple(PatternKind::Var(name))
                } else {
                    simple(PatternKind::Con {
                        name,
                        args: Vec::new(),
                    })
                }
            }
            TokenKind::Int => {
                self.bump();
                simple(PatternKind::Int(self.int_value(token)?))
            }
            TokenKind::Float => {
                self.bump();
                simple(PatternKind::Float(self.float_value(token)))
            }
            TokenKind::String => {
                self.bump();
                let value = self.string_value(token)?;
                simple(PatternKind::String(Rc::from(value)))
            }
            TokenKind::LParen => {
                let open = self.bump().span;
                if let Some(close) = self.eat(TokenKind::RParen) {
                    return Ok(Pattern {
                        kind: PatternKind::Unit,
                        span: open.to(close.span),
                    });
                }
                let inner = self.pattern()?;
                let close = self.expect_closing(TokenKind::RParen, open)?;
                Ok(Pattern {
                    span: open.to(close.span),
                    ..inner
                })
            }
            _ => Err(self.error(
                Diagnostic::error(format!("expected a pattern, found {}", token.kind))
                    .at(token.span)
                    .with_label("expected a pattern"),
            )),
        }
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

    /// A type with an optional context: `Eq<a> => ...` or
    /// `(Eq<a>, Show<b>) => ...`.
    fn scheme(&mut self) -> PResult<TypeScheme> {
        let mut context = Vec::new();
        if self.at_context() {
            if let Some(open) = self.eat(TokenKind::LParen) {
                loop {
                    context.push(self.constraint()?);
                    if self.eat(TokenKind::Comma).is_none() {
                        break;
                    }
                }
                self.expect_closing(TokenKind::RParen, open.span)?;
            } else {
                context.push(self.constraint()?);
            }
            self.expect(TokenKind::FatArrow)?;
        }
        let ty = self.ty()?;
        Ok(TypeScheme { context, ty })
    }

    /// Whether a context starts here: a `=>` follows before anything that
    /// cannot be part of a type.
    fn at_context(&self) -> bool {
        self.tokens[self.pos..]
            .iter()
            .map(|token| token.kind)
            .find(|kind| {
                !matches!(
                    kind,
                    TokenKind::Ident
                        | TokenKind::LAngle
                        | TokenKind::RAngle
                        | TokenKind::Comma
                        | TokenKind::LParen
                        | TokenKind::RParen
                        | TokenKind::Arrow
                )
            })
            == Some(TokenKind::FatArrow)
    }

    /// `Eq<a>`
    fn constraint(&mut self) -> PResult<Constraint> {
        let class = self.ident()?;
        let Some(open) = self.eat(TokenKind::LAngle) else {
            let found = self.peek();
            return Err(self.error(
                Diagnostic::error(format!("expected `<`, found {}", found.kind))
                    .at(found.span)
                    .with_label("expected `<`")
                    .with_help(format!(
                        "a constraint is written like a type: `{}<a>`",
                        class.name
                    )),
            ));
        };
        let arg = self.ty()?;
        let close = self.expect_closing(TokenKind::RAngle, open.span)?;
        Ok(Constraint {
            span: class.span.to(close.span),
            class,
            arg,
        })
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
        if is_lower(&name.name) {
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

/// Whether a name is a variable (lowercase or starting with `_`) rather than
/// a constructor or type (uppercase).
pub fn is_lower(name: &str) -> bool {
    name.starts_with(|c: char| c.is_lowercase() || c == '_')
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
        assert_eq!(roundtrip("x : Int = 1"), "x : Int = 1\n");
        assert_eq!(roundtrip("x = 1"), "x = 1\n");
        assert_eq!(
            roundtrip(r"apply : (a -> b) -> a -> b = \f. \x. (f x)"),
            "apply : (a -> b) -> a -> b = (\\f. (\\x. (f x)))\n"
        );
    }

    #[test]
    fn annotations_with_contexts() {
        assert_eq!(
            roundtrip(r"member : Eq<a> => a -> List<a> -> Bool = \x. \xs. x"),
            "member : Eq<a> => a -> List<a> -> Bool = (\\x. (\\xs. x))\n"
        );
        assert_eq!(
            roundtrip("f : (Ord<k>, Show<v>) => k -> v -> String = g"),
            "f : (Ord<k>, Show<v>) => k -> v -> String = g\n"
        );
        // A parenthesized type is not mistaken for a context.
        assert_eq!(
            roundtrip("f : (List<a>) -> a = head"),
            "f : List<a> -> a = head\n"
        );
        assert_eq!(
            roundtrip(r"x = let f : Eq<a> => a -> Bool = \y. (eq y y) in f"),
            "x = (let f: Eq<a> => a -> Bool = (\\y. (eq y y)) in f)\n"
        );
    }

    #[test]
    fn malformed_contexts() {
        let (_, diagnostics) = parse("f : Eq a => a = x");
        assert_eq!(diagnostics[0].message, "expected `<`, found an identifier");
        assert_eq!(
            diagnostics[0].help.as_deref(),
            Some("a constraint is written like a type: `Eq<a>`")
        );
        let (_, diagnostics) = parse("x = let y : Show<a> => a <- (pure 1) in (pure y)");
        assert_eq!(
            diagnostics[0].message,
            "the result of an action cannot have a context"
        );
    }

    #[test]
    fn annotated_lambda_parameters() {
        assert_eq!(
            roundtrip(r"f = \x: List<Int>. x"),
            "f = (\\x: List<Int>. x)\n"
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
            roundtrip("x = let y: Int = 2 in (add y y)"),
            "x = (let y: Int = 2 in (add y y))\n"
        );
    }

    #[test]
    fn let_bindings_of_action_results() {
        assert_eq!(
            roundtrip(r"x = let s <- (readFile f) in let n: Int <- (pure 1) in (println s n)"),
            "x = (let s <- (readFile f) in (let n: Int <- (pure 1) in (println s n)))\n"
        );
        let (_, diagnostics) = parse("x = let s 1 in s");
        assert_eq!(
            diagnostics[0].message,
            "expected `=` or `<-`, found an integer"
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
        let (_, diagnostics) = parse("a : Int = 1;");
        assert_eq!(diagnostics[0].message, "unexpected character `;`");
    }

    #[test]
    fn integer_and_float_literals() {
        assert_eq!(
            roundtrip("x = [1, -2, 1.0, 2.50, 1e3, -9223372036854775808]"),
            "x = [1, -2, 1.0, 2.5, 1000.0, -9223372036854775808]\n"
        );
        assert_eq!(
            roundtrip("f = \\n. match n | 0 -> 0.5 | _ -> 1e-7"),
            "f = (\\n. (match n | 0 -> 0.5 | _ -> 1e-7))\n"
        );
        let (_, diagnostics) = parse("x = 9223372036854775808");
        assert_eq!(
            diagnostics[0].message,
            "integer literal is too large for an `Int`"
        );
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
                    format!("f : {}Int = 1", "Int -> ".repeat(deep)),
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
    fn type_declarations() {
        assert_eq!(
            roundtrip("type Shape = Circle Int | Rect Int Int"),
            "type Shape = Circle Int | Rect Int Int\n"
        );
        assert_eq!(
            roundtrip(
                "type Tree<a> =\n    | Leaf\n    | Node Tree<a> a Tree<a>\n    | Fn (a -> a)\nx = 1"
            ),
            "type Tree<a> = Leaf | Node Tree<a> a Tree<a> | Fn (a -> a)\nx = 1\n"
        );
    }

    #[test]
    fn type_declarations_end_at_the_next_item() {
        let module = parse_ok("type T = A Int\n(println 1)\ntype U = B\nb : U = B");
        assert_eq!(module.items.len(), 4);
        assert_eq!(
            module.to_string(),
            "type T = A Int\n(println 1)\ntype U = B\nb : U = B\n"
        );
    }

    #[test]
    fn type_declaration_errors() {
        let (_, diagnostics) = parse("type shape = Circle");
        assert_eq!(
            diagnostics[0].message,
            "a type name must start with an uppercase letter"
        );
        let (_, diagnostics) = parse("type Shape = circle");
        assert_eq!(
            diagnostics[0].message,
            "a constructor name must start with an uppercase letter"
        );
        let (_, diagnostics) = parse("type Box<A> = Box A");
        assert_eq!(
            diagnostics[0].message,
            "type parameter `A` must start with a lowercase letter"
        );
    }

    #[test]
    fn match_expressions() {
        assert_eq!(
            roundtrip("f = \\s. match s\n    | Circle r -> (mul r r)\n    | Rect w h -> (mul w h)"),
            "f = (\\s. (match s | Circle r -> (mul r r) | Rect w h -> (mul w h)))\n"
        );
        assert_eq!(
            roundtrip(r#"x = match y | Some (Pair _ 0) -> 1 | None -> "a" | () -> -2"#),
            "x = (match y | Some (Pair _ 0) -> 1 | None -> \"a\" | () -> -2)\n"
        );
    }

    #[test]
    fn nested_match_takes_the_following_arms() {
        let module = parse_ok("x = match a | A -> match b | C -> 1 | D -> 2");
        let Item::Def(def) = &module.items[0] else {
            panic!()
        };
        let ExprKind::Match { arms, .. } = &def.value.kind else {
            panic!()
        };
        assert_eq!(arms.len(), 1);
        let module = parse_ok("x = match a | A -> (match b | C -> 1) | D -> 2");
        let Item::Def(def) = &module.items[0] else {
            panic!()
        };
        let ExprKind::Match { arms, .. } = &def.value.kind else {
            panic!()
        };
        assert_eq!(arms.len(), 2);
    }

    #[test]
    fn match_without_arms_is_an_error() {
        let (_, diagnostics) = parse("x = match y -> 1");
        assert_eq!(diagnostics[0].message, "expected `|`, found `->`");
    }

    #[test]
    fn stray_token_is_reported() {
        let (_, diagnostics) = parse(")");
        assert_eq!(diagnostics[0].message, "expected an expression, found `)`");
    }
}
