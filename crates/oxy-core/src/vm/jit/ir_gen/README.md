# `vm/jit/ir_gen/` — AST → Register IR + CFG

## Purpose

Lowers the type-checked AST into the register IR (`IrFunction`s with basic blocks,
`IrOp`s, and `Terminator`s) that both backends execute. This is where control flow
becomes a CFG and expressions become register operations.

## Files

The `IrGen` struct and its lowering methods are split across one `impl IrGen` per
file. `mod.rs` owns the struct, state, and core helpers; each domain file adds an
`impl IrGen` block (cross-file methods are `pub(super)`).

| File | Responsibility |
|---|---|
| `mod.rs` | The `IrGen` struct + all state fields, `new`, register allocation (`alloc_reg`/`alloc_block`/`alloc_local`), `emit`/`terminate`/`start_block`, `coerce_reg*`, `lookup_local`, `register_enum`, and the submodule wiring. |
| `functions.rs` | Program / module / function lowering: `gen_program`, `gen_module_items`, `gen_fn`/`gen_method`/`gen_fn_named`, and the use/glob/generic-fn registration helpers. |
| `resolve.rs` | Name, path, and type-alias resolution: `resolve_module_path`, `resolve_use_path`, `resolve_callable_name`, `type_ann_to_type_info`, etc. |
| `statements.rs` | `gen_block_stmts`, `gen_stmt`, `gen_store_lvalue`. |
| `expressions.rs` | `gen_expr` (the large expression dispatcher) + `gen_short_circuit`. |
| `control_flow.rs` | `gen_if`/`gen_if_let*`, `gen_match`, `gen_while`/`gen_while_let`, `gen_loop`, `gen_for_in`/`gen_for_destructure`. |
| `patterns.rs` | `gen_pattern_check`, `gen_pattern_bind`. |
| `closures.rs` | `gen_closure` + free-variable analysis (`collect_free_vars`/`collect_idents*`). |
| `tests.rs` | `#[cfg(test)]` unit tests for lowering. |

> `expressions.rs` is still large because `gen_expr` is one cohesive dispatcher; a
> finer split (calls / struct-init / operators) is a possible follow-up.

## Compilation pipeline

1. `gen_program()` — iterate top-level items, dispatch to `gen_fn` / `gen_module_items`.
2. `gen_fn()` — create `IrFunction`, allocate locals for params, lower the body.
3. `gen_stmt()` / `gen_expr()` — walk the AST, emit `IrOp` + `Terminator` into blocks.
4. `gen_module_items()` — recurse with cumulative `"parent::child"` prefix.

## Key types & entry points

- `IrGen` — the lowering state (locals, blocks, use-aliases, variant→enum map).
- `gen_program` — entry point.
- Names are fully qualified (`"parent::child::fn"`); the JIT resolves by name at call
  time, so definition order doesn't matter.

## Invariants & gotchas

- `Expr::Call` resolution order: closure local → use-alias → enum-variant ctor →
  built-in FFI (`spawn`/`sleep`/`select`) → `oxy_call` with the qualified name.
- `Expr::StructInit`: enum-variant ctor → `oxy_struct_update` (if `base`) →
  `oxy_struct_init`.
- Whatever IR you emit here, `../../interp.rs` must be able to interpret (no-wildcard
  match) — adding a new `IrOp` ripples to codegen **and** the interpreter.

## When you change this folder

- Changing lowering output → regenerate IR snapshots
  (`UPDATE_SNAPSHOTS=1 cargo test -p oxy-core ir_snapshot`) and run the parity suite.
- If you split `mod.rs`, update this file table to map each new file.
