# `vm/builtins/` — Per-Type Method Implementations

## Purpose

The actual implementations of built-in methods, one file per type. `vm/mod.rs`'s
`builtin_method` routes a `(receiver, method)` pair here. Method-name dispatch uses
constants from `symbols.rs` (never raw string literals), so the compiler enforces
that every dispatched method is declared.

## Files

| File | Type(s) |
|---|---|
| `mod.rs` | Re-exports + shared dispatch glue. |
| `numeric.rs` | `int` / `byte` / `float` (all numeric values route through here). |
| `string.rs` | `String`. |
| `vec.rs` | `Vec<T>`. |
| `hashmap.rs` / `hashset.rs` | `HashMap` / `HashSet`. |
| `btreemap.rs` / `btreeset.rs` | `BTreeMap` / `BTreeSet`. |
| `binary_heap.rs` / `vec_deque.rs` | `BinaryHeap` / `VecDeque`. |
| `iterator.rs` | Iterator adapters (`map`/`filter`/`fold`/…, eager). |
| `option.rs` / `result.rs` | `Option` / `Result` combinators. |

## Key types & entry points

- Each file exposes a `dispatch(...)` and a `method_names()` helper.
- `method_names()` feeds the symbol-consistency tests.

## Invariants & gotchas

- **Use `symbols::<type>_m::CONSTANT`** in match arms — never a raw string. A method
  in dispatch but not in `symbols.rs` won't compile; a method in `symbols.rs` but not
  in dispatch fails a consistency test.
- All numeric widths must go through `numeric::dispatch`; all collection types must
  be handled — `dispatched_type_names()` + consistency tests catch gaps.
- Higher-order built-ins invoke user closures via `jit_closure_invoker`, which on
  wasm routes through the interpreter hook — keep that path intact.

## When you change this folder

Adding a method requires **all** of: constant + `MethodInfo` in `symbols.rs`, the
dispatch arm here using the constant, and the name in `method_names()`. See
`CONTRIBUTING.md` → "Adding a Built-in Method". Keep this file table current.
