# Mission

Oxy exists because Rust's syntax is a joy but its borrow checker is a gatekeeper.

Millions of developers want sum types, pattern matching, traits, and expression-oriented code — but can't or won't invest the weeks it takes to internalize ownership, lifetimes, and borrowing. Oxy gives them the language they want without the fights they don't.

## What Oxy is

- **A fast, interpreted language with Rust's syntax and semantics** — structs, enums, pattern matching, traits, generics, closures, iterators, async/await. Everything you reach for in Rust, running as a script.
- **A learning bridge** — master Rust's type system and idioms in a forgiving environment, then transition to Rust when you need zero-cost abstractions.
- **A productive scripting tool** — replace Bash, Python, and Node.js scripts with a language that feels like home for Rust developers.

## What Oxy is not

- **Not a Rust replacement** — we don't target systems programming, embedded, or kernel development. Use real Rust for that.
- **Not a borrow checker workaround** — we don't simulate ownership. Values are reference-counted. `&` and `&mut` are accepted syntax but have no semantic meaning.
- **Not a research language** — we prioritize pragmatism over novelty. Every feature has a Rust analogue.

## Guiding principles

1. **Syntax fidelity** — if it's valid Rust syntax, it should be valid Oxy syntax. Deviations are bugs.
2. **Fast feedback** — parse, check, run. No compile step. The REPL is a first-class feature.
3. **Gradual capability** — optional type annotations. Start dynamic, add types as your design solidifies.
4. **Batteries included** — HTTP client, HTTP server, SQLite, JSON, regex, file I/O, networking. One binary.
5. **Rust-native tooling** — written in Rust, distributed as a single binary, extensible in Rust.
