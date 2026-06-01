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
| `check_expr.rs` | Expression type inference. Holds the `infer_expr` dispatcher plus the shared call/generic infrastructure (`resolve_generic_return`, `check_args_against_params`). Each non-trivial `Expr` variant is a focused `infer_<variant>` method, grouped into the `check_expr/` submodules below. |
| `check_expr/calls.rs` | `infer_call`, `infer_method_call`, `infer_path_call`, `infer_macro_call`. |
| `check_expr/operators.rs` | `infer_binary_op`, `infer_unary_op`, `infer_compound_assign`, `infer_as`, `infer_assign` (+ its `check_assign_root_mutable`). |
| `check_expr/control_flow.rs` | `infer_if`, `infer_if_let`, `infer_match` (+ exhaustiveness helpers), `infer_block`, `infer_return`. |
| `check_expr/data.rs` | `infer_struct_init`, `infer_field_access`, `infer_index`, `infer_tuple`, `infer_array`, `infer_repeat`, `infer_range`. |
| `check_expr/primary.rs` | `infer_ident` (+ `name_matches_known_symbol`), `infer_self_ref`, `infer_path`. |
| `check_expr/closures.rs` | `infer_closure`, `infer_async_block`, `infer_await`, `infer_try`. |
| `tests.rs` | Rust unit tests for the type checker. |

## Key types & entry points

- `TypeChecker` — fields: `struct_defs`, `type_aliases`, `fn_return_types`,
  `use_aliases`, `module_stack`, `current_impl_type`, `mutating_methods`.
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
- `infer_method_call` must run `check_path_visible` for resolved user/impl method
  keys so private methods remain module-scoped just like private functions.
- `collect_impl_methods` computes a fixed-point set of mutating methods per impl
  (`self` writes and `self.method()` edges). `infer_method_call` uses that set to
  reject mutating method calls on immutable (`val`) receiver bindings.

## When you change this folder

- New `Expr`/`Stmt`/`Item` → add inference/checking here.
- New built-in type/method → ensure `TypeInfo::from_name` and `symbols.rs` agree.
- A new `Expr` variant gets an `infer_<variant>` method in the matching
  `check_expr/` submodule **and** a dispatch arm in `infer_expr`. The submodule
  methods are `pub(super)` so the parent dispatcher can call them; keep them so.
- Keep this file table current.
