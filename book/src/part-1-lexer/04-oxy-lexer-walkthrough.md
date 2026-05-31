# Oxy's Lexer: A Full Walkthrough

<!-- OPUS_FILL
Write a 1-paragraph intro framing this chapter.
"We know what tokens are and how lexers work in theory. Now let's open the actual file."
Tone: direct, no fluff. Just set up that we're going to walk through the real code.
-->

**Files we're reading:**
- `crates/oxy-core/src/lexer/token.rs` — token and span types
- `crates/oxy-core/src/lexer/mod.rs` — the lexer implementation

Open both files now. This chapter is meant to be read alongside them.

---

## The public API: one function

The entire lexer's public API is one function:

```rust
// crates/oxy-core/src/lexer/mod.rs:919
pub fn tokenize(source: &str) -> Result<Vec<Token>, PipelineError> {
    Lexer::new(source).tokenize()
}
```

Call it with a source string, get back either a `Vec<Token>` or an error. Everything else
is private implementation detail. The pipeline calls `tokenize()` and never touches the
`Lexer` struct directly.

## Step 1: `Lexer::new` — setup

```rust
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
```

The first thing `new` does is convert the source string into a `Vec<char>`. This is a
deliberate trade-off: collecting into a `Vec<char>` uses more memory than iterating
character-by-character, but it gives O(1) random access by index — useful for lookahead
(`self.peek_at(1)`) and for the `chars[pos]` access pattern throughout the scanner.

`pos` indexes into `chars`. `byte_offset` tracks the byte position in the original UTF-8
string (needed for span calculation). Both start at 0. `line` and `column` start at 1
(they are 1-based for human-readable error messages).

## Step 2: `tokenize` — the main loop

```rust
pub fn tokenize(mut self) -> Result<Vec<Token>, PipelineError> {
    let mut tokens = Vec::new();
    loop {
        let token = self.next_token()?;
        let is_eof = token.kind == TokenKind::Eof;
        tokens.push(token);
        if is_eof { break; }
    }
    Ok(tokens)
}
```

The loop runs until `next_token` returns an `Eof` token. The `?` operator propagates any
error immediately. At the end, `tokens` is a `Vec` containing every token including the
terminal `Eof`.

## Step 3: `next_token` — scan one token

This is the heart of the lexer. It:

1. Skips whitespace and comments
2. Records the start position
3. Advances one character
4. Dispatches on that character to determine token kind
5. Constructs and returns a `Token` with the kind and span

The dispatch pattern is a large `match`:

```rust
let ch = self.advance();
let kind = match ch {
    // Single-character delimiters — emit immediately
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

    // Two-character operators — peek at next
    ':' => if self.match_char(':') { ColonColon } else { Colon },
    '.' => { /* Dot, DotDot, or DotDotEq */ }
    '+' => if self.match_char('=') { PlusEq } else { Plus },
    '-' => { /* Minus, MinusEq, or Arrow */ }
    '*' => if self.match_char('=') { StarEq } else { Star },
    '/' => if self.match_char('=') { SlashEq } else { Slash },
    '%' => if self.match_char('=') { PercentEq } else { Percent },
    '=' => { /* Eq, EqEq, or FatArrow */ }
    '!' => if self.match_char('=') { BangEq } else { Bang },
    '<' => { /* Lt, LtEq, or Shl */ }
    '>' => { /* Gt, GtEq, or Shr */ }
    '&' => if self.match_char('&') { AmpAmp } else { Amp },
    '|' => if self.match_char('|') { PipePipe } else { Pipe },

    // Delegated scanning
    '"'  => self.scan_string(start_offset)?,
    '\'' => { /* char literal or label */ }
    c if c.is_ascii_digit() => self.scan_number(c, start_offset)?,
    c if c == '_' || c.is_alphabetic() => self.scan_identifier(c, start_offset),

    other => return Err(/* unexpected character */),
};
```

The actual file at `crates/oxy-core/src/lexer/mod.rs` lines 65–264 contains the full version.
The structure shown here is complete — this is genuinely all the dispatch logic.

## Step 4: `scan_identifier` — words become keywords

```rust
fn scan_identifier(&mut self, first: char, _start_offset: usize) -> TokenKind {
    let mut name = String::from(first);
    while !self.is_at_end()
        && (self.peek().is_alphanumeric() || self.peek() == '_')
    {
        name.push(self.advance());
    }
    TokenKind::from_keyword(&name).unwrap_or(TokenKind::Ident(name))
}
```

Collect characters while they are alphanumeric or `_`. Then call `from_keyword`. If it's
a keyword, return the keyword variant. Otherwise, return `Ident(name)`.

`from_keyword` is a `match` over every keyword string in the language — 34 keywords total.
It is the only place in the entire codebase where keyword strings appear as string literals.
Everywhere else, the code works with typed enum variants.

## Step 5: `scan_number` — integers and floats

Numbers need more logic than identifiers because Oxy supports multiple literal forms:
- Decimal: `42`, `1_000` (underscores as visual separators)
- Hex: `0xFF`, `0x1A_2B`
- Binary: `0b1010`
- Octal: `0o755`
- Float: `3.14`, `1e10`, `2.5E-3`

The scanner peeks at the character after `0` to decide the base. A `.` after digits
switches to float scanning unless followed by another `.` (which would be `0..10` — a range,
not a float). This edge case — `0..10` must be `IntLiteral(0), DotDot, IntLiteral(10)`,
not `FloatLiteral(0.)` — is explicitly tested:

```rust
// crates/oxy-core/src/lexer/mod.rs (tests)
fn test_range_vs_float() {
    assert_eq!(
        kinds("0..10"),
        vec![IntLiteral(0), DotDot, IntLiteral(10), Eof]
    );
}
```

The parse function at the end:

```rust
fn parse_int_literal(digits: &str, radix: u32) -> Result<i64, ()> {
    if let Ok(v) = i64::from_str_radix(digits, radix) {
        return Ok(v);
    }
    // For patterns like 0xFFFFFFFFFFFFFFFF — the bit pattern is valid
    // even though it overflows i64, so try u64 and reinterpret
    u64::from_str_radix(digits, radix)
        .map(|v| v as i64)
        .map_err(|_| ())
}
```

Notice the `as i64` cast from `u64` — this is a deliberate bitwise reinterpretation for
large hex literals where the bit pattern is what matters, not the sign.

## Step 6: `scan_string` — escape sequences

String scanning loops until it finds the closing `"`, handling escape sequences along the way:

```
\n  → newline
\t  → tab
\r  → carriage return
\\  → backslash
\"  → double quote
\'  → single quote
\0  → null byte
\xHH     → hex byte (two hex digits)
\u{HHHH} → Unicode codepoint
```

If the string is not terminated before end-of-file, the lexer returns an error with the
line and column of the opening `"`. This is why the scanner records `start_line`/`start_col`
at the beginning of string scanning — by the time EOF is hit, the position has moved far away.

## Step 7: `scan_fstring` — f-strings

F-strings (`f"Hello {name}!"`) are scanned similarly to strings, but the content is stored
raw with the `{expr}` interpolation parts intact:

```rust
FStringLiteral(String)  // stores "Hello {name}!" verbatim
```

The interpolation is resolved at the **parser level**, not the lexer level. The lexer's job
is just to recognize the `f"..."` form and hand the raw content to the parser. The parser
then splits the string at `{` and `}` markers and constructs the appropriate AST nodes.

This is another example of the separation of concerns: the lexer recognizes syntax; meaning
comes later.

## Tracing the lexer: a complete example

Let's trace `fn add(a: int, b: int) -> int { a + b }` through the lexer:

| Character(s) | Result |
|-------------|--------|
| `fn` | keyword scan → `Fn` |
| ` ` | whitespace skip |
| `add` | identifier scan → `Ident("add")` |
| `(` | single-char → `LParen` |
| `a` | identifier scan → `Ident("a")` |
| `:` | no `::` follows → `Colon` |
| ` int` | identifier scan → `Ident("int")` |
| `,` | single-char → `Comma` |
| ` b` | whitespace skip, then `Ident("b")` |
| `: int` | `Colon`, `Ident("int")` |
| `)` | `RParen` |
| ` ` | whitespace skip |
| `-` then `>` | two-char → `Arrow` |
| ` int` | `Ident("int")` |
| ` ` | whitespace skip |
| `{` | `LBrace` |
| ` a` | whitespace skip, `Ident("a")` |
| ` ` | whitespace skip |
| `+` | no `=` follows → `Plus` |
| ` b` | whitespace skip, `Ident("b")` |
| ` ` | whitespace skip |
| `}` | `RBrace` |
| EOF | `Eof` |

Notice: `int` is tokenized as `Ident("int")`, not as a special keyword token. In Oxy, built-in
type names like `int`, `float`, `byte`, `String`, `Vec` are not keywords — they are identifiers
that the type checker recognizes. The lexer does not know about types.

## What the lexer does not know

The lexer has no idea:
- What `fn` means (the parser will figure that out)
- That `int` is a type (the type checker will figure that out)
- Whether `add(a, b)` is a valid call (the parser and type checker handle that)
- Whether `a + b` type-checks (the type checker handles that)

The lexer knows one thing: how to chop source text into labeled pieces with correct positions.
Everything else is someone else's problem.
