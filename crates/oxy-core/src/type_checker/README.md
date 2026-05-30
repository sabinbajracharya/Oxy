# `type_checker/` — Static Type Checking

## Purpose

Validates the AST before lowering: type inference/checking, name resolution,
visibility enforcement (private fields/items), and registration of struct/fn/method
types. Runs after parsing and before `ir_gen`. A `#[compile_error]` test passes if
this stage (or codegen) rejects the program.

## Files

| File | Responsibility |
|---|---|
| `mod.rs` | `TypeChecker` struct + state, `check_program` (collect → collect_fn_types → check_item), `TypeInfo`, and `from_name` (the single source of truth for type-name → `TypeInfo`). |
| `collect.rs` | `collect_defs` (structs, type aliases, use aliases) + `collect_fn_types` (fn/method return types). |
| `resolve.rs` | Name resolution: paths, use-aliases, module qualification. |
| `check_item.rs` | Type-checking item bodies (fns, impls, traits, modules). |
| `check_stmt.rs` | Statement checking (`let`, `use`, control flow). |
| `check_expr.rs` | Expression type inference (calls, paths, operators, field access, match). |
| `tests.rs` | Rust unit tests for the type checker. |

## Key types & entry points

- `TypeChecker` — fields: `struct_defs`, `type_aliases`, `fn_return_types`,
  `use_aliases`, `module_stack`, `current_impl_type`.
- `check_program()` — order matters: `collect_defs` → `collect_fn_types` →
  `check_item`.
- `TypeInfo::from_name` — **always** use this for type-name conversion; never
  inline-match type-name strings (a partial match with `_ => Unknown` silently
  accepts anything because `accepts()` returns true for `Unknown`).

## Invariants & gotchas

- `collect_fn_types` **must** handle `Item::Impl` and `Item::ImplTrait`, registering
  methods under both `"Type::method"` and `"prefix::Type::method"`. Skipping them
  leaves return types `Unknown` and breaks field-visibility checks.
- `Stmt::Use` **must** populate `use_aliases` — it is not a no-op.
- Visibility (`check_field_visible`, `check_path_visible`) compares the defining
  module against `module_stack`. Use `module_names.contains(parent)` for top-level
  detection, never `contains("::")`.

## When you change this folder

- New `Expr`/`Stmt`/`Item` → add inference/checking here.
- New built-in type/method → ensure `TypeInfo::from_name` and `symbols.rs` agree.
- Keep this file table current; `check_expr.rs` is a split candidate (large).
