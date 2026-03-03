# CLAUDE.md — Project Context

## Project: Ferrite

Interpreted programming language written in Rust. Replicates Rust syntax without borrow checker/ownership. File extension: `.fe`.

## Build & Test (Docker — no local Rust)

```bash
docker compose run --rm dev bash -c "cargo test"                    # All tests
docker compose run --rm dev bash -c "cargo test -p ferrite-core"    # Core only
docker compose run --rm dev bash -c "cargo test -p ferrite-lsp"     # LSP only
docker compose run --rm dev bash -c "cargo fmt --all"               # Format
docker compose run --rm dev bash -c "cargo clippy -- -D warnings"   # Lint
docker compose run --rm dev bash -c "cargo run -- run examples/hello.fe"  # Run
docker compose run --rm test                                        # Full CI check
docker compose run --rm setup                                       # Install npm deps
docker compose run --rm build-ext                                   # Package .vsix
```

## Architecture

```
crates/
├── ferrite-core/src/
│   ├── lib.rs           # Public API exports
│   ├── lexer/           # Tokenizer → Vec<Token>
│   │   ├── mod.rs       # Scanner (scan_token, scan_string, scan_fstring, scan_number)
│   │   └── token.rs     # Token, TokenKind (keywords, operators, literals), Span
│   ├── ast/mod.rs       # AST nodes: Item, Expr, Stmt, FnDef, StructDef, EnumDef, Attribute, FStringPart
│   ├── parser/mod.rs    # Pratt parser (~3200 lines). Precedence levels 0-14.
│   ├── interpreter/mod.rs  # Tree-walking evaluator (~7000+ lines). All runtime logic.
│   ├── types/mod.rs     # Value enum: Integer, Float, Bool, String, Vec, HashMap, Struct, EnumVariant, Function(Box), Future(Box), etc.
│   ├── env/mod.rs       # Lexical scope chain (parent pointer)
│   ├── json/mod.rs      # Hand-written JSON ser/de (no deps)
│   ├── http/mod.rs      # HTTP client wrapping ureq
│   └── errors.rs        # FerriError: Lexer/Parser/Runtime with line/column, Return(Box<Value>), Break, Continue
├── ferrite-cli/src/main.rs  # CLI: run, repl, --dump-tokens, --dump-ast
└── ferrite-lsp/src/main.rs  # LSP server (tower-lsp): diagnostics, completion, hover, symbols, goto-def
editors/vscode/
├── extension.js         # LSP client — launches ferrite-lsp via Docker or native binary
├── package.json         # Extension manifest with ferrite.lsp.mode/path/enabled settings
├── syntaxes/ferrite.tmLanguage.json  # TextMate grammar
└── language-configuration.json       # Brackets, comments, indentation
```

## Key Patterns (follow these when adding features)

### Adding a built-in macro (e.g. `println!`, `vec!`, `format!`)
→ Add match arm in `interpreter::eval_macro_call()`

### Adding a path-call module (e.g. `math::sqrt()`, `json::parse()`, `http::get()`)
→ Add match arm in `interpreter::eval_path_call()` under the module prefix

### Adding a path constant (e.g. `math::PI`)
→ Handle in `interpreter::eval_expr()` under `Expr::Path`

### Adding methods on built-in types (e.g. `.sqrt()`, `.len()`, `.clone()`)
→ Add match arm in `interpreter::call_method()` → type-specific dispatcher:
  - `call_vec_method()`, `call_string_method()`, `call_hashmap_method()`
  - Numeric methods handled inline in `call_method()`

### Adding a new expression type
1. Add variant to `Expr` enum in `ast/mod.rs`
2. Parse it in `parser/mod.rs` (`parse_prefix` or `parse_infix`)
3. Evaluate it in `interpreter::eval_expr()`

### Adding a new item type (struct/enum feature)
1. Extend AST node in `ast/mod.rs` (e.g. add field to `StructDef`)
2. Parse in `parser::parse_item()`
3. Register in `interpreter::register_item()`

### Adding a new Value type
1. Add variant to `Value` enum in `types/mod.rs` + Display impl
2. Handle in `interpreter::call_method()` for methods
3. Handle in binary/comparison operators if needed

## Critical Implementation Details

- **Value::Function and Value::Future are boxed** — `Function(Box<FunctionData>)`, `Future(Box<FutureData>)` to prevent stack overflow in recursive code
- **FerriError::Return and Break are boxed** — `Return(Box<Value>)`, `Break(Option<Box<Value>>)` to satisfy clippy result_large_err
- **Spans are 1-indexed** — line 1, column 1. LSP converts to 0-indexed.
- **Struct init disambiguation** — `Ident { ... }` is struct init only if name starts with uppercase
- **Option/Result** — modeled as `Value::EnumVariant` with `enum_name: "Option"/"Result"`
- **Test pattern** — `run_and_capture(src)` returns `Vec<String>` of output lines (each ending `\n`). Source must wrap in `fn main() { ... }`.
- **Pratt parser precedence** — None(0) → Assignment(1) → Range(2) → Or(3) → And(4) → Equality(8) → Comparison(9) → Term(11) → Factor(12) → Unary(13) → Call(14)
- **Method dispatch order** — Vec → String → HashMap → Option/Result → HttpResponse → HttpRequestBuilder → numeric methods → .to_json() → impl blocks → trait impls → trait defaults
- **Derived traits** tracked in `interpreter.derived_traits: HashMap<String, HashSet<String>>`
- **Async** — simulated, not real threads. `async fn` returns lazy `Value::Future`, `.await` evaluates it.

## Conventions

- `rustfmt.toml` config (max width 100)
- `thiserror` for error types
- Conventional commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`
- Always add trailer: `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`
- Tests: `test_<what>_<scenario>` naming, unit tests in `#[cfg(test)]` modules
