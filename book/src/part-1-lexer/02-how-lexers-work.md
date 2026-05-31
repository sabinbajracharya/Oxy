# How Lexers Work

The mechanism behind a lexer is almost embarrassingly simple once you see it: it walks through
the source text one character at a time, left to right, and it never goes back. There's no
rewinding, no second pass, no "let me reconsider that paragraph." It looks at the character under
the cursor — and, when it has to, the one right after it — decides what kind of token is starting,
scans forward until that token ends, emits it, and picks up where it left off. Then it does that
again, and again, until it runs out of input.

If that feels familiar, it's because it's exactly what you do when you read this sentence aloud.
Your eyes move forward across the letters; your brain quietly groups them into words and
punctuation; and you do not re-read the letter `t` once you've already passed it on your way to
the next word. A lexer reads the way you read — forward only, grouping as it goes. That's it. Now
let's see how Oxy does it.

## One character at a time

The lexer is a loop. Each iteration scans one token. When it's done, it loops again.
When it hits EOF, it emits `Eof` and stops.

```rust
// crates/oxy-core/src/lexer/mod.rs
pub fn tokenize(mut self) -> Result<Vec<Token>, PipelineError> {
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
```

Each call to `next_token()` scans exactly one token and advances the position past it.
The `?` on `self.next_token()?` means: if `next_token` returns an error (unrecognized
character, unterminated string, etc.), stop immediately and return that error. We will cover
the `?` operator in Part 4 when we discuss error handling.

## The state the lexer tracks

The `Lexer` struct holds just four pieces of mutable state:

```rust
// crates/oxy-core/src/lexer/mod.rs
pub struct Lexer<'src> {
    source: &'src str,
    chars: Vec<char>,
    pos: usize,         // current position in the char array
    byte_offset: usize, // current byte offset in the source
    line: usize,        // current line (1-based)
    column: usize,      // current column (1-based)
}
```

`pos` and `byte_offset` are separate because Oxy source can contain Unicode characters —
a single Unicode character might be more than one byte, so byte offset ≠ char offset.
`line` and `column` are updated as we advance, tracking where we are for span generation.

The `'src` lifetime annotation is Rust's way of saying "this struct borrows from `source`
and cannot outlive it." We explain lifetimes in the Rust chapter for this part.

## The main dispatch: match on first character

The core of `next_token` is a `match` on the first character of the current token:

```rust
// crates/oxy-core/src/lexer/mod.rs (simplified)
let ch = self.advance();

let kind = match ch {
    '(' => TokenKind::LParen,
    ')' => TokenKind::RParen,
    '{' => TokenKind::LBrace,
    '}' => TokenKind::RBrace,
    ':' => {
        if self.match_char(':') {
            TokenKind::ColonColon   // ::
        } else {
            TokenKind::Colon        // :
        }
    }
    '=' => {
        if self.match_char('=') {
            TokenKind::EqEq         // ==
        } else if self.match_char('>') {
            TokenKind::FatArrow     // =>
        } else {
            TokenKind::Eq           // =
        }
    }
    '"' => self.scan_string(start_offset)?,
    c if c.is_ascii_digit() => self.scan_number(c, start_offset)?,
    c if c == '_' || c.is_alphabetic() => self.scan_identifier(c, start_offset),
    other => return Err(/* unexpected character error */),
};
```

This is the entire dispatch strategy:
- Single-character tokens (`(`, `)`, `{`, etc.) → emit immediately
- Two-character tokens (`==`, `::`, `->`, etc.) → peek at next character with `match_char`
- Strings → hand off to `scan_string`
- Numbers → hand off to `scan_number`
- Identifiers and keywords → hand off to `scan_identifier`
- Anything else → error

## How `match_char` works: lookahead without going back

Multi-character operators like `==` and `::` require looking at the character *after* the
current one. Oxy handles this with `match_char`:

```rust
// conceptual — consumes next char only if it matches
fn match_char(&mut self, expected: char) -> bool {
    if self.is_at_end() || self.chars[self.pos] != expected {
        return false;
    }
    self.advance();  // consume the character
    true
}
```

If the next character matches, it is consumed and we return `true`. If it doesn't match,
we leave the position unchanged and return `false`. The caller then decides which token
to emit based on the result.

This is "one character of lookahead" — the standard for most languages. Oxy needs at most
two characters of lookahead for `..=` (dot-dot-equals):

```rust
'.' => {
    if self.match_char('.') {    // second dot?
        if self.match_char('=') {
            TokenKind::DotDotEq  // ..=
        } else {
            TokenKind::DotDot    // ..
        }
    } else {
        TokenKind::Dot           // .
    }
}
```

## How identifiers become keywords

When `scan_identifier` scans a word like `fn` or `let`, it collects all alphanumeric
and underscore characters into a `String`, then checks whether that string is a keyword:

```rust
fn scan_identifier(&mut self, first: char, start_offset: usize) -> TokenKind {
    let mut name = String::from(first);
    while !self.is_at_end()
        && (self.peek().is_alphanumeric() || self.peek() == '_')
    {
        name.push(self.advance());
    }

    // Check if it's a keyword, otherwise it's an identifier
    TokenKind::from_keyword(&name).unwrap_or(TokenKind::Ident(name))
}
```

`TokenKind::from_keyword` is a single `match` over all keyword strings. If the string
matches a keyword, it returns `Some(keyword_variant)`. Otherwise `None`, and we fall back
to `Ident(name)`.

This is the moment where `fn` becomes `Fn` and `let` becomes `Let`. Once it happens, the
rest of the pipeline never has to check strings again — it works with enum variants.

## Skipping whitespace and comments

Before each token, the lexer skips whitespace (`' '`, `'\t'`, `'\n'`, `'\r'`) and comments.
Oxy supports two comment styles:

```
// single-line comment (everything until newline)
/* block comment (can span multiple lines) */
```

Block comments are handled by counting nesting depth — `/* /* nested */ */` is valid.
When a block comment is opened, the lexer scans forward until the depth returns to zero,
updating line/column counts as it goes.

## What happens on an error

If the lexer encounters a character it doesn't recognize — say `§` or `@` — it returns
an error:

```rust
other => {
    return Err(PipelineError::Lexer {
        message: format!("unexpected character '{other}'"),
        line: self.line,
        column: self.column - 1,
    });
}
```

The error includes the line and column. This is why the span tracking matters: without it,
all you could say is "unexpected character somewhere in your 500-line file." With it, you say
"line 7, column 12."

The entire `tokenize()` call returns `Result<Vec<Token>, PipelineError>` — either a complete
token list, or the first error encountered. The lexer stops at the first error. This is a
deliberate simplicity choice: error recovery in lexers is complex and Oxy does not attempt it.
