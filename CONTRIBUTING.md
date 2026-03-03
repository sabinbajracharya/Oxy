# Contributing to Ferrite

Thank you for your interest in contributing to Ferrite! 🧲

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/project-ferrite.git`
3. Create a feature branch: `git checkout -b feat/my-feature`
4. Make your changes
5. Run checks: `docker compose run --rm dev bash -c "cargo fmt --all && cargo clippy -- -D warnings && cargo test --workspace"`
6. Commit with conventional commit messages
7. Push and open a Pull Request

## Development Setup (Docker — recommended)

No local Rust install needed. Everything runs inside Docker:

```bash
docker compose run --rm dev bash          # Drop into a dev shell
docker compose run --rm dev bash -c "cargo test --workspace"   # Run all tests
docker compose run --rm dev bash -c "cargo build --release"    # Release build
```

## Architecture Overview

Ferrite is a Cargo workspace with three crates:

```
project-ferrite/
├── crates/
│   ├── ferrite-core/    # Language engine (library crate)
│   │   └── src/
│   │       ├── lib.rs          # Crate root — re-exports all modules
│   │       ├── lexer/          # Tokenizer: source text → Token stream
│   │       │   ├── mod.rs      # Lexer implementation
│   │       │   └── token.rs    # Token, TokenKind, Span definitions
│   │       ├── ast/            # AST node definitions (Expr, Stmt, Item, etc.)
│   │       ├── parser/         # Recursive descent parser (Pratt parsing for expressions)
│   │       ├── interpreter/    # Tree-walking interpreter ← the big one
│   │       │   ├── mod.rs      # Core: eval_expr, eval_stmt, call_function, REPL tests
│   │       │   ├── operations.rs   # Binary/unary operator evaluation
│   │       │   ├── pattern.rs      # Pattern matching & variable binding
│   │       │   ├── macros.rs       # println!, print!, vec!, format! macros
│   │       │   ├── format.rs       # Debug formatting ({:?})
│   │       │   ├── path.rs         # Path resolution (Type::method), associated fns, trait dispatch
│   │       │   ├── json.rs         # json:: module (serialize/deserialize)
│   │       │   ├── http.rs         # http:: module (GET/POST/PUT/DELETE)
│   │       │   └── methods/        # Type-specific method dispatch
│   │       │       ├── mod.rs      # call_method() dispatcher
│   │       │       ├── vec.rs      # Vec methods (push, pop, iter, map, filter, etc.)
│   │       │       ├── string.rs   # String methods (contains, replace, split, etc.)
│   │       │       ├── hashmap.rs  # HashMap methods (insert, get, keys, etc.)
│   │       │       ├── option_result.rs  # Option/Result methods (unwrap, map, etc.)
│   │       │       └── numeric.rs  # Numeric methods (abs, pow, clamp, etc.)
│   │       ├── types/          # Value enum, FunctionData, type constants
│   │       ├── env/            # Lexical scoping (Environment with parent chain)
│   │       ├── stdlib/         # Built-in functions (math, rand, time)
│   │       └── errors/         # FerriError enum, check_arg_count helper
│   ├── ferrite-cli/     # CLI binary (REPL + file execution)
│   └── ferrite-lsp/     # Language Server Protocol implementation
├── editors/
│   └── vscode/          # VS Code extension (syntax highlighting + LSP client)
├── tests/e2e/           # End-to-end test harness
└── examples/            # Example .fe programs
```

### How a Ferrite program executes

```
Source text (.fe file)
    → Lexer (token.rs, mod.rs)     → Vec<Token>
    → Parser (parser/mod.rs)       → Program (AST)
    → Interpreter (interpreter/)   → Result<Value, FerriError>
```

1. **Lexer** scans characters into tokens with span info (line, column)
2. **Parser** builds an AST using recursive descent with Pratt parsing for expressions
3. **Interpreter** walks the AST tree, evaluating expressions and executing statements
4. Values are reference-counted (`Clone`-based) — no borrow checker or ownership

## How To: Add a Standard Library Function

Example: adding `math::floor(x)`.

1. **Register the function** in `stdlib/math.rs` — add a match arm in `call_math_function()`:
   ```rust
   "floor" => {
       check_arg_count("math::floor", 1, args, &span)?;
       match &args[0] {
           Value::Float(f) => Ok(float_to_value(f.floor())),
           Value::Integer(n) => Ok(Value::Integer(*n)),
           _ => Err(FerriError::Runtime { message: "floor() requires a number".into(), line: span.line, column: span.column }),
       }
   }
   ```

2. **Add a test** at the bottom of `stdlib/math.rs` or in `interpreter/mod.rs`:
   ```rust
   #[test]
   fn test_math_floor() {
       let result = run("fn main() { println!(\"{}\", math::floor(3.7)); }");
       assert_eq!(result, "3\n");
   }
   ```

3. **Run tests**: `docker compose run --rm dev bash -c "cargo test --workspace"`

## How To: Add a Method to a Built-in Type

Example: adding `String::repeat(n)`.

1. **Add a match arm** in `interpreter/methods/string.rs` → `call_string_method()`:
   ```rust
   "repeat" => {
       check_arg_count("repeat", 1, args, span)?;
       if let Value::Integer(n) = &args[0] {
           Ok(Value::String(s.repeat(*n as usize)))
       } else {
           Err(FerriError::Runtime { message: "repeat() requires an integer".into(), line: span.line, column: span.column })
       }
   }
   ```

2. **Add a test** and run: `cargo test`

## How To: Add a New AST Node

1. Add the variant to the relevant enum in `ast/mod.rs` (e.g., `Expr::MyNewExpr { ... }`)
2. Add parsing logic in `parser/mod.rs`
3. Add evaluation logic in `interpreter/mod.rs` → `eval_expr()` or `eval_stmt()`
4. Add tests for both parsing and evaluation

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` — new feature
- `fix:` — bug fix
- `refactor:` — code restructuring (no behavior change)
- `test:` — adding or updating tests
- `docs:` — documentation changes
- `chore:` — maintenance tasks

## Code Standards

- Run `cargo fmt --all` before committing
- All code must pass `cargo clippy -- -D warnings`
- All tests must pass: `cargo test --workspace`
- Public items must have doc comments (`///`)
- New features must include tests
- Use `check_arg_count()` from `errors` module for argument validation
- Look for `// WHY:` comments for explanations of non-obvious design decisions

## Testing

- **Unit tests:** In `#[cfg(test)]` modules alongside source code (417+ tests in ferrite-core)
- **Integration tests:** CLI tests in ferrite-cli (8 tests)
- **LSP tests:** In ferrite-lsp (9 tests)
- **E2E tests:** `.fe` files with `.expected` output in `tests/e2e/programs/`

### Running specific tests

```bash
docker compose run --rm dev bash -c "cargo test -p ferrite-core"       # Core only
docker compose run --rm dev bash -c "cargo test -p ferrite-cli"        # CLI only
docker compose run --rm dev bash -c "cargo test -p ferrite-lsp"        # LSP only
docker compose run --rm dev bash -c "cargo test test_closures"         # By name
```

## Common Pitfalls

- **Value is Clone, not Copy** — use `.clone()` when needed, but be aware it's cheap (Rc-based internally)
- **Span propagation** — always pass spans through to error constructors for good error messages
- **Method vs associated function** — methods receive `self` as the object; associated functions (like `Vec::new()`) are dispatched through `eval_path_call` in `path.rs`
- **Trait dispatch** — after built-in operators fail, the interpreter falls back to trait dispatch (e.g., `Add::add`). See `operations.rs`.
- **Docker volumes** — the `target/` directory is a Docker volume, not mounted to host. Build artifacts stay inside Docker.

## Questions?

Open an issue or start a discussion — we're happy to help!
