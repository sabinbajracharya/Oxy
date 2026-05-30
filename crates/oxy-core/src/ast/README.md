# `ast/` — Abstract Syntax Tree

## Purpose

Defines the node types the parser produces and every later stage consumes
(type checker, IR generation). This is the shared vocabulary of the whole front
end — `Program`, `Item`, `Expr`, `Stmt`, and their supporting structs/enums.

## Files

| File | Responsibility |
|---|---|
| `mod.rs` | All AST node definitions: `Program`, `Item` (Function/Struct/Enum/Impl/Trait/Module/Use), `Expr`, `Stmt`, `Pattern`, `FnDef`, `StructDef`, type annotations, etc. |

> `mod.rs` is large (one node family per logical section). If it is split, the
> natural seams are `expr.rs` / `stmt.rs` / `item.rs` / `pattern.rs` / `ty.rs`
> mirroring the `parser/` layout.

## Key types & entry points

- `Program` — the root: a list of `Item`s.
- `Expr` / `Stmt` — the expression and statement node enums (walked everywhere).
- `Item` — top-level declarations.
- Type-annotation nodes — note Oxy rejects `&T`, lifetimes, and the integer-width
  zoo; those constraints are enforced in the parser/type-checker, not by omitting
  the nodes here.

## Invariants & gotchas

- AST nodes are plain data — **no behavior**. Inference/lowering logic belongs in
  `type_checker/` and `vm/jit/ir_gen/`, not here.
- Adding a variant ripples outward: parser must produce it, type checker must check
  it, `ir_gen` must lower it, and (because `interp.rs` has no wildcard match) both
  backends must handle whatever IR it lowers to.

## When you change this folder

- New `Expr`/`Stmt`/`Item` variant → update `parser/`, `type_checker/`,
  `vm/jit/ir_gen/`, and the exhaustive AST walkers (see
  `docs/architecture/exhaustive-ast-walkers.md`).
- Keep this README's file table current if `mod.rs` is split.
