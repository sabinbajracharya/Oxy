# Oxy Execution Model — Register IR + Two Backends

**Status:** canonical. Supersedes the retired `history/bytecode-vm.md`.

## Pipeline

```
Source (.ox)
  → Lexer        → Vec<Token>
  → Parser       → AST (Program)
  → Type Checker → validated AST
  → ir_gen       → Register IR + CFG  (IrFunction per fn)
  → Backend      → output
```

There is **no bytecode and no stack-based VM** anymore, and **no tree-walking
interpreter**. Oxy lowers the AST to a **register IR** and then runs it on one of two
backends.

## Two backends, one IR, one runtime

| Backend | Target | Lives in | Used by |
|---|---|---|---|
| **Cranelift JIT** | native (x86/aarch64/…) | `crates/oxy-core/src/vm/jit/` | CLI, `tug`, native tests |
| **IR interpreter** | `wasm32` | `crates/oxy-core/src/vm/interp.rs` | browser playground/tutorial |

**Why two.** Cranelift emits host machine code and mmaps it executable — it cannot run
in a browser wasm sandbox and has no wasm-emitting backend. The playground needs
in-browser execution, so on `wasm32` we *walk* the IR instead of compiling it.

**Why they can't diverge.** Both backends consume the **identical** `IrFunction`s from
`ir_gen`, and both delegate runtime semantics to the **same shared `oxy_*` FFI**
(`jit/ffi.rs`) and arithmetic helpers (`jit/runtime.rs`). The interpreter does not
reimplement language semantics — it pushes operands and calls the same FFI bodies the
JIT calls. `api.rs` selects the backend per target via `#[cfg(target_arch = "wasm32")]`.

## The register IR

- A function is an `IrFunction`: named, with basic blocks of `IrOp`s ending in a
  `Terminator` (`br`, `ret`, …). Registers (`v0`, `v1`, …) are slots in an array.
- The interpreter walks blocks: each op reads input slots, does work (often via an
  `oxy_*` FFI call), writes a result slot; terminators move control.
- Functions are resolved **by name** at call time, so definition order is irrelevant
  and forward references work naturally.
- Full design + invariants: `crates/oxy-core/src/vm/jit/IR_DESIGN.md`.

## The `oxy_*` FFI — the shared runtime

The `oxy_*` functions (`oxy_add`, `oxy_println_val`, `oxy_call`, `oxy_struct_init`, …)
in `jit/ffi.rs` (+ helpers in `jit/runtime.rs`) are plain Rust functions that perform
the actual work of every operation. They have a stable C-style ABI so Cranelift can
emit calls into them. The JIT calls them as native machine code; the interpreter calls
the same function bodies from its op-walking loop. **One runtime, both backends.**

## The divergence guards (load-bearing)

Because the same feature must work on both backends, three guards make divergence loud:

1. **Compile-time — exhaustive match.** `interp.rs` is compiled on *all* targets; its
   `match` over `IrOp`/`Terminator` has **no wildcard**. Add/remove an IR op and every
   native build fails until the interpreter is updated. (See
   `architecture/exhaustive-ast-walkers.md` for the same idea applied to AST walkers.)
2. **Test-time — FFI surface consistency.** `ffi_decls()` (codegen) and `ffi_symbols()`
   (interpreter) are independent lists; `ffi_consistency_tests` asserts they match.
3. **Runtime opt-out — `unsupported_on_wasm!`.** For a feature that genuinely needs
   native code. Currently nothing uses it — the closure-invoker hook removed the last
   cases.

### The closure-invoker hook

Some callees are reached from *inside* the shared Rust runtime (higher-order built-ins
like `map`/`filter`/`fold`, async eager-runs, user `Display::fmt`). On native they call
through `JitTables.fn_table`; the interpreter's `fn_table` is empty, so it installs a
thread-local hook (`ffi::set_interp_invoke`) that interprets the callee at a target IP.
Both backends share one code path — only *who* runs the callee differs.

## Parity

Run the `examples/features/**` corpus through both backends and diff the output:

```bash
docker compose run --rm dev bash -c "cargo test -p oxy-core --test jit_interp_parity"
```

A failure means the backends disagree — investigate before merging.

## Inspecting the IR

```bash
oxy --dump-ir <file.ox>      # pretty-print the lowered register IR
```

(`--dump-bytecode` is a hidden back-compat alias; Oxy compiles through a register IR,
not bytecode.)

## Related docs

- `crates/oxy-core/src/vm/README.md` — the runtime folder map.
- `crates/oxy-core/src/vm/jit/IR_DESIGN.md` — IR design & invariants.
- `adr/001-unified-jit-calling.md`, `adr/002-eliminate-global-jit-tables.md` — JIT ADRs.
- `history/` — retired bytecode/stack-VM docs (provenance only).
