# `vm/jit/ir_gen/` — AST → Register IR + CFG

## Purpose

Lowers the type-checked AST into the register IR (`IrFunction`s with basic blocks,
`IrOp`s, and `Terminator`s) that both backends execute. This is where control flow
becomes a CFG and expressions become register operations.

## Files

| File | Responsibility |
|---|---|
| `mod.rs` | The entire `IrGen` lowering pass: `gen_program`, `gen_fn`/`gen_method`, `gen_stmt`, `gen_expr`, control flow (`gen_if`/`gen_match`/`gen_while`/`gen_for_*`), patterns, closures. |

> `mod.rs` is the largest file in the tree (~4k lines). It lives in its own directory
> precisely so it can be split by lowering domain (functions / statements /
> expressions / control_flow / patterns / closures) — a planned refactor.

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
