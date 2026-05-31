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
    Assignment = 1, // = += -= etc.
    Range = 2,      // .. ..=
    Or = 3,         // ||
    And = 4,        // &&
    BitOr = 5,      // |
    BitXor = 6,     // ^
    BitAnd = 7,     // &
    Equality = 8,   // == !=
    Comparison = 9, // < > <= >=
    Shift = 10,     // << >>
    Term = 11,      // + -
    Factor = 12,    // * / %
    Unary = 13,     // - ! & *
    Call = 14,      // () .
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
#[allow(irrefutable_let_patterns)] // Item only has Function for now; more variants coming
mod tests {
    use super::*;
    use crate::lexer::IntegerSuffix;

    /// Extract the function body statements from a single-function program.
    fn parse_fn_body(src: &str) -> Vec<Stmt> {
        let program = parse(src).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function item");
        };
        f.body.stmts.clone()
    }

    /// Extract a FnDef from the first item.
    fn parse_fn(src: &str) -> FnDef {
        let program = parse(src).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function item");
        };
        f.clone()
    }

    // === Let statements ===

    #[test]
    fn test_let_simple() {
        let stmts = parse_fn_body("fn main() { let x = 42; }");
        assert_eq!(stmts.len(), 1);
        let Stmt::Let {
            name,
            mutable,
            value,
            ..
        } = &stmts[0]
        else {
            panic!("expected let statement");
        };
        assert_eq!(name, "x");
        assert!(!mutable);
        assert!(value.is_some());
    }

    #[test]
    fn test_let_mut() {
        let stmts = parse_fn_body("fn main() { let mut x = 10; }");
        let Stmt::Let { name, mutable, .. } = &stmts[0] else {
            panic!("expected let statement");
        };
        assert_eq!(name, "x");
        assert!(mutable);
    }

    #[test]
    fn test_let_with_type() {
        let stmts = parse_fn_body("fn main() { let x: i64 = 42; }");
        let Stmt::Let { type_ann, .. } = &stmts[0] else {
            panic!("expected let statement");
        };
        assert_eq!(type_ann.as_ref().unwrap().name(), "i64");
    }

    // === Functions ===

    #[test]
    fn test_fn_no_params() {
        let f = parse_fn("fn main() {}");
        assert_eq!(f.name, "main");
        assert!(f.params.is_empty());
        assert!(f.return_type.is_none());
    }

    #[test]
    fn test_fn_with_params_and_return() {
        let f = parse_fn("fn add(a: i64, b: i64) -> i64 { a }");
        assert_eq!(f.name, "add");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].name, "a");
        assert_eq!(f.params[0].type_ann.name(), "i64");
        assert_eq!(f.params[1].name, "b");
        assert_eq!(f.return_type.as_ref().unwrap().name(), "i64");
    }

    #[test]
    fn test_multiple_functions() {
        let program = parse("fn foo() {} fn bar() {}").unwrap();
        assert_eq!(program.items.len(), 2);
    }

    // === Expressions ===

    #[test]
    fn test_arithmetic_precedence() {
        let stmts = parse_fn_body("fn main() { 1 + 2 * 3; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        // Should be Add(1, Mul(2, 3))
        let Expr::BinaryOp {
            op, left, right, ..
        } = expr
        else {
            panic!("expected BinaryOp");
        };
        assert_eq!(*op, BinOp::Add);
        assert!(matches!(
            **left,
            Expr::IntLiteral(1, IntegerSuffix::None, _)
        ));
        let Expr::BinaryOp { op: inner_op, .. } = right.as_ref() else {
            panic!("expected Mul on right");
        };
        assert_eq!(*inner_op, BinOp::Mul);
    }

    #[test]
    fn test_grouping() {
        let stmts = parse_fn_body("fn main() { (1 + 2) * 3; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::BinaryOp { op, .. } = expr else {
            panic!("expected Mul at top");
        };
        assert_eq!(*op, BinOp::Mul);
    }

    #[test]
    fn test_unary_negation() {
        let stmts = parse_fn_body("fn main() { -42; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::UnaryOp {
            op, expr: inner, ..
        } = expr
        else {
            panic!("expected UnaryOp");
        };
        assert_eq!(*op, UnaryOp::Neg);
        assert!(matches!(
            **inner,
            Expr::IntLiteral(42, IntegerSuffix::None, _)
        ));
    }

    #[test]
    fn test_unary_not() {
        let stmts = parse_fn_body("fn main() { !true; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::UnaryOp { op, .. } = expr else {
            panic!("expected UnaryOp");
        };
        assert_eq!(*op, UnaryOp::Not);
    }

    #[test]
    fn test_comparison_operators() {
        let stmts = parse_fn_body("fn main() { 1 < 2; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::BinaryOp { op, .. } = expr else {
            panic!("expected BinaryOp");
        };
        assert_eq!(*op, BinOp::Lt);
    }

    #[test]
    fn test_logical_operators() {
        let stmts = parse_fn_body("fn main() { true && false || true; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        // || has lower precedence than &&, so: Or(And(true, false), true)
        let Expr::BinaryOp { op, .. } = expr else {
            panic!("expected Or at top");
        };
        assert_eq!(*op, BinOp::Or);
    }

    // === Function calls ===

    #[test]
    fn test_function_call() {
        let stmts = parse_fn_body("fn main() { foo(1, 2); }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::Call { callee, args, .. } = expr else {
            panic!("expected Call");
        };
        assert!(matches!(callee.as_ref(), Expr::Ident(name, _) if name == "foo"));
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_function_call_no_args() {
        let stmts = parse_fn_body("fn main() { foo(); }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::Call { args, .. } = expr else {
            panic!("expected Call");
        };
        assert!(args.is_empty());
    }

    // === Macro calls ===

    #[test]
    fn test_println_macro() {
        let stmts = parse_fn_body(r#"fn main() { println!("hello {}", x); }"#);
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::MacroCall { name, args, .. } = expr else {
            panic!("expected MacroCall");
        };
        assert_eq!(name, "println");
        assert_eq!(args.len(), 2);
    }

    // === If expressions ===

    #[test]
    fn test_if_expr() {
        let stmts = parse_fn_body("fn main() { if true { 1; } }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::If { else_block, .. } = expr else {
            panic!("expected If");
        };
        assert!(else_block.is_none());
    }

    #[test]
    fn test_if_else_expr() {
        let stmts = parse_fn_body("fn main() { if true { 1; } else { 2; } }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::If { else_block, .. } = expr else {
            panic!("expected If");
        };
        assert!(else_block.is_some());
    }

    #[test]
    fn test_if_else_if() {
        let stmts = parse_fn_body("fn main() { if true { 1; } else if false { 2; } else { 3; } }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::If { else_block, .. } = expr else {
            panic!("expected If");
        };
        assert!(matches!(else_block.as_deref(), Some(Expr::If { .. })));
    }

    // === Block expressions ===

    #[test]
    fn test_block_as_value() {
        let stmts = parse_fn_body("fn main() { let x = { 42 }; }");
        let Stmt::Let {
            value: Some(expr), ..
        } = &stmts[0]
        else {
            panic!("expected let with block value");
        };
        assert!(matches!(expr, Expr::Block(_)));
    }

    // === Assignment ===

    #[test]
    fn test_assignment() {
        let stmts = parse_fn_body("fn main() { x = 42; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        assert!(matches!(expr, Expr::Assign { .. }));
    }

    #[test]
    fn test_compound_assignment() {
        let stmts = parse_fn_body("fn main() { x += 1; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::CompoundAssign { op, .. } = expr else {
            panic!("expected compound assignment");
        };
        assert_eq!(*op, BinOp::Add);
    }

    // === Return ===

    #[test]
    fn test_return_value() {
        let stmts = parse_fn_body("fn main() { return 42; }");
        let Stmt::Return { value, .. } = &stmts[0] else {
            panic!("expected return");
        };
        assert!(value.is_some());
    }

    #[test]
    fn test_return_void() {
        let stmts = parse_fn_body("fn main() { return; }");
        let Stmt::Return { value, .. } = &stmts[0] else {
            panic!("expected return");
        };
        assert!(value.is_none());
    }

    // === Tail expressions ===

    #[test]
    fn test_tail_expression() {
        let stmts = parse_fn_body("fn add(a: i64, b: i64) -> i64 { a + b }");
        assert_eq!(stmts.len(), 1);
        let Stmt::Expr { has_semicolon, .. } = &stmts[0] else {
            panic!("expected expression statement");
        };
        assert!(!has_semicolon, "tail expression should not have semicolon");
    }

    // === Pretty print ===

    #[test]
    fn test_pretty_print() {
        let program = parse("fn main() { let x: i64 = 1 + 2; }").unwrap();
        let output = program.pretty_print();
        assert!(output.contains("fn main()"));
        assert!(output.contains("let x: i64 = (1 + 2);"));
    }

    // === Error cases ===

    #[test]
    fn test_missing_semicolon_in_let() {
        let result = parse("fn main() { let x = 42 }");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected ';'"));
    }

    #[test]
    fn test_missing_rbrace() {
        let result = parse("fn main() { let x = 42;");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_fn_name() {
        let result = parse("fn () {}");
        assert!(result.is_err());
    }

    // === Reference syntax (parsed but ignored) ===

    #[test]
    fn test_reference_in_param_rejected() {
        // Oxy rejects `&T` (and `&self`, `&str`, etc.) — Rust-shaped without
        // borrow checking. See CLAUDE.md "Language Identity".
        let result = parse("fn foo(x: &i64) {}\nfn main() {}");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("references are not supported"),
            "expected fix-it error, got: {}",
            msg
        );
    }

    // === Full program ===

    #[test]
    fn test_full_program() {
        let src = r#"
fn add(a: i64, b: i64) -> i64 {
    a + b
}

fn main() {
    let x: i64 = 10;
    let y: i64 = 20;
    let result = add(x, y);
    println!("Result: {}", result);
}
"#;
        let program = parse(src).unwrap();
        assert_eq!(program.items.len(), 2);

        let Item::Function(f0) = &program.items[0] else {
            panic!("expected function item");
        };
        assert_eq!(f0.name, "add");
        assert_eq!(f0.params.len(), 2);

        let Item::Function(f1) = &program.items[1] else {
            panic!("expected function item");
        };
        assert_eq!(f1.name, "main");
        assert_eq!(f1.body.stmts.len(), 4);
    }

    // === String and bool literals ===

    #[test]
    fn test_string_literal_expr() {
        let stmts = parse_fn_body(r#"fn main() { "hello"; }"#);
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        assert!(matches!(expr, Expr::StringLiteral(s, _) if s == "hello"));
    }

    #[test]
    fn test_bool_literal_expr() {
        let stmts = parse_fn_body("fn main() { true; false; }");
        assert_eq!(stmts.len(), 2);
    }

    // === Phase 5: Control Flow ===

    #[test]
    fn test_while_stmt() {
        let stmts = parse_fn_body("fn main() { while x > 0 { x -= 1; } }");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Stmt::While { .. }));
    }

    #[test]
    fn test_loop_stmt() {
        let stmts = parse_fn_body("fn main() { loop { break; } }");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Stmt::Loop { .. }));
    }

    #[test]
    fn test_for_stmt() {
        let stmts = parse_fn_body("fn main() { for i in 0..10 { println!(\"{}\", i); } }");
        assert_eq!(stmts.len(), 1);
        let Stmt::For { name, iterable, .. } = &stmts[0] else {
            panic!("expected for");
        };
        assert_eq!(name, "i");
        assert!(matches!(
            iterable.as_ref(),
            Expr::Range {
                inclusive: false,
                ..
            }
        ));
    }

    #[test]
    fn test_for_inclusive_range() {
        let stmts = parse_fn_body("fn main() { for i in 0..=10 { x; } }");
        let Stmt::For { iterable, .. } = &stmts[0] else {
            panic!();
        };
        assert!(matches!(
            iterable.as_ref(),
            Expr::Range {
                inclusive: true,
                ..
            }
        ));
    }

    #[test]
    fn test_break_stmt() {
        let stmts = parse_fn_body("fn main() { break; }");
        assert!(matches!(&stmts[0], Stmt::Break { value: None, .. }));
    }

    #[test]
    fn test_break_with_value() {
        let stmts = parse_fn_body("fn main() { break 42; }");
        assert!(matches!(&stmts[0], Stmt::Break { value: Some(_), .. }));
    }

    #[test]
    fn test_continue_stmt() {
        let stmts = parse_fn_body("fn main() { continue; }");
        assert!(matches!(&stmts[0], Stmt::Continue { .. }));
    }

    #[test]
    fn test_match_expr() {
        let stmts = parse_fn_body(r#"fn main() { match x { 1 => "one", _ => "other" }; }"#);
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Match { arms, .. } = expr else {
            panic!();
        };
        assert_eq!(arms.len(), 2);
        assert!(matches!(&arms[0].pattern, Pattern::Literal(_)));
        assert!(matches!(&arms[1].pattern, Pattern::Wildcard(_)));
    }

    #[test]
    fn test_match_with_blocks() {
        let stmts = parse_fn_body(
            r#"fn main() { match x { 1 => { println!("one"); } _ => { println!("other"); } }; }"#,
        );
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        assert!(matches!(expr, Expr::Match { .. }));
    }

    #[test]
    fn test_match_variable_pattern() {
        let stmts = parse_fn_body("fn main() { match x { n => n + 1 }; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Match { arms, .. } = expr else {
            panic!();
        };
        assert!(matches!(&arms[0].pattern, Pattern::Ident(name, _) if name == "n"));
    }

    #[test]
    fn test_range_expression() {
        let stmts = parse_fn_body("fn main() { 0..10; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        assert!(matches!(
            expr,
            Expr::Range {
                inclusive: false,
                ..
            }
        ));
    }

    #[test]
    fn test_range_inclusive_expression() {
        let stmts = parse_fn_body("fn main() { 0..=10; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        assert!(matches!(
            expr,
            Expr::Range {
                inclusive: true,
                ..
            }
        ));
    }

    // === Phase 6: Collections & Strings ===

    #[test]
    fn test_array_literal() {
        let stmts = parse_fn_body("fn main() { [1, 2, 3]; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Array { elements, .. } = expr else {
            panic!("expected Array");
        };
        assert_eq!(elements.len(), 3);
    }

    #[test]
    fn test_empty_array() {
        let stmts = parse_fn_body("fn main() { []; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Array { elements, .. } = expr else {
            panic!("expected Array");
        };
        assert_eq!(elements.len(), 0);
    }

    #[test]
    fn test_index_expr() {
        let stmts = parse_fn_body("fn main() { v[0]; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        assert!(matches!(expr, Expr::Index { .. }));
    }

    #[test]
    fn test_method_call() {
        let stmts = parse_fn_body("fn main() { v.push(1); }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::MethodCall { method, args, .. } = expr else {
            panic!("expected MethodCall");
        };
        assert_eq!(method, "push");
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn test_field_access() {
        let stmts = parse_fn_body("fn main() { t.0; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::FieldAccess { field, .. } = expr else {
            panic!("expected FieldAccess");
        };
        assert_eq!(field, "0");
    }

    #[test]
    fn test_tuple_literal() {
        let stmts = parse_fn_body("fn main() { (1, 2, 3); }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Tuple { elements, .. } = expr else {
            panic!("expected Tuple, got {expr:?}");
        };
        assert_eq!(elements.len(), 3);
    }

    #[test]
    fn test_single_element_tuple() {
        let stmts = parse_fn_body("fn main() { (42,); }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Tuple { elements, .. } = expr else {
            panic!("expected Tuple");
        };
        assert_eq!(elements.len(), 1);
    }

    #[test]
    fn test_grouped_expr_not_tuple() {
        let stmts = parse_fn_body("fn main() { (42); }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        assert!(matches!(expr, Expr::Grouped(_, _)));
    }

    #[test]
    fn test_empty_tuple() {
        let stmts = parse_fn_body("fn main() { (); }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Tuple { elements, .. } = expr else {
            panic!("expected Tuple");
        };
        assert_eq!(elements.len(), 0);
    }

    #[test]
    fn test_vec_macro_brackets() {
        let stmts = parse_fn_body("fn main() { vec![1, 2, 3]; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::MacroCall { name, args, .. } = expr else {
            panic!("expected MacroCall");
        };
        assert_eq!(name, "vec");
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn test_chained_method_calls() {
        let stmts = parse_fn_body(r#"fn main() { s.trim().to_uppercase(); }"#);
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::MethodCall { method, object, .. } = expr else {
            panic!("expected MethodCall");
        };
        assert_eq!(method, "to_uppercase");
        assert!(matches!(object.as_ref(), Expr::MethodCall { method, .. } if method == "trim"));
    }

    #[test]
    fn test_chained_index() {
        let stmts = parse_fn_body("fn main() { v[0][1]; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Index { object, .. } = expr else {
            panic!("expected Index");
        };
        assert!(matches!(object.as_ref(), Expr::Index { .. }));
    }

    // === Phase 7: Struct/Enum/Impl parsing ===

    #[test]
    fn test_struct_def() {
        let program = parse(
            r#"
struct Point {
    x: f64,
    y: f64,
}
fn main() {}
"#,
        )
        .unwrap();
        let Item::Struct(s) = &program.items[0] else {
            panic!("expected struct");
        };
        assert_eq!(s.name, "Point");
        assert!(matches!(&s.kind, StructKind::Named(fields) if fields.len() == 2));
    }

    #[test]
    fn test_enum_def() {
        let program = parse(
            r#"
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
    Point,
}
fn main() {}
"#,
        )
        .unwrap();
        let Item::Enum(e) = &program.items[0] else {
            panic!("expected enum");
        };
        assert_eq!(e.name, "Shape");
        assert_eq!(e.variants.len(), 3);
        assert!(matches!(&e.variants[0].kind, EnumVariantKind::Tuple(t) if t.len() == 1));
        assert!(matches!(&e.variants[2].kind, EnumVariantKind::Unit));
    }

    #[test]
    fn test_impl_block() {
        let program = parse(
            r#"
struct Point { x: f64, y: f64 }

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }
}
fn main() {}
"#,
        )
        .unwrap();
        let Item::Impl(i) = &program.items[1] else {
            panic!("expected impl");
        };
        assert_eq!(i.type_name, "Point");
        assert_eq!(i.methods.len(), 1);
        assert_eq!(i.methods[0].name, "new");
    }

    #[test]
    fn test_struct_init_expr() {
        let stmts = parse_fn_body("fn main() { let p = Point { x: 1.0, y: 2.0 }; }");
        let Stmt::Let {
            value: Some(expr), ..
        } = &stmts[0]
        else {
            panic!("expected let with value");
        };
        assert!(matches!(expr, Expr::StructInit { name, .. } if name == "Point"));
    }

    #[test]
    fn test_path_call_expr() {
        let stmts = parse_fn_body("fn main() { Point::new(1.0, 2.0); }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        assert!(matches!(expr, Expr::PathCall { path, .. } if path == &["Point", "new"]));
    }

    #[test]
    fn test_path_expr() {
        let stmts = parse_fn_body("fn main() { Color::Red; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        assert!(matches!(expr, Expr::Path { segments, .. } if segments == &["Color", "Red"]));
    }

    #[test]
    fn test_self_method_param() {
        let program = parse(
            r#"
impl Foo {
    fn bar(self) -> i64 {
        42
    }
}
fn main() {}
"#,
        )
        .unwrap();
        let Item::Impl(i) = &program.items[0] else {
            panic!("expected impl");
        };
        assert_eq!(i.methods[0].params[0].name, "self");
    }

    #[test]
    fn test_enum_variant_pattern() {
        let stmts = parse_fn_body(
            r#"fn main() {
    match x {
        Shape::Circle(r) => r,
        _ => 0.0,
    };
}"#,
        );
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Match { arms, .. } = expr else {
            panic!("expected match");
        };
        assert!(matches!(
            &arms[0].pattern,
            Pattern::EnumVariant {
                enum_name,
                variant,
                ..
            } if enum_name == "Shape" && variant == "Circle"
        ));
    }

    #[test]
    fn test_unit_struct_def() {
        let program = parse("struct Marker;\nfn main() {}").unwrap();
        let Item::Struct(s) = &program.items[0] else {
            panic!("expected struct");
        };
        assert_eq!(s.name, "Marker");
        assert!(matches!(s.kind, StructKind::Unit));
    }

    #[test]
    fn test_tuple_struct_def() {
        let program = parse("struct Pair(i64, i64);\nfn main() {}").unwrap();
        let Item::Struct(s) = &program.items[0] else {
            panic!("expected struct");
        };
        assert_eq!(s.name, "Pair");
        assert!(matches!(&s.kind, StructKind::Tuple(t) if t.len() == 2));
    }

    // === Phase 8: Traits & Generics ===

    #[test]
    fn test_trait_def() {
        let program = parse("trait Greet { fn greet(self) -> String; }\nfn main() {}").unwrap();
        let Item::Trait(t) = &program.items[0] else {
            panic!("expected trait");
        };
        assert_eq!(t.name, "Greet");
        assert_eq!(t.methods.len(), 1);
        assert_eq!(t.methods[0].name, "greet");
    }

    #[test]
    fn test_trait_with_default_method() {
        let program = parse(
            r#"trait Foo { fn bar(self) -> i64 { 42 } }
fn main() {}"#,
        )
        .unwrap();
        let Item::Trait(t) = &program.items[0] else {
            panic!("expected trait");
        };
        assert_eq!(t.name, "Foo");
        assert_eq!(t.methods.len(), 0);
        assert_eq!(t.default_methods.len(), 1);
        assert_eq!(t.default_methods[0].name, "bar");
    }

    #[test]
    fn test_impl_trait_for_type() {
        let program = parse(
            r#"trait Greet { fn greet(self) -> String; }
struct Person { name: String }
impl Greet for Person { fn greet(self) -> String { self.name } }
fn main() {}"#,
        )
        .unwrap();
        assert!(matches!(&program.items[0], Item::Trait(_)));
        let Item::ImplTrait(i) = &program.items[2] else {
            panic!("expected impl trait block");
        };
        assert_eq!(i.trait_name, "Greet");
        assert_eq!(i.type_name, "Person");
        assert_eq!(i.methods.len(), 1);
    }

    #[test]
    fn test_generic_fn_def() {
        let program = parse("fn identity<T>(x: T) -> T { x }\nfn main() {}").unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        assert_eq!(f.name, "identity");
        assert_eq!(f.generic_params.len(), 1);
        assert_eq!(f.generic_params[0].name, "T");
        assert!(f.generic_params[0].bounds.is_empty());
    }

    #[test]
    fn test_generic_fn_with_bounds() {
        let program =
            parse("fn print_val<T: Display>(x: T) { println!(\"{}\", x); }\nfn main() {}").unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        assert_eq!(f.generic_params.len(), 1);
        assert_eq!(f.generic_params[0].name, "T");
        assert_eq!(f.generic_params[0].bounds, vec!["Display"]);
    }

    #[test]
    fn test_generic_fn_multiple_params() {
        let program = parse("fn foo<A, B: Clone + Debug>(a: A, b: B) { }\nfn main() {}").unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        assert_eq!(f.generic_params.len(), 2);
        assert_eq!(f.generic_params[0].name, "A");
        assert!(f.generic_params[0].bounds.is_empty());
        assert_eq!(f.generic_params[1].name, "B");
        assert_eq!(f.generic_params[1].bounds, vec!["Clone", "Debug"]);
    }

    #[test]
    fn test_impl_trait_for_add() {
        let program = parse(
            r#"struct Vec2 { x: f64, y: f64 }
impl Add for Vec2 { fn add(self, other: Vec2) -> Vec2 { Vec2 { x: 0.0, y: 0.0 } } }
fn main() {}"#,
        )
        .unwrap();
        let Item::ImplTrait(i) = &program.items[1] else {
            panic!("expected impl trait block");
        };
        assert_eq!(i.trait_name, "Add");
        assert_eq!(i.type_name, "Vec2");
    }

    // === Phase 9: Error Handling ===

    #[test]
    fn test_if_let_expr() {
        let program =
            parse(r#"fn main() { if let Some(x) = foo() { println!("{}", x); } }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Expr { expr, .. } = &f.body.stmts[0] else {
            panic!("expected expr stmt");
        };
        assert!(matches!(expr, Expr::IfLet { .. }));
    }

    #[test]
    fn test_if_let_with_else() {
        let program = parse(r#"fn main() { if let Some(x) = foo() { x } else { 0 } }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Expr { expr, .. } = &f.body.stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::IfLet { else_block, .. } = expr else {
            panic!("expected if let");
        };
        assert!(else_block.is_some());
    }

    #[test]
    fn test_while_let_stmt() {
        let program =
            parse(r#"fn main() { while let Some(x) = v.pop() { println!("{}", x); } }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        assert!(matches!(&f.body.stmts[0], Stmt::WhileLet { .. }));
    }

    #[test]
    fn test_try_operator() {
        let program = parse(r#"fn main() { let x = foo()?; }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Let {
            value: Some(expr), ..
        } = &f.body.stmts[0]
        else {
            panic!("expected let with value");
        };
        assert!(matches!(expr, Expr::Try { .. }));
    }

    #[test]
    fn test_some_none_pattern() {
        let program = parse(r#"fn main() { match x { Some(v) => v, None => 0, } }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Expr { expr, .. } = &f.body.stmts[0] else {
            panic!("expected expr");
        };
        let Expr::Match { arms, .. } = expr else {
            panic!("expected match");
        };
        assert!(matches!(
            &arms[0].pattern,
            Pattern::EnumVariant {
                enum_name,
                variant,
                ..
            } if enum_name == "Option" && variant == "Some"
        ));
        assert!(matches!(
            &arms[1].pattern,
            Pattern::EnumVariant {
                enum_name,
                variant,
                ..
            } if enum_name == "Option" && variant == "None"
        ));
    }

    #[test]
    fn test_ok_err_pattern() {
        let program = parse(r#"fn main() { match x { Ok(v) => v, Err(e) => 0, } }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Expr { expr, .. } = &f.body.stmts[0] else {
            panic!("expected expr");
        };
        let Expr::Match { arms, .. } = expr else {
            panic!("expected match");
        };
        assert!(matches!(
            &arms[0].pattern,
            Pattern::EnumVariant {
                enum_name,
                variant,
                ..
            } if enum_name == "Result" && variant == "Ok"
        ));
        assert!(matches!(
            &arms[1].pattern,
            Pattern::EnumVariant {
                enum_name,
                variant,
                ..
            } if enum_name == "Result" && variant == "Err"
        ));
    }

    // === Phase 10: Closures ===

    #[test]
    fn test_closure_with_params() {
        let program = parse(r#"fn main() { let f = |x, y| x + y; }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Let {
            value: Some(expr), ..
        } = &f.body.stmts[0]
        else {
            panic!("expected let");
        };
        let Expr::Closure { params, .. } = expr else {
            panic!("expected closure");
        };
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "x");
        assert_eq!(params[1].name, "y");
    }

    #[test]
    fn test_closure_with_types() {
        let program = parse(r#"fn main() { let f = |x: i64| -> i64 { x * 2 }; }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Let {
            value: Some(expr), ..
        } = &f.body.stmts[0]
        else {
            panic!("expected let");
        };
        let Expr::Closure {
            params,
            return_type,
            ..
        } = expr
        else {
            panic!("expected closure");
        };
        assert_eq!(params.len(), 1);
        assert!(params[0].type_ann.is_some());
        assert!(return_type.is_some());
    }

    #[test]
    fn test_empty_closure() {
        let program = parse(r#"fn main() { let f = || 42; }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Let {
            value: Some(expr), ..
        } = &f.body.stmts[0]
        else {
            panic!("expected let");
        };
        let Expr::Closure { params, .. } = expr else {
            panic!("expected closure");
        };
        assert!(params.is_empty());
    }

    #[test]
    fn test_move_closure() {
        let program = parse(r#"fn main() { let f = move |x| x; }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Let {
            value: Some(expr), ..
        } = &f.body.stmts[0]
        else {
            panic!("expected let");
        };
        assert!(matches!(expr, Expr::Closure { .. }));
    }

    // === Phase 11: Modules & Use Statements ===

    #[test]
    fn test_inline_module() {
        let program =
            parse("mod math { fn add(a: i64, b: i64) -> i64 { a + b } } fn main() {}").unwrap();
        assert!(
            matches!(&program.items[0], Item::Module(m) if m.name == "math" && m.body.is_some())
        );
    }

    #[test]
    fn test_file_module() {
        let program = parse("mod utils; fn main() {}").unwrap();
        let Item::Module(m) = &program.items[0] else {
            panic!("expected module");
        };
        assert_eq!(m.name, "utils");
        assert!(m.body.is_none()); // file-based
    }

    #[test]
    fn test_use_simple() {
        let program = parse("use math::add; fn main() {}").unwrap();
        let Item::Use(u) = &program.items[0] else {
            panic!("expected use");
        };
        assert_eq!(u.path, vec!["math", "add"]);
        assert!(matches!(u.tree, UseTree::Simple(None)));
    }

    #[test]
    fn test_use_glob() {
        let program = parse("use math::*; fn main() {}").unwrap();
        let Item::Use(u) = &program.items[0] else {
            panic!("expected use");
        };
        assert_eq!(u.path, vec!["math"]);
        assert!(matches!(u.tree, UseTree::Glob));
    }

    #[test]
    fn test_use_group() {
        let program = parse("use math::{add, sub}; fn main() {}").unwrap();
        let Item::Use(u) = &program.items[0] else {
            panic!("expected use");
        };
        assert_eq!(u.path, vec!["math"]);
        let UseTree::Group(items) = &u.tree else {
            panic!("expected group");
        };
        let names: Vec<&str> = items.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["add", "sub"]);
    }

    #[test]
    fn test_pub_module() {
        let program =
            parse("pub mod math { fn add(a: i64, b: i64) -> i64 { a + b } } fn main() {}").unwrap();
        let Item::Module(m) = &program.items[0] else {
            panic!("expected module");
        };
        assert!(m.visibility.is_pub());
    }

    #[test]
    fn test_pub_fn() {
        let program = parse("pub fn helper() -> i64 { 42 } fn main() {}").unwrap();
        assert!(matches!(&program.items[0], Item::Function(_)));
    }

    #[test]
    fn test_type_alias() {
        let program = parse("type Meters = f64; fn main() {}").unwrap();
        let Item::TypeAlias { name, target, .. } = &program.items[0] else {
            panic!("expected type alias");
        };
        assert_eq!(name, "Meters");
        assert_eq!(target.name(), "f64");
    }

    #[test]
    fn test_const_def() {
        let program = parse("const MAX: i64 = 100; fn main() {}").unwrap();
        let Item::Const {
            name,
            type_ann,
            is_static,
            ..
        } = &program.items[0]
        else {
            panic!("expected const");
        };
        assert_eq!(name, "MAX");
        assert!(!is_static);
        assert_eq!(type_ann.as_ref().unwrap().name(), "i64");
    }

    #[test]
    fn test_static_def() {
        let program = parse("static COUNT: i64 = 0; fn main() {}").unwrap();
        let Item::Const {
            name, is_static, ..
        } = &program.items[0]
        else {
            panic!("expected static");
        };
        assert_eq!(name, "COUNT");
        assert!(is_static);
    }

    #[test]
    fn test_for_destructure() {
        let body = parse_fn_body("fn main() { for (k, v) in items { println!(\"{}\", k); } }");
        assert!(matches!(&body[0], Stmt::ForDestructure { .. }));
        if let Stmt::ForDestructure { names, .. } = &body[0] {
            assert_eq!(names, &["k", "v"]);
        }
    }

    #[test]
    fn test_turbofish_method() {
        let program = parse("fn main() { let x = obj.collect::<i64>(); }").unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected fn")
        };
        let Stmt::Let {
            value: Some(expr), ..
        } = &f.body.stmts[0]
        else {
            panic!("expected let")
        };
        match expr {
            Expr::MethodCall {
                method, turbofish, ..
            } => {
                assert_eq!(method, "collect");
                assert!(turbofish.is_some(), "expected turbofish");
                assert_eq!(turbofish.as_ref().unwrap().len(), 1);
                assert_eq!(turbofish.as_ref().unwrap()[0].name(), "i64");
            }
            other => panic!("expected MethodCall, got {:?}", other),
        }
    }

    #[test]
    fn test_turbofish_nested() {
        let program = parse("fn main() { let x = obj.collect::<Vec<i64>>(); }").unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected fn")
        };
        let Stmt::Let {
            value: Some(expr), ..
        } = &f.body.stmts[0]
        else {
            panic!("expected let")
        };
        match expr {
            Expr::MethodCall { turbofish, .. } => {
                assert!(
                    turbofish.is_some(),
                    "expected turbofish with nested generics"
                );
            }
            other => panic!("expected MethodCall, got {:?}", other),
        }
    }

    #[test]
    fn test_turbofish_call() {
        let program = parse("fn main() { let x = foo::<i64>(42); }").unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected fn")
        };
        let Stmt::Let {
            value: Some(expr), ..
        } = &f.body.stmts[0]
        else {
            panic!("expected let")
        };
        match expr {
            Expr::Call { turbofish, .. } => {
                assert!(turbofish.is_some(), "expected turbofish on call");
            }
            other => panic!("expected Call, got {:?}", other),
        }
    }
}
