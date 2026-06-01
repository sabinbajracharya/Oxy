# `oxy-core` — Compiler & Runtime Core

## Purpose

The heart of Oxy: the full pipeline `parse → type_check → ir_gen (AST → register IR
+ CFG) → codegen (IR → Cranelift CLIF) → native`, plus the wasm IR interpreter, the
standard library, and the shared symbol/type definitions. Consumed by `oxy-cli`,
`oxy-lsp`, and the wasm playground.

## Root files

| File | Responsibility |
|---|---|
| `lib.rs` | Public API surface and module declarations / re-exports. |
| `symbols.rs` | **Single source of truth** for all language symbols — keywords, primitive/built-in types, macros, stdlib modules, and per-type method lists. Both the compiler and the LSP import from here; never hardcode a symbol elsewhere. |
| `diagnostics/` | Structured diagnostics model (codes, labels, notes/help, fix-its) consumed by CLI/LSP adapters. |
| `errors.rs` | Pipeline error/control-flow enum with backward-compatible conversion to structured diagnostics. |

## Submodules (each has its own README)

| Folder | Stage / role |
|---|---|
| `lexer/` | source → tokens |
| `parser/` | tokens → AST |
| `ast/` | AST node definitions |
| `type_checker/` | static checking, visibility, name resolution |
| `vm/` | execution: shared runtime + JIT and interpreter backends |
| `vm/jit/` + `vm/jit/ir_gen/` | register IR, lowering, Cranelift codegen, shared FFI |
| `vm/builtins/` | per-type built-in method implementations |
| `types/` | `Value` enum + type system |
| `env/` | lexical scope chain |
| `diagnostics/` | first-class diagnostic data model |
| `stdlib/` | standard library (`std::*`, `math`, `json`, `rand`, `time`) |
| `json/` | hand-written JSON ser/de |
| `http/` | HTTP client wrapper |

## Key entry points

- `vm::api::run_compiled` / `run_tests` — execute a program.
- `type_checker::TypeChecker::check_program` — static checking.
- `symbols` — the symbol registry every other layer reads.

## Invariants & gotchas

- `symbols.rs` is authoritative — the LSP reads it at runtime, so symbol changes
  propagate without manual sync.
- The two execution backends share one IR and one FFI and are kept in lockstep by
  compile-time + test-time guards — see `vm/README.md`.

## When you change this folder

- New keyword/type/method → start in `symbols.rs`; consistency tests enforce the
  rest.
- New submodule → add a `README.md` for it and link it in the table above.
