# What Is a Token?

<!-- OPUS_FILL
Write a 2-3 paragraph hook. The reader is about to learn what a token is.

Set up the key insight: a lexer has absolutely no idea what the code MEANS.
It just chops it into labeled pieces. A surgeon with scissors and sticky labels,
not a doctor who understands anatomy.

The "aha!" moment to build toward: the lexer's job is not to understand — it's
to label. Understanding comes later (parser, type checker). This separation of
concerns is a feature, not a limitation.

End by transitioning to: "So what is a token, exactly?"
-->

## The definition

A **token** is a labeled piece of source text — the smallest unit that the parser cares about.

Every Oxy program starts as a string of characters. The lexer's job is to group those characters
into meaningful chunks and attach a label to each chunk. The result is a flat list of tokens.

For example, this Oxy program:

```
fn main() {
    let x = 42;
}
```

becomes this list of tokens:

```
Fn
Ident("main")
LParen
RParen
LBrace
Let
Ident("x")
Eq
IntLiteral(42)
Semicolon
RBrace
Eof
```

No structure. No nesting. No understanding of what `fn` means or what `main` does. Just a flat
sequence of labeled pieces. The parser will figure out the structure. The lexer just labels.

## The four categories of tokens in Oxy

Looking at `crates/oxy-core/src/lexer/token.rs`, Oxy tokens fall into four groups:

**Literals** — values embedded directly in source code:
```rust
IntLiteral(i64, IntegerSuffix),
FloatLiteral(f64, FloatSuffix),
StringLiteral(String),
CharLiteral(char),
FStringLiteral(String),
True,
False,
```

**Identifiers** — names the programmer chooses:
```rust
Ident(String),  // "main", "x", "my_function", etc.
```

**Keywords** — reserved names with fixed meanings:
```rust
Let, Mut, Fn, Return, If, Else, While, Loop, For, In, Break, Continue,
Struct, Enum, Impl, Trait, Match, Pub, Use, Mod, Async, Await, ...
```

**Punctuation and operators** — structural and arithmetic symbols:
```rust
Plus, Minus, Star, Slash, Eq, EqEq, LParen, RParen, LBrace, RBrace,
Arrow, FatArrow, ColonColon, ...
```

And exactly one special token to mark the end:
```rust
Eof
```

## Why keywords are not just identifiers

You might wonder: why is `fn` its own token variant (`Fn`) rather than just `Ident("fn")`?

The answer is convenience and safety. If keywords were identifiers, every parser rule that
expects a keyword would have to match `Ident("fn")` — a string comparison at every call site.
That is slow, error-prone, and hard to read.

By converting `fn` → `Fn` (a zero-cost enum variant) in the lexer, the parser can write:

```rust
match token.kind {
    TokenKind::Fn => { /* parse function */ }
    TokenKind::Struct => { /* parse struct */ }
    _ => { /* something else */ }
}
```

No string comparisons. No chance of typos like `"Fn"` vs `"fn"`. The conversion happens
exactly once, in `TokenKind::from_keyword()`, and everything downstream benefits.

## What a token carries

Every token in Oxy is a `Token` struct with two fields:

```rust
// crates/oxy-core/src/lexer/token.rs
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
```

The `kind` is the label. The `span` is where in the source this token came from:

```rust
pub struct Span {
    pub start: usize,   // byte offset where the token starts
    pub end: usize,     // byte offset where it ends (exclusive)
    pub line: usize,    // 1-based line number
    pub column: usize,  // 1-based column number
}
```

The span is how error messages know to say `error at line 3, column 7` instead of just
`error somewhere`. We cover spans in depth in chapter 5 of this part.

## A note on numeric types in Oxy

Notice that `IntLiteral` stores `i64` — not `i32`, not `u64`, not `isize`. Oxy has exactly
three numeric types: `int` (64-bit signed), `byte` (8-bit unsigned), and `float` (64-bit IEEE-754).
The lexer stores every integer literal as `i64` and every float literal as `f64`.

The `IntegerSuffix` and `FloatSuffix` enums exist in the source but each have only one variant
(`None`). They are placeholders from an earlier design that has since been retired — Oxy deliberately
avoids the Rust integer width zoo (`i8`, `i16`, `i32`, `u16`, etc.).
