# `parser/` — Pratt Parser

## Purpose

Turns the lexer's `Vec<Token>` into an AST `Program`. A Pratt (precedence-climbing)
parser with precedence levels 0–14. Also the layer that **rejects** non-Oxy syntax
(`&T`, `&mut`, lifetimes, slice param types) with fix-it error messages.

## Files

| File | Responsibility |
|---|---|
| `mod.rs` | `Parser` struct, `parse_program`, the Pratt core (precedence table, token cursor, error recovery). |
| `expr.rs` | Expression parsing (the bulk of the Pratt grammar). |
| `item.rs` | Item parsing: `fn`, `struct`, `enum`, `impl`, `trait`, `mod`, `use`. |
| `stmt.rs` | Statement parsing: `let`, `use`, `if`, `while`, `for`, `return`. |
| `pattern.rs` | Pattern parsing for match arms and `let` destructuring. |
| `ty.rs` | Type-annotation parsing (and the reference/lifetime/width rejections). |

## Key types & entry points

- `Parser` — owns the token stream and cursor.
- `parse_program` — the public entry; returns a `Program` or a parse error.
- The precedence table in `mod.rs` — the single source of operator precedence.

## Invariants & gotchas

- Oxy is "dynamic Rust": `&T`, `&mut T`, `&self`, `&str`, `&[T]`, lifetimes `'a`,
  and suffixed-width literals are **rejected here with fix-its** — not silently
  accepted. See the top of `CLAUDE.md`. Do not weaken these.
- Precedence is centralized in `mod.rs`; don't scatter precedence constants.

## When you change this folder

- New operator → add precedence in `mod.rs`, parse in `expr.rs`, and update the
  VS Code grammar.
- New syntax form → also update AST, type checker, `ir_gen`, and LSP.
- Keep this file table current.
