# Contributing to Oxy

A guide for adding features, fixing bugs, and improving the Oxy programming language — written for human contributors.

## Setup

You only need Docker.

```bash
git clone https://github.com/sabinbajracharya/Oxy.git
cd Oxy
docker compose run --rm setup
docker compose run --rm dev bash

# Inside the container
cargo build
cargo test -p oxy-core
cargo run -- run examples/hello.ox
```

Enable the pre-commit hook:
```bash
git config core.hooksPath .githooks
```

## How Oxy Works

```
Source (.ox) → Lexer → Parser → Type Checker → Compiler → VM (bytecode)
```

There is no interpreter. One path: compile to bytecode, execute on the stack-based VM. Values use `Rc<RefCell<>>` under the hood — no borrow checker.

## Project Map

```
crates/oxy-core/src/
├── lexer/            token.rs (Token, TokenKind, Span), mod.rs (tokenizer)
├── parser/           mod.rs — Pratt parser, ~3200 lines, 15 precedence levels
├── ast/              mod.rs — Program, Item, Expr, Stmt, FnDef, StructDef, etc.
├── type_checker/     mod.rs — semantic type checking, field visibility enforcement
├── compiler/         mod.rs — AST → bytecode Chunk (prescan → compile → post-pass)
├── vm/
│   ├── mod.rs        Stack-based VM, builtin_method, dispatch_pathcall, run_tests
│   └── builtins/     Per-type method dispatches (string, vec, hashmap, etc.)
├── types/            mod.rs — Value enum (25 variants), type_name, ordering
├── stdlib/           fs, env, process, regex, net, time, rand, math, db, server
├── symbols.rs        ★ Canonical symbol definitions — single source of truth
├── errors.rs         FerriError (Lexer, Parser, TypeError, Runtime)
└── lib.rs            Public API, re-exports
crates/oxy-cli/       CLI binary (run, repl, --dump-tokens, --dump-ast, --dump-bytecode)
crates/oxy-lsp/       LSP server (tower-lsp)
editors/vscode/       VS Code extension (syntax highlighting + LSP client)
examples/features/    Feature tests (.ox files with #[test] / #[compile_error])
tests/                Rust-side tests (vm_tests, feature_examples, symbol_consistency)
```

## Feature Development (TDD)

Every feature follows this process. No exceptions.

### 1. Write the test file

Create `examples/features/<category>/<name>.ox`. Cover every case:

```rust
fn add(a: i64, b: i64) -> i64 { a + b }

#[test]
fn test_add_positive() { assert_eq!(add(2, 3), 5); }

#[test]
fn test_add_negative() { assert_eq!(add(-1, -2), -3); }

#[test]
fn test_add_edge_cases() {
    assert_eq!(add(0, 0), 0);
    assert_eq!(add(i64::MAX, i64::MIN), -1);
}

#[compile_error]
fn test_type_mismatch_rejected() {
    let x: i64 = "not a number";
}
```

A `#[compile_error]` test passes only if compilation fails.

### 2. Run the test

```bash
cargo test -p oxy-core -- feature_examples
```

### 3. Implement

Fix the compiler/type checker/VM. Never change the test to pass when the compiler should reject it.

### 4. Update downstream systems

| Change | Also update |
|--------|-------------|
| New keyword | `symbols.rs` KEYWORDS, `editors/vscode/syntaxes/oxy.tmLanguage.json`, LSP keyword_hover_text |
| New built-in method | `symbols.rs` (constant + MethodInfo), dispatch in `vm/builtins/`, `method_names()` |
| New built-in type | `types/mod.rs` Value variant, `vm/builtins/<type>.rs`, `vm/mod.rs` dispatch + `dispatched_type_names()`, `symbols.rs` (constants + TypeInfo) |
| New syntax (expr/stmt) | Lexer, AST, parser, type checker, compiler, VM, LSP |
| New operator | Lexer, parser (precedence), compiler, VM, `oxy.tmLanguage.json` |

### 5. Validate

```bash
cargo fmt --all
cargo clippy -- -D warnings
cargo clippy -p oxy-lsp -- -D warnings
cargo test -p oxy-core
cargo test -p oxy-lsp
```

All six must pass. The pre-commit hook enforces this.

## Adding a Built-in Method

This is the most common contribution. The constraint system requires all four steps.

### Step 1: Add the constant in symbols.rs

```rust
// symbols.rs
pub mod string_m {
    // ...
    pub const REVERSE: &str = "reverse";
}
```

### Step 2: Add the MethodInfo in symbols.rs

```rust
pub const STRING_METHODS: &[MethodInfo] = methods![
    // ...
    "reverse": "() -> String" => "Return the reversed string.",
];
```

### Step 3: Add the dispatch arm in vm/builtins/

```rust
// vm/builtins/string.rs
match method {
    // ...
    symbols::string_m::REVERSE => Ok(Value::String(s.chars().rev().collect())),
    _ => Err(format!("no method '{}' on type String", method)),
}
```

### Step 4: Add to method_names()

```rust
pub fn method_names() -> &'static [&'static str] {
    &[
        // ...
        symbols::string_m::REVERSE,
    ]
}
```

### Why this order

- Using a raw string instead of a constant → consistency tests fail
- Using the constant before adding it to symbols → **compile error**
- Adding to symbols but skipping the dispatch → reverse consistency test fails

The `tests/symbol_consistency.rs` file has 26 tests that enforce this bi-directionally.

## Adding a New Built-in Type

1. Add `Value::MyType` variant in `types/mod.rs`
2. Create `vm/builtins/my_type.rs` with `dispatch()` and `method_names()`
3. Add dispatch arm in `vm/mod.rs` `builtin_method()`
4. Add to `dispatched_type_names()` in `vm/mod.rs`
5. In `symbols.rs`: add name constants, method name constants, `*_METHODS` list, `TypeInfo` entry in `ALL_TYPES`
6. Add consistency tests covering the new type
7. Add `.ox` feature tests in `examples/features/`
8. LSP picks it up automatically from `symbols`

## The Symbols Module

`crates/oxy-core/src/symbols.rs` is the **single source of truth** for all language symbols. Both the compiler/VM and the LSP read from it. Never hardcode a keyword, type name, or method name in the LSP.

What it defines:
- `KEYWORDS` — all 36 keywords
- `PRIMITIVE_TYPES` — 19 types with descriptions
- `ALL_MACROS` — 9 built-in macros with hover text
- `ALL_MODULES` — 10 stdlib module paths
- `ALL_TYPES` — 11 built-in types, each with its full method list
- Per-type method name constants (`string_m::*`, `vec_m::*`, etc.)
- Type name constants (`I64_TYPE`, `STRING_TYPE`, etc.)

## Adding a Syntax Feature

For new expressions, statements, or patterns:

1. **Lexer** — add tokens in `lexer/token.rs`, tokenize in `lexer/mod.rs`
2. **AST** — add variants in `ast/mod.rs`
3. **Parser** — add parsing in `parser/mod.rs` (Pratt precedence if it's an expression)
4. **Type checker** — add type inference/checking in `type_checker/mod.rs`
5. **Compiler** — add bytecode emission in `compiler/mod.rs`
6. **VM** — add opcode execution in `vm/mod.rs`
7. **Tests** — `.ox` feature tests covering happy path, edge cases, `#[compile_error]`
8. **LSP** — update completions/hover if user-facing
9. **TextMate grammar** — update `editors/vscode/syntaxes/oxy.tmLanguage.json` if keywords, types, or operators changed

## Testing

### Test types

| Type | Location | Run with |
|------|----------|----------|
| Rust unit tests | `#[cfg(test)]` in source files | `cargo test -p oxy-core` |
| VM tests | `tests/vm_tests.rs` | `cargo test -p oxy-core --test vm_tests` |
| Feature tests | `examples/features/**/*.ox` | `cargo test -p oxy-core -- feature_examples` |
| Symbol consistency | `tests/symbol_consistency.rs` | `cargo test -p oxy-core --test symbol_consistency` |
| LSP tests | `oxy-lsp/src/main.rs` | `cargo test -p oxy-lsp` |

### Running subsets

```bash
cargo test -p oxy-core                         # all core tests (~1350)
cargo test -p oxy-core -- feature_examples     # .ox feature tests only
cargo test -p oxy-core --test vm_tests         # VM tests only
cargo test -p oxy-core --test symbol_consistency  # consistency tests only
cargo test -p oxy-core -- test_my_test         # by name
```

## Commit Messages

[Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add String::reverse method
fix: wire all integer widths to numeric dispatch
refactor: extract method name constants to symbols
test: add consistency tests for new LinkedList type
docs: update CLAUDE.md with symbols workflow
style: fix rustfmt
```

No co-author trailers. One logical change per commit.

## Debugging

```bash
# Dump bytecode
cargo run -- --dump-bytecode examples/hello.ox

# Dump tokens
cargo run -- --dump-tokens examples/hello.ox

# Dump AST
cargo run -- --dump-ast examples/hello.ox

# Per-opcode VM execution trace
OXY_VM_TRACE=1 cargo test -p oxy-core --test vm_tests -- test_string_len
```

## Common Pitfalls

- **Visibility check with `contains("::")`** — use `module_names.contains(parent)` instead. `::` appears in struct-qualified names too.
- **Forgetting `pub_vis` in prescan** — forward references to pub items break visibility checks.
- **Skipping `#[compile_error]` tests** — every feature needs negative tests for what the compiler must reject.
- **Adding a method to dispatch without updating symbols** — use the constant from `symbols.rs`, never a raw string literal.
- **`name` moved before later use** — clone before inserting into the first HashMap.

## Getting Help

Open an issue or start a discussion. We're happy to help.
