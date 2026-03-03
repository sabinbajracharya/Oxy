//! Lexer for the Ferrite language.
//!
//! Tokenizes source code into a stream of [`Token`]s matching Rust's lexical grammar.

mod token;

pub use token::{Span, Token, TokenKind};

use crate::errors::FerriError;

/// Lexer that tokenizes Ferrite source code.
pub struct Lexer<'src> {
    source: &'src str,
    chars: Vec<char>,
    /// Current position in the char array.
    pos: usize,
    /// Current byte offset in the source.
    byte_offset: usize,
    /// Current line (1-based).
    line: usize,
    /// Current column (1-based).
    column: usize,
}

impl<'src> Lexer<'src> {
    /// Create a new lexer for the given source code.
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            chars: source.chars().collect(),
            pos: 0,
            byte_offset: 0,
            line: 1,
            column: 1,
        }
    }

    /// Tokenize the entire source, returning all tokens including a trailing `Eof`.
    pub fn tokenize(mut self) -> Result<Vec<Token>, FerriError> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    /// Scan the next token from the source.
    fn next_token(&mut self) -> Result<Token, FerriError> {
        self.skip_whitespace_and_comments()?;

        if self.is_at_end() {
            return Ok(self.make_token(TokenKind::Eof, self.byte_offset));
        }

        let start_offset = self.byte_offset;
        let ch = self.advance();

        let kind = match ch {
            // Single-character delimiters
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            '#' => TokenKind::Hash,
            '?' => TokenKind::Question,
            '^' => TokenKind::Caret,

            // Colon or ColonColon
            ':' => {
                if self.match_char(':') {
                    TokenKind::ColonColon
                } else {
                    TokenKind::Colon
                }
            }

            // Dot, DotDot, DotDotEq
            '.' => {
                if self.match_char('.') {
                    if self.match_char('=') {
                        TokenKind::DotDotEq
                    } else {
                        TokenKind::DotDot
                    }
                } else {
                    TokenKind::Dot
                }
            }

            // Plus or PlusEq
            '+' => {
                if self.match_char('=') {
                    TokenKind::PlusEq
                } else {
                    TokenKind::Plus
                }
            }

            // Minus, MinusEq, or Arrow
            '-' => {
                if self.match_char('=') {
                    TokenKind::MinusEq
                } else if self.match_char('>') {
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }

            // Star or StarEq
            '*' => {
                if self.match_char('=') {
                    TokenKind::StarEq
                } else {
                    TokenKind::Star
                }
            }

            // Slash or SlashEq (comments handled in skip_whitespace_and_comments)
            '/' => {
                if self.match_char('=') {
                    TokenKind::SlashEq
                } else {
                    TokenKind::Slash
                }
            }

            // Percent or PercentEq
            '%' => {
                if self.match_char('=') {
                    TokenKind::PercentEq
                } else {
                    TokenKind::Percent
                }
            }

            // Eq or EqEq or FatArrow
            '=' => {
                if self.match_char('=') {
                    TokenKind::EqEq
                } else if self.match_char('>') {
                    TokenKind::FatArrow
                } else {
                    TokenKind::Eq
                }
            }

            // Bang or BangEq
            '!' => {
                if self.match_char('=') {
                    TokenKind::BangEq
                } else {
                    TokenKind::Bang
                }
            }

            // Lt, LtEq, Shl
            '<' => {
                if self.match_char('=') {
                    TokenKind::LtEq
                } else if self.match_char('<') {
                    TokenKind::Shl
                } else {
                    TokenKind::Lt
                }
            }

            // Gt, GtEq, Shr
            '>' => {
                if self.match_char('=') {
                    TokenKind::GtEq
                } else if self.match_char('>') {
                    TokenKind::Shr
                } else {
                    TokenKind::Gt
                }
            }

            // Amp or AmpAmp
            '&' => {
                if self.match_char('&') {
                    TokenKind::AmpAmp
                } else {
                    TokenKind::Amp
                }
            }

            // Pipe or PipePipe
            '|' => {
                if self.match_char('|') {
                    TokenKind::PipePipe
                } else {
                    TokenKind::Pipe
                }
            }

            // String literal
            '"' => self.scan_string(start_offset)?,

            // Character literal
            '\'' => self.scan_char(start_offset)?,

            // Number literal
            c if c.is_ascii_digit() => self.scan_number(c, start_offset)?,

            // Identifier or keyword (including _)
            c if c == '_' || c.is_ascii_alphabetic() => self.scan_identifier(c, start_offset),

            other => {
                return Err(FerriError::Lexer {
                    message: format!("unexpected character '{other}'"),
                    line: self.line,
                    column: self.column - 1,
                });
            }
        };

        Ok(self.make_token(kind, start_offset))
    }

    // === Scanning helpers ===

    fn scan_string(&mut self, _start_offset: usize) -> Result<TokenKind, FerriError> {
        let start_line = self.line;
        let start_col = self.column;
        let mut value = String::new();

        loop {
            if self.is_at_end() {
                return Err(FerriError::Lexer {
                    message: "unterminated string literal".into(),
                    line: start_line,
                    column: start_col - 1,
                });
            }

            let ch = self.advance();
            match ch {
                '"' => break,
                '\\' => {
                    if self.is_at_end() {
                        return Err(FerriError::Lexer {
                            message: "unterminated escape sequence in string".into(),
                            line: self.line,
                            column: self.column,
                        });
                    }
                    let escaped = self.advance();
                    match escaped {
                        'n' => value.push('\n'),
                        't' => value.push('\t'),
                        'r' => value.push('\r'),
                        '\\' => value.push('\\'),
                        '"' => value.push('"'),
                        '0' => value.push('\0'),
                        '\'' => value.push('\''),
                        'x' => {
                            let hex = self.scan_hex_escape(2)?;
                            value.push(hex);
                        }
                        'u' => {
                            let ch = self.scan_unicode_escape()?;
                            value.push(ch);
                        }
                        _ => {
                            return Err(FerriError::Lexer {
                                message: format!("unknown escape sequence '\\{escaped}'"),
                                line: self.line,
                                column: self.column - 1,
                            });
                        }
                    }
                }
                _ => value.push(ch),
            }
        }

        Ok(TokenKind::StringLiteral(value))
    }

    fn scan_char(&mut self, _start_offset: usize) -> Result<TokenKind, FerriError> {
        if self.is_at_end() {
            return Err(FerriError::Lexer {
                message: "unterminated character literal".into(),
                line: self.line,
                column: self.column,
            });
        }

        let ch = self.advance();
        let value = if ch == '\\' {
            if self.is_at_end() {
                return Err(FerriError::Lexer {
                    message: "unterminated escape sequence in character literal".into(),
                    line: self.line,
                    column: self.column,
                });
            }
            let escaped = self.advance();
            match escaped {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\\' => '\\',
                '\'' => '\'',
                '0' => '\0',
                '"' => '"',
                'x' => self.scan_hex_escape(2)?,
                'u' => self.scan_unicode_escape()?,
                _ => {
                    return Err(FerriError::Lexer {
                        message: format!("unknown escape sequence '\\{escaped}'"),
                        line: self.line,
                        column: self.column - 1,
                    });
                }
            }
        } else {
            ch
        };

        if !self.match_char('\'') {
            return Err(FerriError::Lexer {
                message: "unterminated character literal (expected closing ')".into(),
                line: self.line,
                column: self.column,
            });
        }

        Ok(TokenKind::CharLiteral(value))
    }

    fn scan_hex_escape(&mut self, digits: usize) -> Result<char, FerriError> {
        let mut hex = String::with_capacity(digits);
        for _ in 0..digits {
            if self.is_at_end() || !self.peek().is_ascii_hexdigit() {
                return Err(FerriError::Lexer {
                    message: format!("expected {digits} hex digits in escape sequence"),
                    line: self.line,
                    column: self.column,
                });
            }
            hex.push(self.advance());
        }
        let code = u32::from_str_radix(&hex, 16).map_err(|_| FerriError::Lexer {
            message: format!("invalid hex escape '\\x{hex}'"),
            line: self.line,
            column: self.column,
        })?;
        char::from_u32(code).ok_or_else(|| FerriError::Lexer {
            message: format!("invalid character code in hex escape: {code}"),
            line: self.line,
            column: self.column,
        })
    }

    fn scan_unicode_escape(&mut self) -> Result<char, FerriError> {
        if !self.match_char('{') {
            return Err(FerriError::Lexer {
                message: "expected '{' in unicode escape sequence".into(),
                line: self.line,
                column: self.column,
            });
        }
        let mut hex = String::new();
        while !self.is_at_end() && self.peek() != '}' {
            if !self.peek().is_ascii_hexdigit() {
                return Err(FerriError::Lexer {
                    message: "invalid character in unicode escape".into(),
                    line: self.line,
                    column: self.column,
                });
            }
            hex.push(self.advance());
        }
        if !self.match_char('}') {
            return Err(FerriError::Lexer {
                message: "expected '}' to close unicode escape".into(),
                line: self.line,
                column: self.column,
            });
        }
        if hex.is_empty() || hex.len() > 6 {
            return Err(FerriError::Lexer {
                message: "unicode escape must have 1-6 hex digits".into(),
                line: self.line,
                column: self.column,
            });
        }
        let code = u32::from_str_radix(&hex, 16).map_err(|_| FerriError::Lexer {
            message: format!("invalid unicode escape '\\u{{{hex}}}'"),
            line: self.line,
            column: self.column,
        })?;
        char::from_u32(code).ok_or_else(|| FerriError::Lexer {
            message: format!("invalid unicode code point: U+{code:04X}"),
            line: self.line,
            column: self.column,
        })
    }

    fn scan_number(&mut self, first: char, _start_offset: usize) -> Result<TokenKind, FerriError> {
        let mut num_str = String::new();
        num_str.push(first);

        // Check for hex, octal, binary prefixes
        if first == '0' && !self.is_at_end() {
            match self.peek() {
                'x' | 'X' => {
                    num_str.push(self.advance());
                    while !self.is_at_end()
                        && (self.peek().is_ascii_hexdigit() || self.peek() == '_')
                    {
                        let ch = self.advance();
                        if ch != '_' {
                            num_str.push(ch);
                        }
                    }
                    let hex_part = &num_str[2..];
                    let val = i64::from_str_radix(hex_part, 16).map_err(|_| FerriError::Lexer {
                        message: format!("invalid hex literal '{num_str}'"),
                        line: self.line,
                        column: self.column,
                    })?;
                    return Ok(TokenKind::IntLiteral(val));
                }
                'o' | 'O' => {
                    num_str.push(self.advance());
                    while !self.is_at_end() && (self.peek().is_digit(8) || self.peek() == '_') {
                        let ch = self.advance();
                        if ch != '_' {
                            num_str.push(ch);
                        }
                    }
                    let oct_part = &num_str[2..];
                    let val = i64::from_str_radix(oct_part, 8).map_err(|_| FerriError::Lexer {
                        message: format!("invalid octal literal '{num_str}'"),
                        line: self.line,
                        column: self.column,
                    })?;
                    return Ok(TokenKind::IntLiteral(val));
                }
                'b' | 'B' => {
                    num_str.push(self.advance());
                    while !self.is_at_end()
                        && (self.peek() == '0' || self.peek() == '1' || self.peek() == '_')
                    {
                        let ch = self.advance();
                        if ch != '_' {
                            num_str.push(ch);
                        }
                    }
                    let bin_part = &num_str[2..];
                    let val = i64::from_str_radix(bin_part, 2).map_err(|_| FerriError::Lexer {
                        message: format!("invalid binary literal '{num_str}'"),
                        line: self.line,
                        column: self.column,
                    })?;
                    return Ok(TokenKind::IntLiteral(val));
                }
                _ => {}
            }
        }

        // Decimal digits
        while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
            let ch = self.advance();
            if ch != '_' {
                num_str.push(ch);
            }
        }

        let mut is_float = false;

        // Check for decimal point (but not `..` range)
        if !self.is_at_end() && self.peek() == '.' && !self.peek_next_is('.') {
            // Only treat as float if next char after dot is a digit or the dot
            // is not followed by an identifier (e.g., `1.method()` should be `1` then `.`)
            if self.pos + 1 < self.chars.len() && self.chars[self.pos + 1].is_ascii_digit() {
                is_float = true;
                num_str.push(self.advance()); // consume '.'
                while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
                    let ch = self.advance();
                    if ch != '_' {
                        num_str.push(ch);
                    }
                }
            }
        }

        // Exponent
        if !self.is_at_end() && (self.peek() == 'e' || self.peek() == 'E') {
            is_float = true;
            num_str.push(self.advance());
            if !self.is_at_end() && (self.peek() == '+' || self.peek() == '-') {
                num_str.push(self.advance());
            }
            while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
                let ch = self.advance();
                if ch != '_' {
                    num_str.push(ch);
                }
            }
        }

        // Type suffix (consume and ignore for now: i32, u64, f64, etc.)
        if !self.is_at_end() && (self.peek() == 'i' || self.peek() == 'u' || self.peek() == 'f') {
            let _suffix_start = self.pos;
            self.advance(); // consume letter
            if self.peek() == 'f' && !self.is_at_end() {
                // Could be f32/f64
                is_float = true;
            }
            while !self.is_at_end() && self.peek().is_ascii_digit() {
                self.advance();
            }
        }

        if is_float {
            let val: f64 = num_str.parse().map_err(|_| FerriError::Lexer {
                message: format!("invalid float literal '{num_str}'"),
                line: self.line,
                column: self.column,
            })?;
            Ok(TokenKind::FloatLiteral(val))
        } else {
            let val: i64 = num_str.parse().map_err(|_| FerriError::Lexer {
                message: format!("invalid integer literal '{num_str}'"),
                line: self.line,
                column: self.column,
            })?;
            Ok(TokenKind::IntLiteral(val))
        }
    }

    fn scan_identifier(&mut self, first: char, _start_offset: usize) -> TokenKind {
        let mut name = String::new();
        name.push(first);

        while !self.is_at_end() && (self.peek().is_ascii_alphanumeric() || self.peek() == '_') {
            name.push(self.advance());
        }

        // Check for lone underscore (wildcard pattern)
        if name == "_" {
            return TokenKind::Underscore;
        }

        // Check if it's a keyword
        TokenKind::from_keyword(&name).unwrap_or(TokenKind::Ident(name))
    }

    // === Whitespace and comment handling ===

    fn skip_whitespace_and_comments(&mut self) -> Result<(), FerriError> {
        loop {
            // Skip whitespace
            while !self.is_at_end() && self.peek().is_ascii_whitespace() {
                self.advance();
            }

            if self.is_at_end() {
                break;
            }

            // Line comment
            if self.peek() == '/' && self.peek_at(1) == Some('/') {
                self.advance(); // /
                self.advance(); // /
                while !self.is_at_end() && self.peek() != '\n' {
                    self.advance();
                }
                continue;
            }

            // Block comment (with nesting support)
            if self.peek() == '/' && self.peek_at(1) == Some('*') {
                let start_line = self.line;
                let start_col = self.column;
                self.advance(); // /
                self.advance(); // *
                let mut depth = 1;
                while !self.is_at_end() && depth > 0 {
                    if self.peek() == '/' && self.peek_at(1) == Some('*') {
                        self.advance();
                        self.advance();
                        depth += 1;
                    } else if self.peek() == '*' && self.peek_at(1) == Some('/') {
                        self.advance();
                        self.advance();
                        depth -= 1;
                    } else {
                        self.advance();
                    }
                }
                if depth > 0 {
                    return Err(FerriError::Lexer {
                        message: "unterminated block comment".into(),
                        line: start_line,
                        column: start_col,
                    });
                }
                continue;
            }

            break;
        }
        Ok(())
    }

    // === Low-level character operations ===

    fn is_at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn peek(&self) -> char {
        self.chars[self.pos]
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn peek_next_is(&self, ch: char) -> bool {
        self.peek_at(1) == Some(ch)
    }

    fn advance(&mut self) -> char {
        let ch = self.chars[self.pos];
        self.pos += 1;
        self.byte_offset += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        ch
    }

    fn match_char(&mut self, expected: char) -> bool {
        if !self.is_at_end() && self.peek() == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    fn make_token(&self, kind: TokenKind, start_offset: usize) -> Token {
        // Calculate the start line/column from the start_offset
        let (start_line, start_col) = self.line_col_at(start_offset);
        Token::new(
            kind,
            Span::new(start_offset, self.byte_offset, start_line, start_col),
        )
    }

    fn line_col_at(&self, byte_offset: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for (i, ch) in self.source.char_indices() {
            if i >= byte_offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }
}

/// Convenience function to tokenize a source string.
pub fn tokenize(source: &str) -> Result<Vec<Token>, FerriError> {
    Lexer::new(source).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to extract just the token kinds.
    fn kinds(src: &str) -> Vec<TokenKind> {
        tokenize(src).unwrap().into_iter().map(|t| t.kind).collect()
    }

    // === Operators ===

    #[test]
    fn test_single_char_operators() {
        assert_eq!(
            kinds("+ - * / % ^ ? # ;"),
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::Caret,
                TokenKind::Question,
                TokenKind::Hash,
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_two_char_operators() {
        assert_eq!(
            kinds("== != <= >= && || :: .. << >>"),
            vec![
                TokenKind::EqEq,
                TokenKind::BangEq,
                TokenKind::LtEq,
                TokenKind::GtEq,
                TokenKind::AmpAmp,
                TokenKind::PipePipe,
                TokenKind::ColonColon,
                TokenKind::DotDot,
                TokenKind::Shl,
                TokenKind::Shr,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_compound_assignment() {
        assert_eq!(
            kinds("+= -= *= /= %="),
            vec![
                TokenKind::PlusEq,
                TokenKind::MinusEq,
                TokenKind::StarEq,
                TokenKind::SlashEq,
                TokenKind::PercentEq,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_arrows() {
        assert_eq!(
            kinds("-> =>"),
            vec![TokenKind::Arrow, TokenKind::FatArrow, TokenKind::Eof]
        );
    }

    #[test]
    fn test_dot_dot_eq() {
        assert_eq!(kinds("..="), vec![TokenKind::DotDotEq, TokenKind::Eof]);
    }

    // === Delimiters ===

    #[test]
    fn test_delimiters() {
        assert_eq!(
            kinds("( ) { } [ ] , : ."),
            vec![
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Comma,
                TokenKind::Colon,
                TokenKind::Dot,
                TokenKind::Eof,
            ]
        );
    }

    // === Integers ===

    #[test]
    fn test_integer_literals() {
        assert_eq!(
            kinds("0 42 1_000_000"),
            vec![
                TokenKind::IntLiteral(0),
                TokenKind::IntLiteral(42),
                TokenKind::IntLiteral(1_000_000),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_hex_literal() {
        assert_eq!(
            kinds("0xFF 0x1A_2B"),
            vec![
                TokenKind::IntLiteral(0xFF),
                TokenKind::IntLiteral(0x1A2B),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_binary_literal() {
        assert_eq!(
            kinds("0b1010 0b1111_0000"),
            vec![
                TokenKind::IntLiteral(0b1010),
                TokenKind::IntLiteral(0b11110000),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_octal_literal() {
        assert_eq!(
            kinds("0o77 0o755"),
            vec![
                TokenKind::IntLiteral(0o77),
                TokenKind::IntLiteral(0o755),
                TokenKind::Eof,
            ]
        );
    }

    // === Floats ===

    #[test]
    fn test_float_literals() {
        assert_eq!(
            kinds("3.25 1.0 0.5"),
            vec![
                TokenKind::FloatLiteral(3.25),
                TokenKind::FloatLiteral(1.0),
                TokenKind::FloatLiteral(0.5),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_float_exponent() {
        assert_eq!(
            kinds("1e10 2.5E-3"),
            vec![
                TokenKind::FloatLiteral(1e10),
                TokenKind::FloatLiteral(2.5e-3),
                TokenKind::Eof,
            ]
        );
    }

    // === Strings ===

    #[test]
    fn test_string_literal() {
        assert_eq!(
            kinds(r#""hello world""#),
            vec![
                TokenKind::StringLiteral("hello world".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_string_escapes() {
        assert_eq!(
            kinds(r#""\n\t\r\\\"" "#),
            vec![
                TokenKind::StringLiteral("\n\t\r\\\"".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_string_hex_escape() {
        assert_eq!(
            kinds(r#""\x41""#),
            vec![TokenKind::StringLiteral("A".into()), TokenKind::Eof,]
        );
    }

    #[test]
    fn test_string_unicode_escape() {
        assert_eq!(
            kinds(r#""\u{1F600}""#),
            vec![TokenKind::StringLiteral("😀".into()), TokenKind::Eof,]
        );
    }

    #[test]
    fn test_unterminated_string() {
        let result = tokenize(r#""hello"#);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("unterminated string"));
    }

    // === Chars ===

    #[test]
    fn test_char_literal() {
        assert_eq!(
            kinds("'a' 'Z' '0'"),
            vec![
                TokenKind::CharLiteral('a'),
                TokenKind::CharLiteral('Z'),
                TokenKind::CharLiteral('0'),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_char_escape() {
        assert_eq!(
            kinds(r"'\n' '\t' '\\'"),
            vec![
                TokenKind::CharLiteral('\n'),
                TokenKind::CharLiteral('\t'),
                TokenKind::CharLiteral('\\'),
                TokenKind::Eof,
            ]
        );
    }

    // === Keywords ===

    #[test]
    fn test_keywords() {
        assert_eq!(
            kinds("let mut fn return if else while loop for in"),
            vec![
                TokenKind::Let,
                TokenKind::Mut,
                TokenKind::Fn,
                TokenKind::Return,
                TokenKind::If,
                TokenKind::Else,
                TokenKind::While,
                TokenKind::Loop,
                TokenKind::For,
                TokenKind::In,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_more_keywords() {
        assert_eq!(
            kinds("struct enum impl trait match pub use mod self Self"),
            vec![
                TokenKind::Struct,
                TokenKind::Enum,
                TokenKind::Impl,
                TokenKind::Trait,
                TokenKind::Match,
                TokenKind::Pub,
                TokenKind::Use,
                TokenKind::Mod,
                TokenKind::SelfLower,
                TokenKind::SelfUpper,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_bool_keywords() {
        assert_eq!(
            kinds("true false"),
            vec![TokenKind::True, TokenKind::False, TokenKind::Eof]
        );
    }

    // === Identifiers ===

    #[test]
    fn test_identifiers() {
        assert_eq!(
            kinds("foo bar_baz _private x1 CamelCase"),
            vec![
                TokenKind::Ident("foo".into()),
                TokenKind::Ident("bar_baz".into()),
                TokenKind::Ident("_private".into()),
                TokenKind::Ident("x1".into()),
                TokenKind::Ident("CamelCase".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_underscore_wildcard() {
        assert_eq!(
            kinds("_ _a"),
            vec![
                TokenKind::Underscore,
                TokenKind::Ident("_a".into()),
                TokenKind::Eof,
            ]
        );
    }

    // === Comments ===

    #[test]
    fn test_line_comment() {
        assert_eq!(
            kinds("let x // this is a comment\nlet y"),
            vec![
                TokenKind::Let,
                TokenKind::Ident("x".into()),
                TokenKind::Let,
                TokenKind::Ident("y".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_block_comment() {
        assert_eq!(
            kinds("let /* comment */ x"),
            vec![TokenKind::Let, TokenKind::Ident("x".into()), TokenKind::Eof,]
        );
    }

    #[test]
    fn test_nested_block_comment() {
        assert_eq!(
            kinds("let /* outer /* inner */ still comment */ x"),
            vec![TokenKind::Let, TokenKind::Ident("x".into()), TokenKind::Eof,]
        );
    }

    #[test]
    fn test_unterminated_block_comment() {
        let result = tokenize("/* never closed");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unterminated block comment"));
    }

    // === Spans ===

    #[test]
    fn test_span_tracking() {
        let tokens = tokenize("let x = 42;").unwrap();
        // 'let' starts at line 1, col 1
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[0].span.column, 1);
        // 'x' starts at line 1, col 5
        assert_eq!(tokens[1].span.line, 1);
        assert_eq!(tokens[1].span.column, 5);
    }

    #[test]
    fn test_multiline_span() {
        let tokens = tokenize("let x\nlet y").unwrap();
        assert_eq!(tokens[0].span.line, 1); // let
        assert_eq!(tokens[1].span.line, 1); // x
        assert_eq!(tokens[2].span.line, 2); // let
        assert_eq!(tokens[3].span.line, 2); // y
    }

    // === Full program ===

    #[test]
    fn test_full_program() {
        let src = r#"
fn main() {
    let x: i64 = 42;
    println!("value: {}", x);
}
"#;
        let tokens = tokenize(src).unwrap();
        let token_kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();

        assert_eq!(token_kinds[0], &TokenKind::Fn);
        assert_eq!(token_kinds[1], &TokenKind::Ident("main".into()));
        assert_eq!(token_kinds[2], &TokenKind::LParen);
        assert_eq!(token_kinds[3], &TokenKind::RParen);
        assert_eq!(token_kinds[4], &TokenKind::LBrace);
        assert_eq!(token_kinds[5], &TokenKind::Let);
        assert_eq!(token_kinds[6], &TokenKind::Ident("x".into()));
        assert_eq!(token_kinds[7], &TokenKind::Colon);
        assert_eq!(token_kinds[8], &TokenKind::Ident("i64".into()));
        assert_eq!(token_kinds[9], &TokenKind::Eq);
        assert_eq!(token_kinds[10], &TokenKind::IntLiteral(42));
        assert_eq!(token_kinds[11], &TokenKind::Semicolon);
        // println is an identifier, ! is Bang
        assert_eq!(token_kinds[12], &TokenKind::Ident("println".into()));
        assert_eq!(token_kinds[13], &TokenKind::Bang);
        assert!(matches!(token_kinds.last(), Some(&&TokenKind::Eof)));
    }

    // === Error cases ===

    #[test]
    fn test_invalid_character() {
        let result = tokenize("let x = §;");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unexpected character"));
    }

    #[test]
    fn test_empty_source() {
        assert_eq!(kinds(""), vec![TokenKind::Eof]);
    }

    #[test]
    fn test_only_whitespace() {
        assert_eq!(kinds("   \n\n\t  "), vec![TokenKind::Eof]);
    }

    #[test]
    fn test_only_comments() {
        assert_eq!(kinds("// just a comment"), vec![TokenKind::Eof]);
        assert_eq!(kinds("/* block */"), vec![TokenKind::Eof]);
    }

    // === Edge cases ===

    #[test]
    fn test_range_vs_float() {
        // `0..10` should be int, dotdot, int — not float
        assert_eq!(
            kinds("0..10"),
            vec![
                TokenKind::IntLiteral(0),
                TokenKind::DotDot,
                TokenKind::IntLiteral(10),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_dot_dot_eq_range() {
        assert_eq!(
            kinds("0..=10"),
            vec![
                TokenKind::IntLiteral(0),
                TokenKind::DotDotEq,
                TokenKind::IntLiteral(10),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_negative_number_is_two_tokens() {
        // `-42` should be Minus + IntLiteral (parser handles unary minus)
        assert_eq!(
            kinds("-42"),
            vec![TokenKind::Minus, TokenKind::IntLiteral(42), TokenKind::Eof,]
        );
    }

    #[test]
    fn test_struct_definition() {
        assert_eq!(
            kinds("struct Point { x: f64, y: f64 }"),
            vec![
                TokenKind::Struct,
                TokenKind::Ident("Point".into()),
                TokenKind::LBrace,
                TokenKind::Ident("x".into()),
                TokenKind::Colon,
                TokenKind::Ident("f64".into()),
                TokenKind::Comma,
                TokenKind::Ident("y".into()),
                TokenKind::Colon,
                TokenKind::Ident("f64".into()),
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_closure_syntax() {
        assert_eq!(
            kinds("|x| x + 1"),
            vec![
                TokenKind::Pipe,
                TokenKind::Ident("x".into()),
                TokenKind::Pipe,
                TokenKind::Ident("x".into()),
                TokenKind::Plus,
                TokenKind::IntLiteral(1),
                TokenKind::Eof,
            ]
        );
    }
}
