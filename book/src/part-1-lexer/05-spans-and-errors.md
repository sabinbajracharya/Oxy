# Spans: How Error Messages Know Where You Went Wrong

<!-- OPUS_FILL
Write a 2-paragraph hook.
The core observation: the difference between a good error message ("line 7, column 3: unexpected '§'")
and a useless one ("error: unexpected character") is just whether the compiler tracked its position.
Spans cost almost nothing — a few extra integers per token. The payoff is enormous.
Frame it as: spans are the difference between a compiler that helps you and one that abandons you.
-->

## The `Span` type

Every token carries a `Span` — four integers describing where in the source the token came from:

```rust
// crates/oxy-core/src/lexer/token.rs
pub struct Span {
    pub start: usize,   // byte offset where the token starts
    pub end: usize,     // byte offset where it ends (exclusive)
    pub line: usize,    // 1-based line number
    pub column: usize,  // 1-based column number
}
```

The `Token` struct pairs a kind with its span:

```rust
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
```

Every token knows exactly where it came from. This information flows through the entire
pipeline — the parser attaches spans to AST nodes, the type checker includes spans in type
errors, and the error formatter uses them to print the line and column.

## How spans are tracked

The `Lexer` struct maintains two position counters that it updates on every character it consumes:

```rust
pub struct Lexer<'src> {
    // ...
    line: usize,    // current line (1-based)
    column: usize,  // current column (1-based)
}
```

In the `advance()` method:

```rust
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
```

Every time we consume a character, we update the byte offset and, if the character was a
newline, reset the column to 1 and increment the line. Otherwise we just increment the column.

When a token is created, the span captures the `line` and `column` at the **start** of the token
(saved in `start_offset` before calling `advance()` for the first character of the token):

```rust
fn make_token(&self, kind: TokenKind, start_offset: usize) -> Token {
    let (line, col) = self.line_col_at(start_offset);
    Token::new(kind, Span::new(start_offset, self.byte_offset, line, col))
}
```

## Why `start_line`/`start_col` matters for multi-character tokens

Consider scanning an unterminated string:

```
let s = "this string never ends
```

The lexer starts scanning the string at the `"`, records `start_line = 1, start_col = 9`.
Then it scans forward, advancing through every character in the source. By the time it
hits EOF, `self.line` and `self.column` point to the end of the file — which is not
where the error is.

The error message correctly points at line 1, column 9 (the opening quote) because
`start_line` and `start_col` were saved before scanning began:

```rust
fn scan_string(&mut self, _start_offset: usize) -> Result<TokenKind, PipelineError> {
    let start_line = self.line;    // save position at opening "
    let start_col = self.column;
    // ...
    if self.is_at_end() {
        return Err(PipelineError::Lexer {
            message: "unterminated string literal".into(),
            line: start_line,     // use saved position, not current position
            column: start_col - 1,
        });
    }
}
```

This pattern — save position at the start of a potentially long scan — appears whenever
the scanner might advance far before discovering an error.

## Spans through the pipeline

Spans do not disappear after the lexer. They are attached to AST nodes in the parser:

```rust
// Example AST node with span
struct LetStmt {
    name: String,
    value: Expr,
    span: Span,   // where this let statement appeared in source
}
```

And they appear in type errors:

```
error[E0]: type mismatch
  --> examples/my_program.ox:12:5
   |
12 |     let x: String = 42;
   |     ^^^^^^^^^^^^^^^^^^^
   |     expected String, found int
```

The `12:5` comes from the span on the `let` statement. The underline `^^^` uses the span's
`start` and `end` byte offsets to underline exactly the right range.

## The cost of spans

Four `usize` integers per token. On a 64-bit system, that's 32 bytes per token. A typical
Oxy source file might have a few thousand tokens — that's a few hundred kilobytes at most.

The benefit: every error message in the entire pipeline can point exactly at the source location.
The cost: negligible.

If Oxy had been built without spans, adding them later would require threading position
information through every data structure in the pipeline. This is why spans are built into
the token from the very beginning — they are much cheaper to add at the start than to retrofit.

## Testing span tracking

The lexer tests verify that spans are correct:

```rust
// crates/oxy-core/src/lexer/mod.rs (tests)
fn test_span_tracking() {
    let tokens = tokenize("let x = 42;").unwrap();
    // 'let' starts at line 1, col 1
    assert_eq!(tokens[0].span.line, 1);
    assert_eq!(tokens[0].span.column, 1);
    // 'x' starts at line 1, col 5
    assert_eq!(tokens[1].span.line, 1);
    assert_eq!(tokens[1].span.column, 5);
}

fn test_multiline_span() {
    let tokens = tokenize("let x\nlet y").unwrap();
    assert_eq!(tokens[2].span.line, 2); // second 'let' is on line 2
}
```

If you change how `advance()` updates line/column, these tests will break. They are the
specification of what "correct span tracking" means.
