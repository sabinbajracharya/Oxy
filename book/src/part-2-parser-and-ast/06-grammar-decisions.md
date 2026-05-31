# Grammar Decisions: Why Oxy Looks Like Rust

Oxy looks like Rust because that resemblance is the entire pitch: it's "dynamic Rust," Rust's
syntax with Rust's ownership model surgically removed. But that goal creates a constant tension at
the grammar level, because a lot of Rust's syntax exists *specifically* to serve the borrow
checker. So every syntactic decision in Oxy came down to one question: does this notation help a
human read the program, or does it exist only to feed the ownership machinery? Keep the former,
banish the latter.

And we do mean banished — on principle, not by accident. Reference syntax (`&T`, `&mut self`,
`&str`) isn't quietly accepted-and-ignored; the parser rejects it outright with a fix-it, because
a `&` with no borrow checker behind it is a lie about what the program does. The integer width zoo
(`i8` through `usize`) got the same treatment: Oxy has exactly three numeric types — `int`, `byte`,
`float` — and the type checker steers you back to them. These weren't oversights or
not-yet-implemented gaps. They were calls, made deliberately, and the rest of this chapter walks
through them.

Because in the end, syntax is never just decoration — it's a statement about what a language thinks
matters. Rust's surface says *safety comes from tracking ownership*. Oxy keeps the same surface but
changes the sentence underneath it to: *safety comes from types, and ownership is the runtime's
problem, not yours.*

## What Oxy kept from Rust

Almost everything visible in source code:

- `fn`, `struct`, `enum`, `impl`, `trait`, `mod`, `use` — all Rust keywords, all present
- `let`, `let mut`, `const`, `static` — same semantics as Rust (mutability of bindings)
- `if let`, `while let`, `match` with pattern matching — all supported
- `impl Trait for Type` — trait implementations work
- `Vec<T>`, `Option<T>`, `Result<T, E>`, `HashMap<K, V>` — standard collection types
- `async`, `await`, `spawn` — async programming model
- `pub`, `pub(crate)`, `pub(super)` — same visibility rules
- Operator precedence — identical to Rust (avoids surprise for Rust readers)
- `?` operator — early return on `Err`/`None`, same as Rust
- F-strings (`f"hello {name}!"`) — added as a convenience Rust lacks
- Generic parameters (`<T: Display + Clone>`) — same syntax

## What Oxy dropped

**Reference syntax** — `&T`, `&mut T`, `&self`, `&mut self`, `&str`, `&[T]`, `&expr`.

The parser actively rejects these with a fix-it error message. They are not "accepted but
ignored" — they are compile errors. If you write `fn foo(x: &str)`, Oxy tells you:
"Oxy does not have reference types. Use `String` instead."

Why? References exist to support the borrow checker. Without the borrow checker, references
have no semantic content. Allowing `&T` would just confuse readers who expect ownership
semantics.

**Lifetimes** — `'a`, `<'a: 'b>`. Not parsed. No `'static` lifetime. The parser errors on
lifetime syntax. Same reason: lifetimes annotate reference validity regions. No references,
no lifetimes.

**The integer width zoo** — `i8`, `i16`, `i32`, `i64`, `u16`, `u32`, `u64`, `isize`, `usize`.
The type checker rejects all of them with a fix-it suggesting `int`. The only integer types
are `int` (64-bit signed), `byte` (8-bit unsigned), and `float` (64-bit IEEE-754).

This was a deliberate language design decision. Most code does not need specific widths.
When it does — binary protocol parsing, graphics buffers — you can use `as byte` casts.
The three-type system eliminates an entire category of "which int type should I use?" questions.

**Slice types** — `[T]` as a parameter type. Use `Vec<T>` instead.

## The `mut` asymmetry

In Rust, mutability applies to both bindings and references:
- `let mut x` — the binding is mutable (can be reassigned)
- `&mut x` — the reference allows mutation through it

In Oxy, only binding mutability exists:
- `let mut x` — x can be reassigned
- `mut self` in methods — self can be reassigned within the method body
- `mut param: T` — the parameter binding can be reassigned

This is the same as `let`/`const` in JavaScript or `final`/`var` in Kotlin. It controls
rebinding, not aliasing. There is no aliasing concept in Oxy because there are no references.

## Why the same syntax but different semantics?

Oxy is explicitly targeting Rust programmers who want to prototype quickly or build tools
without ownership overhead. Keeping the syntax identical means:
- Rust developers can read Oxy code immediately
- Concepts learned in Oxy transfer to Rust (loops, match, traits, generics)
- Existing Rust tooling (syntax highlighters, formatters) mostly works on Oxy files

The semantic difference — no ownership, no borrowing — is handled at runtime (garbage
collection) rather than compile time. The surface stays familiar; the safety model changes.

## The `no_struct_literal` disambiguation (revisited)

One grammar decision worth highlighting: `if score { }` is ambiguous. Does `score {` start
a struct initializer or is `{` the start of the if-body?

Rust resolves this by disallowing struct literals in expression positions where a block
could follow (`if`, `while`, `for` headers, `match` scrutinees). Oxy copies this rule
exactly via the `no_struct_literal` context flag in the parser.

This is the kind of grammar decision that sounds academic until you've spent two hours
debugging why `if point { ... }` doesn't parse and `if (point) { ... }` does.
