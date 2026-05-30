# Changelog

## Unreleased

- Renamed the `--dump-bytecode` CLI flag to `--dump-ir`, reflecting the
  register-IR pipeline (Oxy compiles through a register IR to native code via
  Cranelift, not bytecode). `--dump-bytecode` is kept as a hidden alias.

## v0.4.0 — 2026-05-25

Highlights: a brand-new package manager (`tug`), a Rust-style split between
compiler and dependency tooling, and a substantially larger standard library.

### New: `tug` package manager (separate binary)

Oxy now ships a second executable, `tug`, that owns project layout,
dependency management, and orchestration of the `oxy` compiler — exactly
the way `cargo` complements `rustc`.

- `tug new <name>` / `tug init` — scaffold a project (`tug.toml`,
  `src/main.ox`, `.gitignore`).
- `tug add <spec> [--git URL] [--tag T|--rev R] [--path P]` /
  `tug remove <name>` — edit `tug.toml` and sync `tug.lock`.
- `tug install <path|url>` / `tug uninstall <name>` / `tug list` —
  package store at `~/.oxy/packages/`.
- `tug build` / `tug run [args...]` / `tug test` — resolves deps and
  shells out to `oxy` with the appropriate `--extern` flags.
- New manifest (`tug.toml`) and lockfile (`tug.lock`) formats, both TOML.

### Breaking: compiler split

- The `oxy` binary no longer has `install`, `uninstall`, or `list`
  subcommands. Use `tug` for package management. Standalone scripts
  (`oxy run file.ox`) work as before for self-contained files.
- The compiler no longer reads `~/.oxy/packages/` directly. Dependencies
  must be supplied by the caller via the new `oxy --extern <name>=<path>`
  flag — mirroring `rustc --extern`. `tug` builds this map automatically.
- `oxy_core::package` module removed; replaced by `oxy_tug::install` etc.

### Standard library additions

- `std::process::spawn(program, args, callback)` — line-by-line streaming
  subprocess execution, with stdout/stderr tagged in the callback.
- `std::path` — lexical path manipulation (`join`, `split`, `extension`,
  `with_extension`, `parent`, `file_stem`, `is_absolute`, etc.).
- Real `std::env::args()` plus a `std::args::parse(spec)` helper.
- `std::server` — closure-callback HTTP server (`server::start(addr, fn)`).
- `std::io` — stdin reading; `std::db` — bundled SQLite client.
- `std::regex::Regex::new(pat).<method>(text)` — OOP-style regex.
- `int.signum()` / `byte.signum()` / `float.signum()`.
- `String::lines()` and `String::split_whitespace()`.

### Language features

- Breaking: integer types collapsed to `int`, `byte`, `float` (the
  `i8 / i16 / i32 / i64 / u8 / u16 / u32 / u64 / f32 / f64` zoo is
  rejected with a fix-it suggestion). Width semantics are enforced at
  function boundaries and typed `let` bindings; arithmetic widens to
  `int` for ergonomics.
- Breaking: Rust reference syntax (`&T`, `&mut T`, `'a`, `&self`,
  `&str`, `&[T]`) is rejected — Oxy commits to dynamic Rust.
- `fn main() -> Result<(), E>` is allowed; `Err(_)` is surfaced and
  exits with a non-zero status.
- Struct update syntax: `Foo { field: v, ..base }`.
- `if let Some(x) = expr && guard { ... }` — pattern with `&&` guard.
- Or-patterns: `1 | 2 | 3 => ...` (including in `let`).
- Generic `impl<T> Type<T>` blocks.
- 3-segment enum paths `mod::Enum::Variant` compile.
- Closure type inference and fixed-size array types `[T; N]`.

### Internals (no user-visible change)

- `vm/mod.rs` split into `arith`, `format`, `call`, `api` submodules.
- `parser/mod.rs` split into `ty`, `item`, `stmt`, `expr`, `pattern`.
- `type_checker/mod.rs` split into `resolve`, `collect`, `check_item`,
  `check_stmt`, `check_expr`.
- `compiler/mod.rs` split into `expr`, `pattern`, `helpers`,
  `path_resolution`, `visibility`, `loop_context`, `sym_table`.
- Built-in dispatch is now table-driven via `stdlib::registry`.

### LSP & tooling

- LSP gains AST-aware completions, compiler diagnostics, improved
  goto-definition and hover.
- New shared `symbols.rs` (single source of truth for keywords, types,
  methods, modules) with compile-time-enforced consistency.

### Bug fixes (selected)

- Stack-discipline fixes for `println!` and `match` in recursive fns.
- Match arm guard-fail no longer underflows into caller frame.
- Field-assignment now enforces declared immutability.
- Numeric `signum`, `clamp`, `rand_int` correctness.
- Lazy iterator adapters share state via `Rc<RefCell>` so `next()`
  advances stored state correctly.

### Release / CI

- GitHub release workflow now builds and publishes both `oxy` and
  `tug` for Linux, macOS (aarch64), and Windows.
- Docker runtime image ships `oxy`, `tug`, and `oxy-lsp`.

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
