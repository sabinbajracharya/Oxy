# `types/` — Runtime Values & Type System

## Purpose

Defines `Value` — the runtime representation of every Oxy value — plus the
type-system helpers shared by the type checker and the runtime (type names,
ordering, display). This is the data the `oxy_*` FFI operates on.

## Files

| File | Responsibility |
|---|---|
| `mod.rs` | The `Value` enum (int/byte/float/String/Vec/HashMap/struct/enum/closure/Option/Result/…), `type_name`, ordering, conversions, and display helpers. |

## Key types & entry points

- `Value` — the central runtime enum. Collections are reference-counted
  (`Rc<RefCell<…>>`) so assignment shares data; `.clone()` makes independent copies.
- `type_name` — canonical type-name strings; must agree with `symbols.rs`.

## Invariants & gotchas

- Oxy has exactly three numeric types: `int` (signed 64-bit wrapping), `byte`
  (unsigned 8-bit wrapping), `float` (64-bit IEEE-754). No width zoo.
- Reference semantics via `Rc<RefCell<>>` are deliberate — there is no borrow
  checker. Don't add ownership/borrow concepts.
- A new `Value` variant must be wired through dispatch (`vm/builtins/`), the FFI
  (`vm/jit/ffi.rs`), and `symbols.rs`.

## When you change this folder

- New `Value` variant → `vm/builtins/<type>.rs` dispatch, `vm/mod.rs`
  `dispatched_type_names()`, and `symbols.rs` (`ALL_TYPES`, method constants).
- Keep this README current if `mod.rs` is split (natural seams: `value.rs`,
  `type_info.rs`, `display.rs`).
