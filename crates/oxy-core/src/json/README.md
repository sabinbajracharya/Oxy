# `json/` — JSON Serialization / Deserialization

## Purpose

A hand-written JSON parser and serializer used by the `json::` stdlib module
(`parse`, `serialize`, `deserialize`). Hand-rolled (no serde) to keep the
dependency surface small and to map directly onto Oxy `Value`s.

## Files

| File | Responsibility |
|---|---|
| `mod.rs` | The JSON tokenizer/parser and the value→JSON serializer; converts between JSON text and Oxy `Value`. |

## Key types & entry points

- The parse entry (JSON text → `Value`) and serialize entry (`Value` → JSON text).
- Backs `stdlib/json.rs`.

## Invariants & gotchas

- Maps to/from the `Value` enum in `types/` — keep the two in sync when `Value`
  gains a representable variant.
- Must behave identically on both backends; it is plain Rust called through the
  shared layer, so there is no backend-specific path.

## When you change this folder

- Update `stdlib/json.rs` if the public surface changes.
- Keep the file table current.
