//! Token types and source span definitions for the Oxide lexer.

/// Source span for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Byte offset of the start of the span.
    pub start: usize,
    /// Byte offset of the end of the span (exclusive).
    pub end: usize,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number.
    pub column: usize,
}

impl Span {
    /// Creates a new span from byte offsets and a 1-based line/column position.
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// A token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    /// Creates a new token with the given kind and source span.
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// All possible token kinds in the Oxide language.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // === Literals ===
    /// Integer literal, e.g. `42`, `0xFF`, `0b1010`, `1_000`
    IntLiteral(i64),
    /// Float literal, e.g. `3.14`, `1e10`, `2.5f64`
    FloatLiteral(f64),
    /// String literal, e.g. `"hello"`
    StringLiteral(String),
    /// Character literal, e.g. `'a'`
    CharLiteral(char),
    /// F-string literal, e.g. `f"Hello {name}!"` — raw content with `{expr}` intact
    FStringLiteral(String),
    /// Boolean `true`
    True,
    /// Boolean `false`
    False,

    // === Identifier ===
    /// Any identifier, e.g. `foo`, `my_var`
    Ident(String),

    // === Keywords ===
    /// `let` — variable binding
    Let,
    /// `mut` — mutable binding modifier
    Mut,
    /// `fn` — function definition
    Fn,
    /// `return` — return from function
    Return,
    /// `if` — conditional branch
    If,
    /// `else` — alternative branch
    Else,
    /// `while` — conditional loop
    While,
    /// `loop` — infinite loop
    Loop,
    /// `for` — iterator loop
    For,
    /// `in` — iterator binding keyword
    In,
    /// `break` — exit a loop
    Break,
    /// `continue` — skip to next loop iteration
    Continue,
    /// `struct` — struct definition
    Struct,
    /// `enum` — enum definition
    Enum,
    /// `impl` — implementation block
    Impl,
    /// `trait` — trait definition
    Trait,
    /// `match` — pattern matching
    Match,
    /// `pub` — public visibility modifier
    Pub,
    /// `use` — import declaration
    Use,
    /// `mod` — module declaration
    Mod,
    /// `self` — current instance reference
    SelfLower,
    /// `Self` — current type reference
    SelfUpper,
    /// `as` — type casting
    As,
    /// `ref` — reference binding in patterns
    Ref,
    /// `const` — compile-time constant
    Const,
    /// `static` — static lifetime binding
    Static,
    /// `type` — type alias
    Type,
    /// `where` — generic constraint clause
    Where,
    /// `move` — move capture in closures
    Move,
    /// `async` — asynchronous function modifier
    Async,
    /// `await` — await an asynchronous value
    Await,
    /// `dyn` — dynamic dispatch trait object
    Dyn,
    /// `super` — parent module reference
    Super,
    /// `crate` — crate root reference
    Crate,

    // === Operators ===
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `==`
    EqEq,
    /// `!=`
    BangEq,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    LtEq,
    /// `>=`
    GtEq,
    /// `&&`
    AmpAmp,
    /// `||`
    PipePipe,
    /// `!`
    Bang,
    /// `&`
    Amp,
    /// `|`
    Pipe,
    /// `^`
    Caret,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
    /// `=`
    Eq,
    /// `+=`
    PlusEq,
    /// `-=`
    MinusEq,
    /// `*=`
    StarEq,
    /// `/=`
    SlashEq,
    /// `%=`
    PercentEq,
    /// `->`
    Arrow,
    /// `=>`
    FatArrow,
    /// `::`
    ColonColon,
    /// `..`
    DotDot,
    /// `..=`
    DotDotEq,
    /// `?`
    Question,

    // === Delimiters ===
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `,`
    Comma,
    /// `;`
    Semicolon,
    /// `:`
    Colon,
    /// `.`
    Dot,
    /// `#`
    Hash,
    /// `_`
    Underscore,

    // === Special ===
    /// End of file
    Eof,
}

impl TokenKind {
    /// Returns the keyword `TokenKind` for a given string, or `None` if not a keyword.
    pub fn from_keyword(word: &str) -> Option<Self> {
        match word {
            "let" => Some(Self::Let),
            "mut" => Some(Self::Mut),
            "fn" => Some(Self::Fn),
            "return" => Some(Self::Return),
            "if" => Some(Self::If),
            "else" => Some(Self::Else),
            "while" => Some(Self::While),
            "loop" => Some(Self::Loop),
            "for" => Some(Self::For),
            "in" => Some(Self::In),
            "break" => Some(Self::Break),
            "continue" => Some(Self::Continue),
            "struct" => Some(Self::Struct),
            "enum" => Some(Self::Enum),
            "impl" => Some(Self::Impl),
            "trait" => Some(Self::Trait),
            "match" => Some(Self::Match),
            "pub" => Some(Self::Pub),
            "use" => Some(Self::Use),
            "mod" => Some(Self::Mod),
            "self" => Some(Self::SelfLower),
            "Self" => Some(Self::SelfUpper),
            "as" => Some(Self::As),
            "ref" => Some(Self::Ref),
            "const" => Some(Self::Const),
            "static" => Some(Self::Static),
            "type" => Some(Self::Type),
            "where" => Some(Self::Where),
            "move" => Some(Self::Move),
            "async" => Some(Self::Async),
            "await" => Some(Self::Await),
            "dyn" => Some(Self::Dyn),
            "true" => Some(Self::True),
            "false" => Some(Self::False),
            "super" => Some(Self::Super),
            "crate" => Some(Self::Crate),
            _ => None,
        }
    }

    /// Returns a human-readable description for error messages.
    pub fn description(&self) -> &'static str {
        match self {
            Self::IntLiteral(_) => "integer literal",
            Self::FloatLiteral(_) => "float literal",
            Self::StringLiteral(_) => "string literal",
            Self::CharLiteral(_) => "character literal",
            Self::FStringLiteral(_) => "f-string literal",
            Self::True => "'true'",
            Self::False => "'false'",
            Self::Ident(_) => "identifier",
            Self::Let => "'let'",
            Self::Mut => "'mut'",
            Self::Fn => "'fn'",
            Self::Return => "'return'",
            Self::If => "'if'",
            Self::Else => "'else'",
            Self::While => "'while'",
            Self::Loop => "'loop'",
            Self::For => "'for'",
            Self::In => "'in'",
            Self::Break => "'break'",
            Self::Continue => "'continue'",
            Self::Struct => "'struct'",
            Self::Enum => "'enum'",
            Self::Impl => "'impl'",
            Self::Trait => "'trait'",
            Self::Match => "'match'",
            Self::Pub => "'pub'",
            Self::Use => "'use'",
            Self::Mod => "'mod'",
            Self::SelfLower => "'self'",
            Self::SelfUpper => "'Self'",
            Self::As => "'as'",
            Self::Ref => "'ref'",
            Self::Const => "'const'",
            Self::Static => "'static'",
            Self::Type => "'type'",
            Self::Where => "'where'",
            Self::Move => "'move'",
            Self::Async => "'async'",
            Self::Await => "'await'",
            Self::Dyn => "'dyn'",
            Self::Super => "'super'",
            Self::Crate => "'crate'",
            Self::Plus => "'+'",
            Self::Minus => "'-'",
            Self::Star => "'*'",
            Self::Slash => "'/'",
            Self::Percent => "'%'",
            Self::EqEq => "'=='",
            Self::BangEq => "'!='",
            Self::Lt => "'<'",
            Self::Gt => "'>'",
            Self::LtEq => "'<='",
            Self::GtEq => "'>='",
            Self::AmpAmp => "'&&'",
            Self::PipePipe => "'||'",
            Self::Bang => "'!'",
            Self::Amp => "'&'",
            Self::Pipe => "'|'",
            Self::Caret => "'^'",
            Self::Shl => "'<<'",
            Self::Shr => "'>>'",
            Self::Eq => "'='",
            Self::PlusEq => "'+='",
            Self::MinusEq => "'-='",
            Self::StarEq => "'*='",
            Self::SlashEq => "'/='",
            Self::PercentEq => "'%='",
            Self::Arrow => "'->'",
            Self::FatArrow => "'=>'",
            Self::ColonColon => "'::'",
            Self::DotDot => "'..'",
            Self::DotDotEq => "'..='",
            Self::Question => "'?'",
            Self::LParen => "'('",
            Self::RParen => "')'",
            Self::LBrace => "'{'",
            Self::RBrace => "'}'",
            Self::LBracket => "'['",
            Self::RBracket => "']'",
            Self::Comma => "','",
            Self::Semicolon => "';'",
            Self::Colon => "':'",
            Self::Dot => "'.'",
            Self::Hash => "'#'",
            Self::Underscore => "'_'",
            Self::Eof => "end of file",
        }
    }
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IntLiteral(n) => write!(f, "{n}"),
            Self::FloatLiteral(n) => write!(f, "{n}"),
            Self::StringLiteral(s) => write!(f, "\"{s}\""),
            Self::CharLiteral(c) => write!(f, "'{c}'"),
            Self::FStringLiteral(s) => write!(f, "f\"{s}\""),
            Self::Ident(name) => write!(f, "{name}"),
            other => write!(f, "{}", other.description()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_lookup() {
        assert_eq!(TokenKind::from_keyword("let"), Some(TokenKind::Let));
        assert_eq!(TokenKind::from_keyword("fn"), Some(TokenKind::Fn));
        assert_eq!(TokenKind::from_keyword("self"), Some(TokenKind::SelfLower));
        assert_eq!(TokenKind::from_keyword("Self"), Some(TokenKind::SelfUpper));
        assert_eq!(TokenKind::from_keyword("true"), Some(TokenKind::True));
        assert_eq!(TokenKind::from_keyword("false"), Some(TokenKind::False));
        assert_eq!(TokenKind::from_keyword("notakeyword"), None);
        assert_eq!(TokenKind::from_keyword("LET"), None);
    }

    #[test]
    fn test_span_display() {
        let span = Span::new(0, 5, 1, 1);
        assert_eq!(format!("{span}"), "1:1");
    }

    #[test]
    fn test_token_kind_display() {
        assert_eq!(format!("{}", TokenKind::IntLiteral(42)), "42");
        assert_eq!(
            format!("{}", TokenKind::StringLiteral("hi".into())),
            "\"hi\""
        );
        assert_eq!(format!("{}", TokenKind::Plus), "'+'");
        assert_eq!(format!("{}", TokenKind::Let), "'let'");
    }
}
