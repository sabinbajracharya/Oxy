# CLAUDE.md — Oxy Language Project

## Project

Oxy is a compiled programming language written in Rust. Rust-like syntax without borrow checker/ownership. File extension: `.ox`.

**Pipeline:** `parse → type_check → ir_gen (AST → Register IR + CFG) → codegen (IR → Cranelift CLIF) → native`

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
docker compose run --rm dev bash -c "rustup target add wasm32-unknown-unknown 2>/dev/null; cargo check --target wasm32-unknown-unknown -p oxy-core --no-default-features"
```

All must pass. No exceptions.

**WASM note:** `std::thread::sleep` panics on `wasm32` (calls `unreachable`).
Always gate thread-sleep behind `#[cfg(not(target_arch = "wasm32"))]` and provide
a WASM fallback (no-op or skip). `std::time::Instant` works on WASM (backed by
`performance.now()`).

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
├── type_checker/
│   ├── mod.rs                   #   TypeChecker struct, check_program, TypeInfo
│   ├── check_expr.rs            #   Expression type inference
│   ├── check_item.rs            #   Item type checking
│   ├── check_stmt.rs            #   Statement type checking
│   ├── collect.rs               #   collect_defs + collect_fn_types
│   ├── resolve.rs               #   Name resolution
│   └── tests.rs                 #   Rust unit tests for type checker
├── vm/
│   ├── mod.rs                   #   VmResult type, public API re-exports
│   ├── api.rs                   #   Public entry points (run_compiled, run_tests)
│   ├── scheduler.rs             #   Async task scheduler
│   ├── interp.rs                #   IR interpreter backend (wasm32) — compiled on all targets
│   ├── builtins/                #   Per-type method implementations
│   │   ├── mod.rs               #     Re-exports
│   │   ├── numeric.rs           #     int/byte/float methods
│   │   ├── string.rs            #     String methods
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
│   └── tests.rs                 #   Rust unit tests (compile via JIT)
├── vm/jit/
│   ├── mod.rs                   #   JitEngine, JitVm
│   ├── context.rs               #   JitContext (buffer, locals, error state)
│   ├── ir.rs                    #   Register IR types (IrOp, Terminator, IrFunction)
│   ├── ir_gen/mod.rs            #   AST → Register IR + CFG
│   ├── codegen.rs               #   IR → Cranelift CLIF
│   ├── ffi.rs                   #   FFI bridge (oxy_* functions)
│   ├── ir_snapshot.rs           #   IR pretty-printer for snapshot tests
│   └── runtime.rs               #   Arithmetic/cast helpers called by FFI
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
└── errors.rs                    #   PipelineError: Lexer, Parser, TypeError, Runtime
crates/oxy-cli/src/main.rs       #   CLI binary: run, test, repl (run_repl lives here)
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

## Per-Folder Documentation (keep it current)

Every source folder has a `README.md` documenting, for both humans and AI agents,
what the folder is for, what each file does, its key types/entry points, its
invariants/gotchas, and what else must change when you change it. The same applies to
each crate root (`crates/oxy-*/README.md`).

**Rule (load-bearing):** when you **add, remove, rename, or change the responsibility
of a file** in a folder, **update that folder's `README.md` in the same change**. A PR
that restructures a folder without updating its `README.md` is incomplete. When you
split a large file, map each new file in the README's file table.

- Folder map and architecture overview: each `crates/oxy-core/src/**/README.md`.
- Project-level architecture: [`docs/README.md`](docs/README.md) →
  [`docs/execution-model.md`](docs/execution-model.md) is the canonical execution model.
- Retired docs (the removed tree-walking interpreter and bytecode VM) live under
  `docs/history/` — do not treat them as current.

## Two Execution Backends (native JIT + wasm IR interpreter)

Oxy has **two execution backends that run the same register IR**:

| Backend | Target | Lives in | Used by |
|---|---|---|---|
| **Cranelift JIT** | native (x86/aarch64/…) | `vm/jit/` (`codegen.rs`, `JitEngine`, `JitVm`) | CLI, `tug`, native tests |
| **IR interpreter** | `wasm32` | `vm/interp.rs` (`InterpEngine`, `Interpreter`) | browser playground/tutorial |

**Why two.** Cranelift emits host machine code and mmaps it executable — it cannot run in a browser wasm sandbox, and it has no wasm-emitting backend. The playground (`playground/wasm` → `oxy-wasm`) needs in-browser execution, so on `wasm32` we walk the IR instead of compiling it.

**One IR, one runtime.** Both backends consume the identical `IrFunction`s from `ir_gen`, and both delegate runtime semantics to the **same shared `oxy_*` FFI** (`jit/ffi.rs`) and arithmetic helpers (`jit/runtime.rs`). The interpreter does **not** reimplement language semantics — it pushes operands and calls the same FFI bodies the JIT calls. Arithmetic, collections, strings, structs, enums, `?`, closures, user methods, recursion all ride that shared layer, so they cannot diverge by construction. `api.rs` picks the backend per target via `#[cfg(target_arch = "wasm32")]`.

### The divergence guards (this is load-bearing — do not weaken)

Because the same feature must work on both backends, every runtime change risks silently breaking the wasm path. Three guards make divergence **loud**:

1. **Compile-time — exhaustive match.** `vm/interp.rs` is compiled on **all** targets (not just wasm). Its `match` over `IrOp` / `Terminator` has **no wildcard arm**. Add or remove an IR op and *every native build* fails to compile until the interpreter is updated. Never paper over this with a `_ => {}` arm.
2. **Test-time — FFI surface consistency.** `ffi_decls()` (codegen's CLIF signatures) and `ffi_symbols()` (the shared pointer+ABI table the interpreter dispatches from) are independent hand-maintained lists. `ffi_consistency_tests` in `jit/mod.rs` asserts they describe the same names and the same return ABI. Add an `oxy_*` to one list but not the other → test fails.
3. **Runtime opt-out — `unsupported_on_wasm!`.** For a feature reachable through the shared FFI that genuinely cannot run without native code, route it through the `unsupported_on_wasm!(ctx, "feature")` macro in `vm/interp.rs`. It produces a clear error instead of silent wrong output. Grep for the macro to audit what's deliberately unsupported — **currently nothing** (the closure-invoker hook below removed the last cases). The macro is kept ready for future use.

### The closure-invoker hook (how callees reach the interpreter)

The JIT invokes a compiled function by calling its native pointer in `JitTables.fn_table`. The interpreter's `fn_table` is **empty**, so any runtime site that would call through it has nothing to invoke. Direct calls (`oxy_call`, `oxy_method_call`, `oxy_call_closure`, path calls, operator overloads) are intercepted at the IR level in `interp.rs` and interpreted recursively. But some callees are reached from *inside* the shared Rust runtime, where IR-level interception can't help:

- **higher-order built-ins** — `map`/`filter`/`fold`/`sort_by`/`for_each`/Option·Result combinators, and `std::process::spawn`'s per-line callback, all invoke a user closure from inside a Rust loop via `jit_closure_invoker`;
- **async eager-runs** — `oxy_spawn_ffi`/`oxy_await_ffi` run a task/future body to completion through a native pointer;
- **user `Display::fmt`** — `display_via_user_fmt` renders a struct/enum through its compiled `fmt` method.

For these, the interpreter installs a **thread-local hook** (`ffi::set_interp_invoke`, installed for the whole run by `Interpreter::install_invoker`) that interprets a function at a given `target_ip`. Each of the sites above, on an `fn_table` miss, calls the hook instead of native code. The JIT never installs the hook and always resolves through `fn_table`, so both backends share one code path — the only difference is *who* runs the callee. To support a new "called from inside the runtime" feature on wasm, route its `fn_table` miss to the hook the same way; don't reach for `unsupported_on_wasm!` unless the feature genuinely needs the host.

### When you change the runtime (adding/removing/changing a feature)

You **must** keep the interpreter in sync — there is no "native-only" shortcut for a language feature:
- New `IrOp` / `Terminator` → the build breaks until you handle it in `interp.rs` (guard #1).
- New `oxy_*` FFI → add it to **both** `ffi_decls()` and `ffi_symbols()` (guard #2 enforces it). The interpreter then calls it automatically via `ffi_symbols`.
- A feature that can't run on the interpreter → mark it with `unsupported_on_wasm!` (guard #3) **or** implement it in `interp.rs`. Do not let it fall through to an FFI that misbehaves on an empty `fn_table`.

### Parity command

Run the same `.ox` corpus through both backends and diff the output:

```bash
docker compose run --rm dev bash -c "cargo test -p oxy-core --test jit_interp_parity"
```

This is the feature-parity check between the JIT and the wasm interpreter. A failure means the two backends disagree on a program's output — investigate before merging.

### Known interpreter gaps

None. The whole `examples/features/**` corpus is at parity (`jit_interp_parity`): async (`spawn`/`await`/`sleep`/`select`), higher-order built-ins (`map`/`filter`/`fold`/`sort_by`/`for_each`/Option·Result combinators), `std::process::spawn` streaming, and user `Display::fmt` all run identically on both backends via the closure-invoker hook above. If you add a feature the interpreter can't yet run, mark it with `unsupported_on_wasm!` and list it here.

## Test Infrastructure

### Test types

| Type | Mechanism | Location |
|------|-----------|----------|
| Runtime tests | `#[test]` fn in `.ox` file | `examples/features/<category>/` |
| Compile-error tests | `#[compile_error]` fn in `.ox` file | `examples/features/<category>/` |
| Rust unit tests | `#[test]` in `#[cfg(test)]` modules | `crates/oxy-core/tests/vm_tests/` (topic submodules under `main.rs`) |
| Integration test | `feature_examples.rs` globs all `.ox` | `crates/oxy-core/tests/feature_examples.rs` |
| Leetcode tests | Same as feature examples | `crates/oxy-core/tests/leetcode_solutions.rs` |
| Symbol consistency | `#[test]` cross-referencing `symbols.rs` vs builtins/lexer/JIT | `crates/oxy-core/tests/symbol_consistency.rs` |
| Extern modules | `#[test]` for `--extern` dependency loading | `crates/oxy-core/tests/extern_modules.rs` |
| IR snapshot tests | golden-file compare of the pretty-printed register IR | `crates/oxy-core/tests/ir_snapshot_tests.rs` (+ `tests/snapshots/ir/**`) |
| JIT↔interp parity | runs the `examples/features/**` corpus through both backends and diffs output | `crates/oxy-core/tests/jit_interp_parity.rs` |

**IR snapshot golden files are pinned to LF via `.gitattributes`** (`tests/snapshots/ir/**/*.txt text eol=lf`). The pretty-printer only emits `\n`; without the pin, a Windows checkout (`core.autocrlf`) rewrites them to CRLF and fails every snapshot test with a phantom "no line differences" mismatch. The test also normalizes newlines defensively. Regenerate goldens with `UPDATE_SNAPSHOTS=1 cargo test -p oxy-core ir_snapshot`.

### `run_tests()` flow (`vm/mod.rs`)

```
1. parse(source) → Program
2. Split: normal_items (no #[compile_error]) vs compile_error_fns
3. type_check(normal_items) + compile(normal_items) — must succeed
4. Run each #[test] fn via JIT → TestResult { passed, error }
5. For each #[compile_error] fn:
   a. Build program: normal_items + this fn
   b. Try type_check + compile
   c. Err → passed (expected error); Ok → FAILED (expected error, got none)
6. Return combined results
```

A `#[compile_error]` test passes if EITHER the type checker OR ir_gen/codegen rejects it.

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
4. **Fix the ir_gen/codegen** — NEVER change the test to pass when the type checker should catch it
5. **Iterate** until all tests pass
6. **Update downstream systems as needed:**
   - **LSP** (`crates/oxy-lsp/src/main.rs`) — new AST nodes, keywords, built-in types, or methods may need completion/hover/diagnostic updates
   - **VS Code extension** (`editors/vscode/`) — new keywords, types, or operators may need syntax highlighting updates in `oxy.tmLanguage.json`
   - **REPL** (`crates/oxy-cli/src/main.rs`, `run_repl`) — new language constructs may need REPL-specific handling
   - **Folder docs** — if you add/remove a file or change a folder's responsibility, update that folder's `README.md` (see "Per-Folder Documentation" below)
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

## IR Gen Internals (`jit/ir_gen/mod.rs`)

### Compilation pipeline

```
1. gen_program() — iterate top-level items, dispatch to gen_fn / gen_module_items
2. gen_fn() — create IrFunction, allocate locals for params, generate body IR
3. gen_stmt() / gen_expr() — walk AST, emit IrOp + Terminator into basic blocks
4. gen_module_items() — recurse into modules with cumulative "parent::child" prefix
```

Forward references work naturally: all functions are generated as named `IrFunction` entries regardless of definition order. The JIT resolves them by name at call time.

### Module compilation

- **`gen_module_items()`**: recurses with cumulative prefix `"parent::child"`
- Items in nested modules get fully qualified names: `"parent::child::fn_name"`
- Use aliases are resolved in `gen_program()` and stored in `self.use_aliases`

### Function call resolution (`Expr::Call`)

1. Check if callee is a local holding a closure → route to `oxy_call_closure`
2. Resolve use aliases (`use calc::triple` → `"calc::triple"`)
3. Check if name is an enum variant constructor → route to `oxy_make_enum_variant`
4. Check for built-in FFI functions (spawn, sleep, select)
5. Otherwise → `CallBuiltin("oxy_call", ...)` with the qualified name

### StructInit compilation (Expr::StructInit)

1. Check enum variant constructor via `variant_to_enum` map → route to `oxy_make_enum_variant`
2. If `base` is present → route to `oxy_struct_update`
3. Otherwise → `CallBuiltin("oxy_struct_init", ...)`

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

- **Quick workaround or hack** instead of proper type-checker/ir_gen implementation
- **Change `.ox` test to pass** when the type checker should reject/fail/error
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
- **Inline-match on type name strings** in the type checker or ir_gen — use `TypeInfo::from_name()` instead. A partial match with `_ => Unknown` silently accepts any type because `TypeInfo::accepts()` returns `true` when either side is `Unknown`. The `from_name` function in `type_checker/mod.rs` is the single source of truth for type name → TypeInfo conversion.

## Symbol Definitions (`symbols.rs`)

`crates/oxy-core/src/symbols.rs` is the **single source of truth** for all language symbols. Both ir_gen/codegen and the LSP import from it. Never hardcode keyword/type/method names in the LSP.

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

### `OXY_VM_TRACE=1` (env var)
```bash
OXY_VM_TRACE=1 cargo test -p oxy-core --test feature_examples 2> ir_dump.txt
```
Dumps the register IR for every compiled function to stderr. Each function shows its basic blocks with register ops and terminators. Use when debugging a specific test failure to see what IR was generated.

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

## JIT Codegen Anti-Patterns

When debugging JIT codegen failures, do NOT brute-force individual test fixes. Instead, ask:

1. **Is this a one-off routing bug?** (e.g. `vec!` not wired to `oxy_make_array`) — fix the routing.
2. **Is it a shared-resource collision?** (e.g. two things sharing one buffer/block/stack and stepping on each other) — separate them architecturally, don't add offsets or guards.
3. **Is it a missing IR concept?** (e.g. no way to express Phi isolation) — add the IR primitive.

The trampoline approach (IR continuation blocks for Phi isolation) and the spill-from-top approach (two stacks growing toward each other in one buffer) are examples of architectural fixes that eliminated entire classes of bugs at once.

If a fix adds a magic constant, an offset, or a special-case guard, flag it — it's probably papering over an architectural issue.

**If you fix the same bug pattern in more than one place, stop and create a shared abstraction.** The `move_value` helper is the canonical example: `invoke_jit_fn`, `oxy_call_closure`, and `pop` all moved `Value` between buffer slots via `ptr::read` — and two of the three had the "forgot to clear source" double-free bug. The moment you recognize a repeated unsafe pattern, encode the invariant once (e.g. `move_value(src, dst)` always clears the source) so the invariant can't be forgotten at the next call site.

**Always check whether a design shortcut creates a mismatch that can silently corrupt state.** Per-function local counts stored in the engine vs. inferred from `main` is the canonical example: `call_fn` used `engine.local_count` (main's) for every function's buffer, but codegen computed spill offsets from each function's own `local_count`. The mismatch caused silent heap corruption only when a function had more locals than `main` — a latent bug that became a crash only when test files grew complex.

## JIT Debugging Protocol

When debugging a JIT feature test failure, the ONLY valid investigation path is:

1. **Read the failing `.ox` test** — what value does it expect vs get?
2. **Trace `ir_gen` for that Expr/Stmt** — what IR ops does it emit?
3. **Trace `codegen` for those IR ops** — what CLIF/FFI calls result?
4. **If the FFI function looks wrong, read it in `ffi.rs` or `jit/runtime.rs`**

The `vm/` directory contains the public API entry points (`api.rs`), the async scheduler (`scheduler.rs`), built-in method implementations (`builtins/`), the wasm IR interpreter (`interp.rs`), the shared `VmResult` type, and the `jit/` backend. Arithmetic helpers live in `jit/runtime.rs`. The JIT and the interpreter are the only two execution engines, and they share one runtime (see "Two Execution Backends" above).
