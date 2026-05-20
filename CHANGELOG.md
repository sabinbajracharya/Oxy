# Changelog

## v0.3.0 — 2026-05-20

### Language Features

- **Generics**: generic structs (`struct Box<T>`), generic enums (`enum MyOption<T>`), generic functions with trait bounds, and turbofish syntax (`func::<Type>(args)`). Type erasure at runtime with AST-level monomorphization at turbofish call sites.
- **Traits**: trait definitions, `impl Trait for Type`, default methods with inheritance, multiple trait bounds, where clauses. Operator overloading via `impl Add/Sub/Mul/Div/Rem/Neg` for custom types. `#[derive(Default)]` support.
- **Closures**: mutable capture with Cell sharing, closure value passing, higher-order functions, nested closures, and closure-as-return-value.
- **Structs & Enums**: tuple struct constructors, unit struct values, `Path { fields }` init syntax, struct field patterns in match, `let _ = expr` discard.
- **Error handling**: `Option::or`/`or_else`, `Result::or_else`, `unwrap_or_else` on Result.
- **Control flow**: `return` as expression, `~` bitwise NOT operator.
- **Numbers**: 9 integer widths (i8-u64), 2 float widths (f32/f64), type suffixes, hex/octal/binary literals, width-aware arithmetic with wrapping.
- **Type system**: `fn(i64) -> i64` function type syntax, width-aware type checking, type aliases.
- **Modules**: `pub`, `pub(crate)`, `pub(super)`, `use as` renaming, `pub use module::*` glob re-exports, `self`/`super`/`crate` path resolution.

### Compiler & VM

- 100% bytecode execution (interpreter removed — 395 VM tests, 593 feature tests)
- AST-level monomorphization for generic functions with turbofish
- Trait-bound method resolution on generic type parameters
- Tuple struct and unit struct constructor resolution
- Cell write-through for mutable closure captures
- Trait method dispatch on built-in types

### Bug Fixes

- `StoreLocal` Cell write-through used `continue` causing repeated execution (closure capture corruption)
- `vm_rem`/`vm_div` panic on custom types (to_f64 called before numeric guard)
- `max_slot` undercount in `CallClosure`/`run_closure` frame setup
- `Neg` operator now dispatches through operator overloading for custom types

### Developer Tools

- `--dump-bytecode` CLI flag for bytecode disassembly
- `OXY_VM_TRACE=1` env var for per-opcode execution tracing
- `disassemble_chunk`/`disassemble_source` API functions

### Test Coverage

- 593 feature example tests across 9 categories: numbers (151), strings (97), control flow (82), collections (98), error handling (58), structs & enums (35), closures (28), traits (28), generics (16)
- 395 VM integration tests
- 228 unit tests
- 38 LeetCode solutions verified

## v0.2.2 — 2025-12-01

- Initial VM migration complete
- 38 LeetCode solutions passing
- Bytecode compiler for core language features

## v0.2.1 — 2025-10-15

- Renamed project from Ferrite to Oxide (later Oxy)
- File extension changed from .fe to .ox

## v0.2.0 — 2025-09-01

- Tree-walking interpreter with Pratt parser
- Basic Rust-like syntax: structs, enums, match, closures, async
- LSP server with diagnostics, completion, hover

## v0.1.0 — 2025-07-01

- First release of the language (as Ferrite)
- Lexer, parser, AST, basic interpreter
