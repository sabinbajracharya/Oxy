# CLAUDE.md — Project Context for Claude Code

## Project: Ferrite

**Ferrite** is an interpreted programming language written in Rust that replicates Rust's syntax without the borrow checker or ownership rules. Values are reference-counted internally. File extension: `.fe`.

## Quick Reference

### Build
```bash
cargo build                    # Debug build
cargo build --release          # Release build
```

### Test
```bash
cargo test                     # Run all tests
cargo test -p ferrite-core     # Core library tests only
cargo test -p ferrite-cli      # CLI integration tests only
```

### Lint
```bash
cargo fmt --all --check        # Check formatting
cargo fmt --all                # Auto-format
cargo clippy --all-targets --all-features -- -D warnings  # Lint
```

### Run
```bash
cargo run -- --version         # Print version
cargo run -- --help            # Print help
cargo run -- run file.fe       # Execute a Ferrite file
cargo run -- repl              # Start interactive REPL
cargo run -- --dump-tokens f.fe # Dump token stream
cargo run -- --dump-ast f.fe    # Dump AST
```

### Docker
```bash
docker compose run dev                    # Dev shell with Rust toolchain
docker compose run dev cargo test         # Run tests in container
docker compose run test                   # Full CI checks in container
docker compose build build                # Build release Docker image
```

## Architecture

Cargo workspace with two crates:

- **`ferrite-core`** (library): Language engine — lexer, parser, AST, interpreter, types, environment, stdlib, errors
- **`ferrite-cli`** (binary): CLI interface — REPL and file execution

### Module Structure (ferrite-core)
```
src/
├── lib.rs          # Public API
├── lexer/          # Tokenization
├── parser/         # Recursive descent parser
├── ast/            # AST node definitions
├── interpreter/    # Tree-walking evaluator
├── types/          # Value system (Rc<RefCell<Value>>)
├── env/            # Lexical scoping environment
├── stdlib/         # Built-in functions and types
└── errors/         # Error types with span info
```

## Conventions

### Code Style
- Follow `rustfmt.toml` config (max width 100)
- All public items must have doc comments (`///`)
- Use `thiserror` for error type derivation
- Prefer `impl Display` over `ToString`
- Use `#[must_use]` on pure functions returning values
- All match arms must be exhaustive (no catch-all `_ =>` unless intentional)

### Error Handling
- Library functions return `Result<T, FerriError>` (the project's error type)
- CLI handles errors at the top level and sets exit codes
- All errors carry source span information for user-facing messages

### Testing
- Unit tests in `#[cfg(test)]` modules within source files
- Integration tests in `tests/` directories
- E2E tests: `.fe` source files paired with `.expected` output files in `tests/e2e/programs/`
- Use snapshot testing (`insta` crate) for AST and token dumps
- Test names follow `test_<what>_<scenario>` pattern

### Git
- Conventional commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`
- Keep commits atomic — one logical change per commit
- Always include `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>` trailer

### Key Design Decisions
- **No borrow checker:** All values are `Rc<RefCell<Value>>`. Borrow syntax (`&`, `&mut`) is parsed but ignored at runtime.
- **No lifetimes:** Lifetime annotations are parsed but ignored.
- **Dynamic typing internally:** Types are checked at runtime. Type annotations are parsed for syntax fidelity but enforcement is gradual.
- **`println!` is special:** Parsed as a pseudo-macro (not a real macro system). Same for `vec![]`, `format!()`, etc.
- **Immutability is enforced:** `let x` creates an immutable binding; `let mut x` creates a mutable one. Assignment to immutable binding is a runtime error.

## Current Phase

Phase 7: Structs, Enums & Impl Blocks — COMPLETE. 224 tests passing.

### What Works Now
- `let` / `let mut` bindings with immutability enforcement
- Arithmetic: `+`, `-`, `*`, `/`, `%` (integers, floats, mixed)
- String concatenation with `+`
- Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
- Logical: `&&`, `||`, `!`
- Bitwise: `&`, `|`, `^`, `<<`, `>>`
- `if` / `else if` / `else` expressions (return values)
- Block expressions: `{ let y = 10; y + 1 }`
- `while condition { body }` loops
- `loop { body }` with `break value` support
- `for x in 0..10 { body }` and `0..=10` inclusive ranges
- `for x in vec { body }` — iterate over Vec and String
- `break` and `continue` in all loop types
- `match expr { pattern => body }` with literal, wildcard (`_`), variable, and enum variant patterns
- Function definitions and calls with proper scoping
- Recursion (factorial, fibonacci)
- `return` statements
- `println!("{}", x)` with `{}` format placeholders and `{:?}` debug format
- `print!()` without newline
- `vec![1, 2, 3]` macro for vector creation
- `[1, 2, 3]` array literal syntax (creates Vec)
- `v[0]` index access for Vec, String, Tuple
- `v[0] = x` index assignment for Vec
- Vec methods: `.push()`, `.pop()`, `.len()`, `.is_empty()`, `.contains()`, `.first()`, `.last()`, `.reverse()`, `.join()`
- String methods: `.len()`, `.is_empty()`, `.contains()`, `.to_uppercase()`, `.to_lowercase()`, `.trim()`, `.starts_with()`, `.ends_with()`, `.replace()`, `.chars()`, `.split()`, `.repeat()`, `.push_str()`
- Tuples: `(a, b, c)`, tuple index `t.0`, empty tuple `()`, single-element `(x,)`
- Structs: `struct Point { x: f64, y: f64 }`, unit structs, tuple structs
- Struct instantiation: `Point { x: 1.0, y: 2.0 }`, shorthand `Point { x, y }`
- Struct field access: `p.x`, field assignment: `p.x = 3.0`
- Enums: `enum Shape { Circle(f64), Rectangle(f64, f64), Point }`
- Enum variant construction: `Shape::Circle(5.0)`, unit variants `Color::Red`
- Enum pattern matching: `Shape::Circle(r) => ...` with destructuring
- Impl blocks: `impl Point { fn new(x: f64, y: f64) -> Self { ... } }`
- Methods: `&self`, `&mut self`, `self` parameters, called via `p.method()`
- Associated functions: `Point::new(1.0, 2.0)` via `Type::func()` syntax
- `Self` type resolution in impl blocks
- `self` keyword for accessing receiver in methods
- Auto debug format for structs and enums via `{:?}`
- `&` and `&mut` syntax parsed but ignored (no borrow checker)
- Shadowing: `let x = 1; let x = "hello";`
- Compound assignment: `+=`, `-=`, `*=`, `/=`, `%=`
- REPL with persistent environment
- File execution with `ferrite run file.fe`
