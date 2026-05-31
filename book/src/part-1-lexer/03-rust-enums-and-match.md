# Rust Concepts: Enums and Match

If the word "enum" makes you think of a list of named constants — `RED`, `GREEN`, `BLUE`, each
secretly just an integer — then Rust is about to expand your definition. A Rust enum is that, but
it is also enormously more: each variant can carry its own payload, and different variants can
carry completely different payloads. One variant holds nothing. Another holds a string. Another
holds two numbers and a nested struct. They're all the same type, and at any moment a value of
that type is exactly one of them. This is the algebraic data type, and the first time it clicks,
the reaction is usually some version of *oh — enums can do that?*

Hold onto that reaction, because this single feature is the structural backbone of the entire
compiler. Tokens are an enum. AST nodes are an enum. The type checker's notion of "what type is
this" is an enum. The IR's instructions are an enum. Runtime values are an enum. Over and over,
the same move: define one variant per case, let each variant carry exactly the data that case
needs, and then use `match` to take them apart again with the compiler enforcing that you handled
every one. Learn this pattern here, in the lexer, where it's simplest — you'll be using it on
every page from now on.

## What a Rust enum is

In many languages, an `enum` is just a named integer:

```java
// Java
enum Direction { North, South, East, West }
```

In Rust, each variant of an enum can carry completely different data:

```rust
enum TokenKind {
    // No data — just a label
    Plus,
    Minus,
    Eof,

    // Carries a String
    Ident(String),

    // Carries an i64 and an IntegerSuffix
    IntLiteral(i64, IntegerSuffix),

    // Carries a String (the literal content)
    StringLiteral(String),
}
```

This is called an **algebraic data type** (or "tagged union"). At runtime, a `TokenKind`
value is exactly one of these variants — and if it's `Ident`, it contains a `String`; if
it's `IntLiteral`, it contains both an `i64` and an `IntegerSuffix`.

This is the entire `TokenKind` in Oxy. The lexer creates values of this type; the parser
reads them.

## How you read an enum value: `match`

To extract data from an enum, Rust uses `match` — exhaustive pattern matching:

```rust
fn describe_token(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Plus => "the + operator".to_string(),
        TokenKind::Ident(name) => format!("the identifier '{name}'"),
        TokenKind::IntLiteral(n, _) => format!("the integer {n}"),
        TokenKind::Eof => "end of file".to_string(),
        // ... every variant must be handled
    }
}
```

Three things to note:

1. **Exhaustive**: `match` must cover every variant. If you add a new `TokenKind::Star`
   variant and forget to add it to every `match` in the codebase, the Rust compiler rejects
   the program. This is a compile-time safety guarantee — it's how Oxy's "exhaustive match"
   divergence guard works (more on that in Part 8).

2. **Binding**: When you match `TokenKind::Ident(name)`, the variable `name` is bound to the
   `String` inside the variant. You can use it in the match arm body. The `_` in
   `IntLiteral(n, _)` means "I don't care about the second field."

3. **No `switch` fallthrough**: Each arm is independent. There is no implicit fallthrough to
   the next arm.

## The wildcard: `_`

If you want to handle "everything else" without listing every variant:

```rust
match kind {
    TokenKind::Fn => parse_function(),
    TokenKind::Struct => parse_struct(),
    _ => parse_expression(),   // handles all other variants
}
```

The `_` wildcard matches any variant not already listed. It is the equivalent of a `default:`
case in C's `switch`.

**Oxy's divergence guard deliberately avoids `_` in certain matches.** In `vm/interp.rs`,
the match over `IrOp` has no wildcard arm. That means adding a new `IrOp` variant to the
IR breaks the interpreter's build immediately — you cannot forget to handle it. We cover
this in detail in Part 8.

## `if let`: matching one variant

When you only care about one specific variant, `match` with 10 arms is verbose. Rust has `if let`:

```rust
// Instead of:
match kind {
    TokenKind::Ident(name) => println!("got ident: {name}"),
    _ => {} // ignore everything else
}

// You can write:
if let TokenKind::Ident(name) = kind {
    println!("got ident: {name}");
}
```

This is used heavily in the parser when checking for a specific token type.

## Deriving traits: `Debug`, `Clone`, `PartialEq`

In the Oxy source you will see:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind { ... }
```

These `#[derive(...)]` annotations auto-generate common functionality:

| Derived trait | What it generates |
|---------------|-------------------|
| `Debug` | A `{:?}` formatter for printing the enum in tests and error messages |
| `Clone` | A `.clone()` method to make a deep copy |
| `PartialEq` | The `==` and `!=` operators for comparing two values |

Without `PartialEq`, you could not write `token.kind == TokenKind::Eof`. The derive
attribute generates the comparison code so you do not have to write it by hand.

## Enums throughout the compiler

Enums are not unique to the lexer. They appear at every layer:

| Layer | Enum | Represents |
|-------|------|-----------|
| Lexer | `TokenKind` | Each kind of token |
| Parser | `Expr`, `Stmt`, `Item` | AST node types |
| Type checker | `TypeInfo` | Type of an expression |
| IR gen | `IrOp`, `Terminator` | IR instruction types |
| Runtime | `Value` | A runtime value (int, string, struct, etc.) |

In every case, the pattern is the same: define an enum with one variant per case, use `match`
to dispatch on it exhaustively. This is the backbone of the entire compiler.
