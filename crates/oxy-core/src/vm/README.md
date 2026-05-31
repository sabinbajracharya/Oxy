# `vm/` — Execution: Shared Runtime + Two Backends

## Purpose

Owns program execution. Oxy has **two backends that run the same register IR** and
share **one runtime** (the `oxy_*` FFI + arithmetic helpers), so they cannot diverge
by construction:

| Backend | Target | Lives in | Used by |
|---|---|---|---|
| Cranelift JIT | native (x86/aarch64/…) | `jit/` | CLI, `tug`, native tests |
| IR interpreter | `wasm32` | `interp.rs` | browser playground/tutorial |

`api.rs` picks the backend per target through one seam: the `ExecutionBackend`
trait (implemented by `JitBackend` and `InterpBackend`) plus a single
`#[cfg(target_arch = "wasm32")]`-selected `ActiveBackend` alias. Every public
dispatcher routes through `ActiveBackend`, so the target switch is one
polymorphic call rather than a `#[cfg]` branch repeated at each entry point.

## Files

| File | Responsibility |
|---|---|
| `mod.rs` | `VmResult`, public re-exports, `builtin_method` dispatch, `dispatched_type_names()`, `run_tests` plumbing, FFI consistency tests. |
| `api.rs` | Public entry points (`run_compiled`, `run_tests`, `disassemble_source`); the `ExecutionBackend` seam + `ActiveBackend` per-target selection. |
| `scheduler.rs` | Task registry for `spawn`/`await`/`select`. Tasks run eagerly to completion (JIT code can't be paused), so this just holds each task's result + virtual `sleep` time. |
| `interp.rs` | The IR interpreter backend (compiled on **all** targets, used on wasm). |
| `builtins/` | Per-type method implementations (see its README). |
| `jit/` | The native Cranelift backend, register IR, ir_gen, FFI (see its README). |
| `tests.rs` | Rust unit tests that compile/run via the JIT. |

## Key types & entry points

- `api::run_compiled` / `api::run_tests` — how the CLI/tests execute a program.
- `builtin_method` (`mod.rs`) — routes a method call to the right `builtins/` file.
- `disassemble_source` — backs `oxy --dump-ir`.

## Invariants & gotchas — the divergence guards (load-bearing)

1. **Exhaustive match.** `interp.rs`'s `match` over `IrOp`/`Terminator` has **no
   wildcard** — adding/removing an IR op breaks *every native build* until the
   interpreter is updated. Never add a `_ => {}` arm.
2. **FFI surface consistency.** `ffi_decls()` (codegen) and `ffi_symbols()`
   (interpreter) are independent lists; `ffi_consistency_tests` asserts they match.
   Add an `oxy_*` to one → add it to both.
3. **`unsupported_on_wasm!`** marks features that genuinely need native code
   (currently none — the closure-invoker hook removed the last cases).

The **closure-invoker hook** (`ffi::set_interp_invoke`) lets callees reached from
inside the shared Rust runtime (higher-order built-ins, async eager-runs, user
`Display::fmt`) be interpreted on wasm. To support a new "called from inside the
runtime" feature on wasm, route its `fn_table` miss to the hook — don't reach for
`unsupported_on_wasm!` unless it truly needs the host.

## When you change this folder

- New `IrOp`/`Terminator` → handle it in `interp.rs` (build breaks otherwise).
- New `oxy_*` FFI → add to both `ffi_decls()` and `ffi_symbols()`.
- Run the parity suite: `cargo test -p oxy-core --test jit_interp_parity`.
- Keep the file table and the backend table current.
