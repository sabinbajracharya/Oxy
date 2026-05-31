# What Is a Token?

Here is the most important thing to understand about a lexer before you understand anything else
about it: it has no idea what your code means. None. It does not know that `fn` introduces a
function or that `+` adds numbers. It cannot tell a variable name from a typo. If you handed it a
program that was complete gibberish in every semantic sense but used only legal characters, it
would happily chop it up and hand back a tidy list of pieces without the slightest complaint.

That sounds like a weakness. It is actually the whole trick. The lexer is a surgeon with scissors
and a roll of sticky labels, not a doctor who understands anatomy. Its entire job is to look at a
flat stream of characters and cut it into chunks, slapping a label on each one — *this chunk is a
keyword, this one is a name, this one is a number, this one is an open paren* — and then to hand
that labeled pile to the next stage. Meaning is somebody else's problem. The parser will worry
about structure; the type checker will worry about correctness. The lexer worries about exactly
one thing, does it fast, and stops.

This is the first instance of a pattern you'll see at every layer of the compiler: each stage does
one small job and refuses to do anyone else's. The separation isn't a limitation we're apologizing
for — it's the reason the whole thing stays comprehensible. So what is a token, exactly?

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
