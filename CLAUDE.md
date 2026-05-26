# CLAUDE.md — Oxy Language Project

## Project

Oxy is a compiled programming language written in Rust. Rust-like syntax without borrow checker/ownership. File extension: `.ox`.

**Pipeline:** `parse → type_check → compile → bytecode → VM`

There is no interpreter. One execution path: compiler to VM.

## Language Identity: Dynamic Rust

Oxy is **dynamic Rust** — Rust-like syntax WITHOUT ownership, lifetimes, or borrow checking. This is a deliberate, load-bearing choice. Do not weaken it.

### What Oxy DOES NOT have
- **Reference syntax**: `&T`, `&mut T`, `&self`, `&mut self`, `&str`, `&[T]`, `&expr`. The parser **rejects these** with a fix-it error message. They are not "accepted but ignored" — they error.
- **Lifetimes**: `'a`, `<'a>`. Not parsed, not supported.
- **Borrow checker**, move semantics, ownership-tracking rules. None of it.
- **Slice types**: `[T]` as a parameter type. Use `Vec<T>` instead.
- **Rust-style integer width zoo**: `i8 / i16 / i32 / i64 / u16 / u32 / u64 / isize / usize` are **not Oxy types**. The type checker rejects them with a fix-it suggesting `int`. `u8` is rejected too (use `byte`). `f32` is rejected (use `float`). The full width zoo was retired in favour of three numeric types: `int`, `byte`, `float`.

### What Oxy DOES have
- **Variable-level mutability**: `let mut x`, `mut self` for methods, `mut param: T` for fn parameters. Controls whether the binding can be reassigned. This is independent of borrow checking — it's the same as `const`/`let` in JS or `final` in Java.
- `Vec<T>` for dynamic-length lists. `String` for text. `[T; N]` for fixed-size arrays (coerce to `Vec` at fn boundaries). `Option<T>` and `Result<T, E>` for absence/error.
- **Exactly three numeric types**: `int` (signed, 64-bit wrapping), `byte` (unsigned, 8-bit wrapping), `float` (64-bit IEEE-754). Width semantics are enforced at function-call boundaries (entry and return) and at typed `let` bindings; intermediate arithmetic widens to `int` to keep mixed-type expressions ergonomic.

### If a user asks to add reference / borrow / lifetime features
**Push back before implementing.** Quote this section. Ask:

> Oxy's identity is dynamic-Rust-without-borrow-checking. Adding references/borrows/lifetimes would contradict that. Are you sure that's the direction you want?

Only proceed if they explicitly confirm and can articulate a coherent reason — and even then, flag the divergence in the commit message and update this section to document the policy change.

### If a user asks to reintroduce a Rust-style integer width (i8/i16/i32/u16/u32/u64/...)
**Push back.** Same protocol: Oxy chose `int + byte + float` deliberately — see the discussion above. Adding a width back is a language-design decision that needs a clear reason (e.g., binary protocol parsing where a specific width is load-bearing). Default answer is no; if accepted, document the rationale and update this section.

### Syntax mapping (for migration)
| Rust-ish | Oxy |
|---|---|
| `&self` | `self` |
| `&mut self` | `mut self` |
| `&T` (param) | `T` |
| `&mut T` (param) | `mut T` |
| `&str` | `String` |
| `&[T]` | `Vec<T>` |
| `&expr` | `expr` |
| `i8` / `i16` / `i32` / `i64` / `u16` / `u32` / `u64` / `isize` / `usize` | `int` |
| `u8` | `byte` |
| `f32` / `f64` | `float` |
| Suffixed literals (`5i8`, `255u8`, `3.14f32`) | bare literal, optionally `as int` / `as byte` / `as float` |

## Build & Test (Docker)

```bash
docker compose run --rm dev bash -c "cargo test"                    # All tests
docker compose run --rm dev bash -c "cargo test -p oxy-core"        # Core only
docker compose run --rm dev bash -c "cargo test -p oxy-lsp"         # LSP only
docker compose run --rm dev bash -c "cargo test -p oxy-tug"         # Tug only
docker compose run --rm dev bash -c "cargo fmt --all"               # Format
docker compose run --rm dev bash -c "cargo clippy --all-targets -- -D warnings"   # Lint
docker compose run --rm dev bash -c "cargo run -- run examples/hello.ox"  # Run
docker compose run --rm test                                        # Full CI
docker compose run --rm setup                                       # npm deps
docker compose run --rm build-ext                                   # Package .vsix
```

### Pre-Commit Checklist (run before every commit)

```bash
docker compose run --rm dev bash -c "cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p oxy-core"
docker compose run --rm dev bash -c "cargo clippy -p oxy-lsp --all-targets -- -D warnings && cargo test -p oxy-lsp"
docker compose run --rm dev bash -c "cargo clippy -p oxy-tug --all-targets -- -D warnings && cargo test -p oxy-tug"
```

All must pass. No exceptions.

## Architecture

```
crates/oxy-core/src/
├── lib.rs                       # Public API exports
├── lexer/mod.rs                 # Tokenizer → Vec<Token>
├── lexer/token.rs               # Token, TokenKind, Span (1-indexed)
├── ast/mod.rs                   # AST: Program, Item, Expr, Stmt, FnDef, etc.
├── parser/
│   ├── mod.rs                   #   Parser struct, parse_program (Pratt, precedence 0-14)
│   ├── expr.rs                  #   Expression parsing
│   ├── item.rs                  #   Item parsing (fn, struct, enum, impl, trait, mod)
│   ├── stmt.rs                  #   Statement parsing (let, use, if, while, for, return)
│   ├── pattern.rs               #   Pattern parsing (match arms, let destructure)
│   └── ty.rs                    #   Type annotation parsing
├── compiler/
│   ├── mod.rs                   #   Prescan, compile items, module handling, post-pass
│   ├── expr.rs                  #   Expression compilation + PathCall/StructInit
│   ├── pattern.rs               #   Pattern compilation (match, if-let, while-let)
│   ├── helpers.rs               #   Shared compiler helpers
│   ├── path_resolution.rs       #   Path name resolution
│   ├── visibility.rs            #   Visibility checks (is_visible, check_path_visible)
│   ├── loop_context.rs          #   Loop break/continue tracking
│   └── sym_table.rs             #   Symbol table
├── type_checker/
│   ├── mod.rs                   #   TypeChecker struct, check_program, TypeInfo
│   ├── check_expr.rs            #   Expression type inference
│   ├── check_item.rs            #   Item type checking
│   ├── check_stmt.rs            #   Statement type checking
│   ├── collect.rs               #   collect_defs + collect_fn_types
│   ├── resolve.rs               #   Name resolution
│   └── tests.rs                 #   Rust unit tests for type checker
├── vm/
│   ├── mod.rs                   #   Stack-based VM: dispatch, builtin_method, run_tests()
│   ├── builtins/                #   Per-type method implementations
│   │   ├── mod.rs               #     Re-exports
│   │   ├── numeric.rs           #     int/byte/float methods (signum, etc.)
│   │   ├── string.rs            #     String methods (find, lines, split_whitespace, etc.)
│   │   ├── vec.rs               #     Vec methods
│   │   ├── hashmap.rs           #     HashMap methods
│   │   ├── hashset.rs           #     HashSet methods
│   │   ├── btreemap.rs          #     BTreeMap methods
│   │   ├── btreeset.rs          #     BTreeSet methods
│   │   ├── iterator.rs          #     Iterator adapter methods
│   │   ├── option.rs            #     Option methods
│   │   ├── result.rs            #     Result methods
│   │   ├── binary_heap.rs       #     BinaryHeap methods
│   │   └── vec_deque.rs         #     VecDeque methods
│   ├── arith.rs                 #   Arithmetic operations
│   ├── call.rs                  #   Function call dispatch
│   ├── format.rs                #   String formatting (println!, format!)
│   ├── api.rs                   #   Public VM API
│   └── tests.rs                 #   Rust unit tests for VM
├── stdlib/
│   ├── mod.rs                   #   Stdlib registration + table-driven registry
│   ├── args.rs                  #   std::args::parse()
│   ├── db.rs                    #   std::db (SQLite)
│   ├── env.rs                   #   std::env
│   ├── fs.rs                    #   std::fs
│   ├── http.rs                  #   std::http
│   ├── io.rs                    #   std::io (stdin)
│   ├── json.rs                  #   json::
│   ├── math.rs                  #   math::
│   ├── net.rs                   #   std::net
│   ├── path.rs                  #   std::path
│   ├── process.rs               #   std::process (command + spawn)
│   ├── rand.rs                  #   rand::
│   ├── regex.rs                 #   std::regex
│   ├── registry.rs              #   Table-driven built-in dispatch
│   ├── server.rs                #   std::server (HTTP)
│   └── time.rs                  #   time::
├── symbols.rs                   #   Canonical symbol definitions (keywords, types, methods, modules)
├── types/mod.rs                 #   Value enum, type system
├── env/mod.rs                   #   Lexical scope chain
├── json/mod.rs                  #   Hand-written JSON ser/de
├── http/mod.rs                  #   HTTP client (ureq wrapper)
├── errors.rs                    #   FerriError: Lexer, Parser, TypeError, Runtime
└── repl.rs                      #   REPL utilities
crates/oxy-cli/src/main.rs       #   CLI binary: run, test, repl
crates/oxy-lsp/src/main.rs       #   LSP server (tower-lsp)
crates/oxy-tug/                  #   Package manager (tug)
├── src/
│   ├── main.rs                  #     CLI entry point
│   ├── install.rs               #     tug install/uninstall/list
│   ├── manifest.rs              #     tug.toml parsing
│   ├── lockfile.rs              #     tug.lock management
│   ├── project.rs               #     Project resolution
│   ├── runner.rs                #     tug build/run/test
│   └── scaffold.rs              #     tug new/init
└── tests/                       #   Integration tests
```

## Test Infrastructure

### Test types

| Type | Mechanism | Location |
|------|-----------|----------|
| Runtime tests | `#[test]` fn in `.ox` file | `examples/features/<category>/` |
| Compile-error tests | `#[compile_error]` fn in `.ox` file | `examples/features/<category>/` |
| Rust unit tests | `#[test]` in `#[cfg(test)]` modules | `crates/oxy-core/tests/vm_tests.rs` |
| Integration test | `feature_examples.rs` globs all `.ox` | `crates/oxy-core/tests/feature_examples.rs` |
| Leetcode tests | Same as feature examples | `crates/oxy-core/tests/leetcode_solutions.rs` |
| Symbol consistency | `#[test]` cross-referencing `symbols.rs` vs builtins/lexer/VM | `crates/oxy-core/tests/symbol_consistency.rs` |
| Extern modules | `#[test]` for `--extern` dependency loading | `crates/oxy-core/tests/extern_modules.rs` |

### `run_tests()` flow (`vm/mod.rs`)

```
1. parse(source) → Program
2. Split: normal_items (no #[compile_error]) vs compile_error_fns
3. type_check(normal_items) + compile(normal_items) — must succeed
4. Run each #[test] fn via VM → TestResult { passed, error }
5. For each #[compile_error] fn:
   a. Build program: normal_items + this fn
   b. Try type_check + compile
   c. Err → passed (expected error); Ok → FAILED (expected error, got none)
6. Return combined results
```

A `#[compile_error]` test passes if EITHER the type checker OR the compiler rejects it.

### Rust-side test helpers

- `run_and_capture(src) → Vec<String>` — compile + run main, return stdout lines
- `run(src) → Result<Value>` — compile + run main, return final value or error
- Source must wrap in `fn main() { ... }`
- `run_tests(path, source) → Result<Vec<TestResult>>` — run #[test] + #[compile_error] functions

## TDD Feature Development Process

This is the ONLY acceptable process for adding features:

1. **Write `.ox` test file first** in `examples/features/<category>/<name>.ox`
2. **Cover all cases:**
   - `#[test]` for each success/happy-path scenario
   - `#[test]` for edge cases and corner cases
   - `#[compile_error]` for every error case that should be rejected at compile time
3. **Run the feature test** to find failures:
   ```bash
   docker compose run --rm dev bash -c "cargo test -p oxy-core -- feature_examples"
   ```
4. **Fix the compiler/type checker** — NEVER change the test to pass when the compiler should catch it
5. **Iterate** until all tests pass
6. **Update downstream systems as needed:**
   - **LSP** (`crates/oxy-lsp/src/main.rs`) — new AST nodes, keywords, built-in types, or methods may need completion/hover/diagnostic updates
   - **VS Code extension** (`editors/vscode/`) — new keywords, types, or operators may need syntax highlighting updates in `oxy.tmLanguage.json`
   - **REPL** (`crates/oxy-core/src/repl.rs`) — new language constructs may need REPL-specific handling
   - At minimum, verify the LSP compiles: `cargo test -p oxy-lsp`
7. **Run full validation:**
   ```bash
   docker compose run --rm dev bash -c "cargo fmt --all && cargo clippy -- -D warnings && cargo test -p oxy-core"
   docker compose run --rm dev bash -c "cargo clippy -p oxy-lsp -- -D warnings && cargo test -p oxy-lsp"
   ```
8. **Commit** only when everything is green

### Write tests for ALL of these:
- Basic success case
- Edge case (empty, boundary, extreme values)
- Error cases via `#[compile_error]` (visibility, type mismatch, missing fields, etc.)
- Interaction with other features (modules + generics, visibility + impl blocks, etc.)

## Compiler Internals

### Compilation pipeline (`compiler/mod.rs`)

```
1. prescan_items() — register all fn/struct/enum names + pub_vis (so forward refs resolve)
2. preresolve_uses() — process use statements against prescanned data
3. compile items — function bodies, struct/enum definitions, modules, impls
4. Post-pass — patch forward calls, deferred globs
```

### Prescan phase (critical for forward references)

The prescan registers items BEFORE any function body is compiled. This allows `fn a()` to call `fn b()` even if `b` is defined after `a`.

**Must register in prescan:**
- `self.functions` — name → `usize::MAX` (placeholder IP)
- `self.fn_meta` — params + body + return type
- `self.struct_defs` — qualified name → StructDef
- `self.enum_defs` — qualified name → EnumDef
- `self.pub_vis` — qualified name → Visibility (if pub)

If `pub_vis` is NOT populated during prescan, `is_visible()` will return false for forward-referenced functions, breaking valid calls.

### Module compilation

Two code paths in `compiler/mod.rs`:
- **`compile_module()`**: top-level `mod foo { ... }` — prefix = `module.name`
- **`compile_module_items()` Item::Module**: nested modules — prefix = `"parent::child"` (cumulative)

Items in nested modules get fully qualified names: `"parent::child::fn_name"`.

### Visibility system

- **`pub_vis: HashMap<String, Visibility>`** — tracks pub items. Populated in prescan AND during compilation.
- **`module_names: HashSet<String>`** — tracks known module qualified names. Populated during `compile_module` and `compile_module_items`.

#### `is_visible(qualified) → bool`

1. If name not in functions/structs/enums/modules → `true` (untracked, e.g. builtins)
2. If in `pub_vis`:
   - `Pub` / `PubCrate` → `true`
   - `PubSuper` → check parent module ancestry
   - `Private` → `true` only if parent is NOT a module (top-level or struct-scoped)
3. If NOT in `pub_vis` (private):
   - `true` only if parent is empty or parent is NOT a known module
   - `false` otherwise (item inside a module, not pub)

**Key rule:** Top-level items and items scoped to structs (methods) are always accessible. Only items inside modules are subject to visibility restrictions.

#### `check_path_visible(path, span) → Result<(), Error>`

Called in PathCall and StructInit compilation. Checks:
1. Each intermediate path segment that's a module — must be visible
2. The leaf item (function/struct/enum) — must be visible

Module visibility: a private module is accessible only from its parent module or descendants.

### PathCall compilation

Resolution order for 2-segment paths like `Foo::bar()`:
1. Check if path[0] is an enum variant constructor
2. Try `self.functions.get("Foo::bar")` — direct match
3. Try type alias + use-aliased prefix
4. Try use_aliases on the full qualified name
5. Try module-qualified (current module prefix + path)
6. Try builtin path

**Must call `check_path_visible(path, span)?` before emitting Call.**

### StructInit compilation

1. Resolve name: `Self` → `current_impl_type`, then type_aliases, then use_aliases
2. Check enum variant constructor (if name contains `::`)
3. **Check `is_visible(resolved_name)`** — reject private structs
4. Check field visibility for each field via `check_field_visibility()`
5. Emit StructInit opcode

## Type Checker Internals

### `TypeChecker` struct fields

- `struct_defs` — qualified name → StructDef
- `type_aliases` — alias name → TypeAnnotation
- `fn_return_types` — qualified name → TypeInfo (return type)
- `use_aliases` — short_name → qualified_name
- `module_stack` — current module nesting
- `current_impl_type` — for `Self` resolution

### `check_program()` order

```
1. collect_defs(items, prefix)  — structs, type aliases, use aliases
2. collect_fn_types(items, prefix) — function/method return types
3. check_item() for each item — type-check bodies
```

### `collect_fn_types()` — MUST handle ALL of these:

- `Item::Function` — register return type under qualified name
- `Item::Module` — recurse with nested prefix
- **`Item::Impl`** — register method return types under `"Type::method"` AND `"prefix::Type::method"`
- **`Item::ImplTrait`** — same as Impl
- DO NOT skip Impl/ImplTrait — method return types will be Unknown, breaking field visibility checks

### `check_stmt()` — `Stmt::Use`

**MUST** populate `use_aliases` (not be a no-op):
```rust
Stmt::Use(use_def) => {
    // Process Simple, Group, Glob — same logic as Item::Use
    self.use_aliases.insert(local_name, qualified_name);
}
```

### PathCall return type resolution

Try in order:
1. `fn_return_types.get(path.join("::"))` — direct match
2. If 2-segment: resolve path[0] through use_aliases, try `"resolved::method"` in fn_return_types
3. Try module-qualified: `"current_module::path"`

### FieldAccess type inference

1. `infer_expr(object)` → get object type
2. If `UserStruct(struct_name)`: resolve struct name, call `check_field_visible()`
3. Return the field's declared type from struct_defs

### `check_field_visible()` — compile-time field enforcement

Compares struct's defining module against current `module_stack`. Private fields accessible only from within the same module.

## Anti-Patterns (NEVER DO THESE)

- **Quick workaround or hack** instead of proper compiler/type-checker implementation
- **Change `.ox` test to pass** when the compiler should reject/fail/error
- **Skip visibility checks** in PathCall, StructInit, or field access paths
- **Use `contains("::")`** for top-level detection — check `module_names.contains(parent)` instead
- **Forget to register `pub_vis` in prescan** — forward references will break
- **Leave `Stmt::Use` as a no-op** in type checker — use_aliases won't populate, field visibility checks won't resolve
- **Skip `Item::Impl`/`Item::ImplTrait` in `collect_fn_types`** — method return types will be Unknown
- **Skip `check_path_visible` / `is_visible()` in PathCall/StructInit** — private items will be accessible
- **Cut corners silently** — if a shortcut is unavoidable, tell the user and explain why
- **Use raw string literals in builtins dispatch match arms** — use `symbols::<type>_m::CONSTANT` instead. If you add a method without adding its constant to `symbols.rs`, it won't compile
- **Add a built-in method only to builtins or only to symbols** — must update both: the dispatch match arm (using the constant) AND the `MethodInfo` list in `symbols.rs`. Consistency tests + compile-time constants enforce this
- **Wire up only some Value variants** for a built-in dispatch — all integer/float widths must go through `numeric::dispatch`, all collection types must be handled. `dispatched_type_names()` + consistency tests catch gaps
- **Inline-match on type name strings** in the type checker or compiler — use `TypeInfo::from_name()` instead. A partial match with `_ => Unknown` silently accepts any type because `TypeInfo::accepts()` returns `true` when either side is `Unknown`. The `from_name` function in `type_checker/mod.rs` is the single source of truth for type name → TypeInfo conversion.

## Symbol Definitions (`symbols.rs`)

`crates/oxy-core/src/symbols.rs` is the **single source of truth** for all language symbols. Both the compiler/VM and the LSP import from it. Never hardcode keyword/type/method names in the LSP.

### Adding a new built-in method

1. Add a method name constant in `symbols.rs` (e.g., `pub mod string_m { pub const NEW_METHOD: &str = "new_method"; }`)
2. Add a `MethodInfo` entry in the corresponding `*_METHODS` list
3. Add the match arm in the builtins dispatch file using the constant (e.g., `symbols::string_m::NEW_METHOD => ...`)
4. Add the method name to `method_names()` in the builtins file (using the constant)
5. Update the `MethodInfo` list in `symbols.rs` if adding the method to a type's `*_METHODS`

### Adding a new built-in type

1. Add a `TypeInfo` entry to `ALL_TYPES` in `symbols.rs`
2. Add method name constants (a new `*_m` module) and a `*_METHODS` list
3. Add the type's dispatch in `vm/mod.rs` `builtin_method()`
4. Add the type to `dispatched_type_names()` in `vm/mod.rs`
5. Add the dispatch implementation in `vm/builtins/` with `method_names()` helper

### Enforcement

| Constraint | Mechanism |
|-----------|-----------|
| Method in dispatch but not in symbols | Compile error — constant doesn't exist |
| Method in symbols but not in dispatch | 26 consistency tests via `method_names()` + pre-commit hook |
| New type wired up incompletely | `test_dispatched_types_in_symbols` / `test_symbols_types_have_dispatch` |
| LSP out of date with symbols | LSP reads from `symbols` at runtime — no manual sync needed |

## Common Pitfalls & Their Fixes

| Pitfall | Fix |
|---------|-----|
| Forward ref breaks `is_visible()` | Register `pub_vis` in prescan phase |
| `name` moved before later use | Clone before first insert: `functions.insert(name.clone(), usize::MAX)` |
| `is_visible()` returns true for untracked items | Check `module_names` in the "is tracked" condition |
| Nested module items not found | `compile_module_items` uses cumulative prefix `"parent::child"` |
| Method return type Unknown | Register in `fn_return_types` under both `"Type::method"` and `"module::Type::method"` |
| Top-level items incorrectly blocked | Check `!self.module_names.contains(parent)` not `contains("::")` |
| `#[compile_error]` test passes when it shouldn't | Ensure the error is caught at type-check OR compile time; check both stages |

## Conventions

- `rustfmt.toml`: max width 100
- `thiserror` for error types
- Conventional commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`
- **No co-author trailers** on commits
- Test naming: `test_<what>_<scenario>`
- Unit tests in `#[cfg(test)]` modules within source files
- `.ox` feature tests in `examples/features/<category>/<name>.ox`

## Debug Tools

### `--dump-bytecode <file>` (CLI)
```bash
docker compose run --rm dev bash -c "cargo run --bin oxy -- --dump-bytecode examples/foo.ox"
```
Prints compiled bytecode: opcodes with IPs, slot names, function/closure entry points.

### `OXY_VM_TRACE=1` (env var)
```bash
OXY_VM_TRACE=1 docker compose run --rm dev bash -c "cargo test -p oxy-core --test feature_examples"
```
Per-opcode execution tracing to stderr: IP, stack state, frame info, compact values. Use when debugging a specific test failure.

### `disassemble_chunk(&Chunk) → String`
Programmatic bytecode disassembly in `oxy_core::vm::disassemble_chunk`.

### `disassemble_source(path, source) → Result<String>`
Parse + type-check + compile + disassemble in one call: `oxy_core::vm::disassemble_source`.

## Context-Mode MCP Tools

### Blocked commands — do NOT use
- `curl` / `wget` → use `ctx_fetch_and_index(url, source)` or `ctx_execute` with fetch
- `WebFetch` → use `ctx_fetch_and_index(url, source)` then `ctx_search(queries)`

### Tool selection
1. **GATHER**: `ctx_batch_execute(commands, queries)` — one call replaces many
2. **FOLLOW-UP**: `ctx_search(queries: ["q1", "q2"])` — query indexed content
3. **PROCESSING**: `ctx_execute(language, code)` / `ctx_execute_file(path, language, code)` — sandbox execution
4. **WEB**: `ctx_fetch_and_index(url, source)` then `ctx_search(queries)`

Bash is ONLY for: git, mkdir, rm, mv, navigation, and short-output commands. For everything else use sandbox equivalents.

### ctx commands

| Command | Action |
|---------|--------|
| `ctx stats` | Display context consumption statistics |
| `ctx doctor` | Diagnose context-mode installation |
| `ctx upgrade` | Upgrade context-mode to latest |
