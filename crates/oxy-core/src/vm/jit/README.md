# `vm/jit/` — Native Backend (Register IR + Cranelift)

## Purpose

The native execution path: lower the AST to a register IR + CFG (`ir_gen/`), compile
that IR to Cranelift CLIF and then to native machine code (`codegen.rs`), and provide
the shared `oxy_*` runtime (`ffi.rs`, `runtime.rs`) that **both** backends call. The
IR interpreter (`../interp.rs`) consumes the identical `IrFunction`s.

## Files

| File | Responsibility |
|---|---|
| `mod.rs` | `JitEngine` / `JitVm`, `dump_ir` (backs `oxy --dump-ir` / `tug build`), FFI consistency tests. |
| `ir.rs` | Register IR types: `IrOp`, `Terminator`, `IrFunction`, registers, basic blocks. |
| `ir_gen/` | AST → register IR + CFG (see its README). |
| `codegen.rs` | IR → Cranelift CLIF → native code; declares FFI signatures via `ffi_decls()`. |
| `ffi/` | The shared `oxy_*` runtime functions both backends call. `mod.rs` holds the core machinery (push/pop, call stack, arithmetic + macros, closures, method dispatch, scheduler, the closure-invoker hook, and `ffi_symbols()`); the self-contained domains are split out: `collections.rs`, `strings_fmt.rs`, `structs.rs`, `casts.rs`, `enums.rs`. Moved fns are `pub(super)` and referenced by module path in `ffi_symbols()`. |
| `runtime.rs` | Arithmetic / cast helpers invoked by the FFI. |
| `context.rs` | `JitContext` — output buffer, locals, error state. |
| `ir_snapshot.rs` | IR pretty-printer used by the snapshot tests. |

## Companion design docs (in this folder)

- `IR_DESIGN.md` — canonical reference for the register IR design & invariants.
- `IR_SNAPSHOT_FORMAT.md` — the snapshot serialization format spec (implemented by `ir_snapshot.rs`).

The IR snapshot test-coverage plan is fulfilled (82 golden files under
`crates/oxy-core/tests/snapshots/ir/**`, asserted by `tests/ir_snapshot_tests.rs`); the
original plan is retired to `docs/history/ir-test-coverage-plan.md`.

## Key types & entry points

- `IrFunction` — the unit both backends run; functions resolved by name at call time
  (forward references work naturally).
- `JitEngine` — compiles and runs on native targets.
- `ffi_decls()` (here in codegen) ↔ `ffi_symbols()` (here in ffi) — kept consistent
  by `ffi_consistency_tests` in `mod.rs`.

## Invariants & gotchas

- The IR and the `oxy_*` FFI are the **shared contract** with `../interp.rs`. A new
  `oxy_*` must be added to **both** `ffi_decls()` and `ffi_symbols()`.
- Don't paper over codegen bugs with magic offsets/guards — see the JIT anti-patterns
  in `CLAUDE.md`. Repeated unsafe `Value`-move patterns belong behind `move_value`.
- Per-function local counts must come from each function's own `local_count`, never
  `main`'s — mismatches cause silent heap corruption.

## When you change this folder

- New `IrOp`/`Terminator` → update `ir.rs`, `codegen.rs`, `ir_gen/`, **and**
  `../interp.rs` (no-wildcard match breaks the build until you do).
- Regenerate snapshots when IR output changes:
  `UPDATE_SNAPSHOTS=1 cargo test -p oxy-core ir_snapshot`.
- `ffi/mod.rs` is still large (the entangled core); further extraction of the
  call-stack and closure machinery is a possible follow-up. Keep this table current.
