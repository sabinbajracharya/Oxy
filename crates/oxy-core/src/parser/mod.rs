//! Recursive descent parser with Pratt parsing for expressions.
//!
//! Parses a token stream into an AST. Operator precedence follows Rust's rules.

use crate::ast::*;
use crate::errors::PipelineError;
use crate::lexer::{Span, Token, TokenKind};
use crate::types::{ERR_VARIANT, NONE_VARIANT, OK_VARIANT, OPTION_TYPE, RESULT_TYPE, SOME_VARIANT};

mod expr;
mod item;
mod pattern;
mod stmt;
mod ty;

/// Parsing-context state that varies as the parser descends into and
/// out of certain syntactic positions. Bundled together so it can grow
/// without inflating the Parser's surface area, and so the mode-flag
/// idioms (push/pop, save/restore) live in one place.
#[derive(Default)]
struct ParseContext {
    /// Stack of enclosing fn names while parsing nested fn bodies. Used
    /// to mangle nested item names so they can be hoisted to top-level
    /// without colliding (`fn outer() { fn inner() {} }` →
    /// `outer__inner`).
    fn_name_stack: Vec<String>,
    /// Nested items extracted from fn bodies during parsing; appended
    /// to `Program::items` at the end of `parse()`.
    hoisted_items: Vec<Item>,
    /// When true, an `Ident { ... }` sequence does NOT eagerly parse
    /// as a struct initializer. Set while parsing the condition of an
    /// `if` / `while` / `for` header so `if score < MAX { ... }`
    /// doesn't mistake `MAX { ... }` for a struct init. Matches Rust's
    /// "no-struct-literal" disambiguation.
    no_struct_literal: bool,
}

/// Parser for the Oxy language.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    ctx: ParseContext,
}

/// Operator precedence levels (lower number = lower precedence = binds less tightly).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
#[allow(dead_code)] // Variants like Call will be used in later phases
enum Precedence {
    None = 0,
    Assignment = 1,  // = += -= etc.
    Range = 3,       // .. ..=
    Or = 4,          // ||
    And = 5,         // &&
    BitOr = 6,       // |
    BitXor = 7,      // ^
    BitAnd = 8,      // &
    Equality = 9,    // == !=
    Comparison = 10, // < > <= >=
    Shift = 11,      // << >>
    Term = 12,       // + -
    Factor = 13,     // * / %
    Unary = 14,      // - ! & *
    Call = 15,       // () .
}

impl Precedence {
    /// Get precedence for a binary operator token.
    fn of_binary(kind: &TokenKind) -> Self {
        match kind {
            TokenKind::PipePipe => Precedence::Or,
            TokenKind::AmpAmp => Precedence::And,
            TokenKind::Pipe => Precedence::BitOr,
            TokenKind::Caret => Precedence::BitXor,
            TokenKind::Amp => Precedence::BitAnd,
            TokenKind::EqEq | TokenKind::BangEq => Precedence::Equality,
            TokenKind::Lt | TokenKind::Gt | TokenKind::LtEq | TokenKind::GtEq => {
                Precedence::Comparison
            }
            TokenKind::Shl | TokenKind::Shr => Precedence::Shift,
            TokenKind::Plus | TokenKind::Minus => Precedence::Term,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Factor,
            TokenKind::LParen
            | TokenKind::Dot
            | TokenKind::LBracket
            | TokenKind::Question
            | TokenKind::As => Precedence::Call,
            TokenKind::Eq
            | TokenKind::PlusEq
            | TokenKind::MinusEq
            | TokenKind::StarEq
            | TokenKind::SlashEq
            | TokenKind::PercentEq => Precedence::Assignment,
            TokenKind::DotDot | TokenKind::DotDotEq => Precedence::Range,
            _ => Precedence::None,
        }
    }
}

impl Parser {
    /// Create a new parser from a token stream.
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            ctx: ParseContext::default(),
        }
    }

    /// Parse with `no_struct_literal` forced to `true`, restoring the
    /// previous value on return. Replaces the
    /// `let saved = …; self.…no_struct_literal = true; …; self.…no_struct_literal = saved;`
    /// idiom that used to be open-coded at every `if`/`while`/`for`
    /// header.
    fn with_no_struct_literal<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        let saved = self.ctx.no_struct_literal;
        self.ctx.no_struct_literal = true;
        let result = f(self);
        self.ctx.no_struct_literal = saved;
        result
    }

    /// Parse the tokens into a [`Program`].
    pub fn parse(mut self) -> Result<Program, PipelineError> {
        let start_span = self.current_span();
        let mut items = Vec::new();

        while !self.is_at_end() {
            items.push(self.parse_item()?);
        }

        // Append any items hoisted from nested fn bodies. Their names are
        // mangled (e.g. `outer__inner`) so they coexist with user items.
        items.append(&mut self.ctx.hoisted_items);

        let end_span = if items.is_empty() {
            start_span
        } else {
            items.last().unwrap().span()
        };

        Ok(Program {
            items,
            span: self.merge_spans(start_span, end_span),
        })
    }

    // === Token-level utilities ===

    fn peek_kind(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn current_span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn prev_span(&self) -> Span {
        if self.pos > 0 {
            self.tokens[self.pos - 1].span
        } else {
            self.tokens[0].span
        }
    }

    fn is_at_end(&self) -> bool {
        self.peek_kind() == &TokenKind::Eof
    }

    fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.pos];
        if !self.is_at_end() {
            self.pos += 1;
        }
        token
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind)
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<&Token, PipelineError> {
        if self.check(&kind) {
            Ok(self.advance())
        } else {
            Err(self.error(format!(
                "expected {}, found {}",
                kind.description(),
                self.peek_kind().description()
            )))
        }
    }

    fn expect_ident(&mut self) -> Result<String, PipelineError> {
        match self.peek_kind().clone() {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            // Also accept `Self` as an identifier in type position
            TokenKind::SelfUpper => {
                self.advance();
                Ok("Self".to_string())
            }
            other => Err(self.error(format!(
                "expected identifier, found {}",
                other.description()
            ))),
        }
    }

    /// Like `expect_ident` but also accepts `self`, `super`, `crate`, `Self` as path segments.
    fn expect_path_segment(&mut self) -> Result<String, PipelineError> {
        match self.peek_kind().clone() {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            TokenKind::SelfUpper => {
                self.advance();
                Ok("Self".to_string())
            }
            TokenKind::SelfLower => {
                self.advance();
                Ok("self".to_string())
            }
            TokenKind::Super => {
                self.advance();
                Ok("super".to_string())
            }
            TokenKind::Crate => {
                self.advance();
                Ok("crate".to_string())
            }
            other => Err(self.error(format!(
                "expected identifier or path segment, found {}",
                other.description()
            ))),
        }
    }

    fn error(&self, message: String) -> PipelineError {
        let span = self.current_span();
        PipelineError::Parser {
            message,
            line: span.line,
            column: span.column,
        }
    }

    fn merge_spans(&self, start: Span, end: Span) -> Span {
        Span::new(start.start, end.end, start.line, start.column)
    }

    /// Parse zero or more elements separated by commas, with trailing comma
    /// allowed before any token in `close`. Returns immediately if the next
    /// token is one of the close delimiters.
    fn parse_comma_separated<T>(
        &mut self,
        close: &[TokenKind],
        mut parse_one: impl FnMut(&mut Self) -> Result<T, PipelineError>,
    ) -> Result<Vec<T>, PipelineError> {
        let mut items = Vec::new();
        if close.iter().any(|c| self.check(c)) {
            return Ok(items);
        }
        loop {
            items.push(parse_one(self)?);
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
            if close.iter().any(|c| self.check(c)) {
                break; // trailing comma
            }
        }
        Ok(items)
    }

    /// Parse the raw content of an f-string into `FStringPart`s.
    fn parse_fstring_parts(
        &self,
        raw: &str,
        span: Span,
    ) -> Result<Vec<FStringPart>, PipelineError> {
        let mut parts = Vec::new();
        let mut literal = String::new();
        let chars: Vec<char> = raw.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '{' {
                // Escaped brace `{{` → literal `{`
                if i + 1 < chars.len() && chars[i + 1] == '{' {
                    literal.push('{');
                    i += 2;
                    continue;
                }
                // Interpolation: collect until matching `}`
                if !literal.is_empty() {
                    parts.push(FStringPart::Literal(std::mem::take(&mut literal)));
                }
                i += 1; // skip `{`
                let mut depth = 1;
                let mut expr_text = String::new();
                while i < chars.len() && depth > 0 {
                    let c = chars[i];
                    if c == '{' {
                        depth += 1;
                        expr_text.push(c);
                    } else if c == '}' {
                        depth -= 1;
                        if depth > 0 {
                            expr_text.push(c);
                        }
                    } else {
                        expr_text.push(c);
                    }
                    i += 1;
                }
                if depth > 0 {
                    return Err(PipelineError::Parser {
                        message: "unterminated interpolation in f-string".into(),
                        line: span.line,
                        column: span.column,
                    });
                }
                // Parse the expression text via a sub-parser
                let tokens =
                    crate::lexer::tokenize(&expr_text).map_err(|_| PipelineError::Parser {
                        message: format!("failed to tokenize f-string expression: {expr_text}"),
                        line: span.line,
                        column: span.column,
                    })?;
                let mut sub_parser = Parser::new(tokens);
                let expr =
                    sub_parser
                        .parse_expr(Precedence::None)
                        .map_err(|_| PipelineError::Parser {
                            message: format!("failed to parse f-string expression: {expr_text}"),
                            line: span.line,
                            column: span.column,
                        })?;
                parts.push(FStringPart::Expr(Box::new(expr)));
            } else if chars[i] == '}' {
                // Escaped brace `}}` → literal `}`
                if i + 1 < chars.len() && chars[i + 1] == '}' {
                    literal.push('}');
                    i += 2;
                    continue;
                }
                literal.push('}');
                i += 1;
            } else {
                literal.push(chars[i]);
                i += 1;
            }
        }

        if !literal.is_empty() {
            parts.push(FStringPart::Literal(literal));
        }

        Ok(parts)
    }
}

/// Convenience function to parse source code into an AST.
pub fn parse(source: &str) -> Result<Program, PipelineError> {
    let tokens = crate::lexer::tokenize(source)?;
    Parser::new(tokens).parse()
}

/// Read the declared name from an Item (for the nested-item hoist machinery).
/// Only Function/Struct/Enum are reachable from `parse_stmt`'s nested-item
/// branch today; other variants are not currently parseable inside a fn body.
fn item_name(item: &Item) -> &str {
    match item {
        Item::Function(f) => &f.name,
        Item::Struct(s) => &s.name,
        Item::Enum(e) => &e.name,
        Item::Impl(i) => &i.type_name,
        Item::ImplTrait(i) => &i.type_name,
        Item::Trait(t) => &t.name,
        Item::Module(m) => &m.name,
        Item::TypeAlias { name, .. } => name,
        Item::Const { name, .. } => name,
        Item::Use(_) => "",
    }
}

/// Replace an Item's declared name (used to rename hoisted nested items
/// before they're appended to the program's top-level items).
fn rename_item(item: Item, new_name: String) -> Item {
    match item {
        Item::Function(mut f) => {
            f.name = new_name;
            Item::Function(f)
        }
        Item::Struct(mut s) => {
            s.name = new_name;
            Item::Struct(s)
        }
        Item::Enum(mut e) => {
            e.name = new_name;
            Item::Enum(e)
        }
        other => other,
    }
}

#[cfg(test)]
mod tests;
