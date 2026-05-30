# `tests/vm_tests/` — Native execution test suite

## Purpose

The Rust-side behavioral test suite for the language. Every test compiles a small
`.ox` program through the real pipeline (parse → type-check → ir_gen → Cranelift JIT)
and asserts on its captured stdout or final value. This is the single `vm_tests`
integration-test target, split by topic so no one file is unwieldy.

## Layout

`main.rs` is the test-target crate root (Cargo auto-discovers `tests/<dir>/main.rs`
as one integration test named `vm_tests`). It holds the shared imports and the
`run_and_capture` / `run_and_get_value` helpers; every topic submodule pulls them in
with `use super::*`.

| File | What it covers |
|---|---|
| `main.rs` | Crate root: imports, shared helpers, `mod` declarations. |
| `basics.rs` | Literals, variables, arithmetic, comparisons, logic, `as` casts. |
| `functions.rs` | Function definition, calls, returns, arity checking. |
| `control_flow.rs` | if/else, match, loops, labeled break/continue, range patterns. |
| `collections.rs` | Vec, HashMap, HashSet, BinaryHeap, VecDeque, `collect`. |
| `strings.rs` | String/char methods, substrings, f-string interpolation. |
| `structs_enums.rs` | Struct & enum definition, impls, field mutation, built-in nodes. |
| `traits_generics.rs` | Traits, generics, where-clauses, type aliases, constants, derive. |
| `error_handling.rs` | `Result`/`Option`, `?`, panics, parse failures. |
| `closures.rs` | Closures, higher-order fns, iterator chains, captures. |
| `modules.rs` | Modules, `use`, visibility, field-visibility enforcement. |
| `stdlib.rs` | JSON, HTTP, math, CLI args. |
| `diagnostics.rs` | Error-message DX, assert macros, the test runner, recursion limit. |
| `patterns.rs` | let/for destructuring and assorted syntax-gap regression tests. |
| `reference_syntax.rs` | `&`/`&mut` rejection — Oxy is dynamic Rust (see `CLAUDE.md`). |

## Key entry points

- `run_and_capture(src) -> Vec<String>` — compile + run `main`, return stdout lines.
- `run_and_get_value(src) -> Value` — compile + run `main`, return the final value.
- `run` / `run_capturing` — re-exported from `oxy_core::vm` for tests that assert on
  raw `Result`s (compile errors, panics).

## Invariants & gotchas

- Submodules resolve relative to `main.rs` because Cargo treats it as the crate root.
  A top-level `tests/vm_tests.rs` would instead look for siblings in `tests/` — that's
  why the entry lives **inside** the directory as `main.rs`.
- These tests run on the **native JIT** path. Cross-backend agreement with the wasm
  interpreter is checked separately by `jit_interp_parity`, not here.

## When you change this folder

- Add a test to the submodule that matches its topic; create a new submodule (and a
  `mod` line in `main.rs` + a row above) only for a genuinely new area.
- Keep this table and the `main.rs` `mod` list in sync with the files present.
