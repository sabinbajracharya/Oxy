# `env/` — Lexical Scope Chain

## Purpose

Lexical scoping with a parent chain. Each scope holds variable bindings and their
mutability. Used where name→value resolution needs a runtime scope (notably the
IR interpreter path and closure capture).

## Files

| File | Responsibility |
|---|---|
| `mod.rs` | `Environment` + the `Env = Rc<RefCell<Environment>>` alias; bindings, mutability flags, and parent-chain lookup. |

## Key types & entry points

- `Environment` — one scope: a `HashMap` of bindings plus an optional parent.
- `Env` — the shared, reference-counted handle threaded through scopes.

## Invariants & gotchas

- Mutability here is **binding-level** (`let` vs `let mut`), not borrow checking —
  it controls reassignment only, like `const`/`let` in JS.
- Scopes are `Rc<RefCell<>>`-shared; mutating through one handle is visible through
  others by design (closure capture relies on this).

## When you change this folder

- Changes to scoping/capture semantics must stay consistent across both execution
  backends (JIT and interpreter) — see `vm/README.md`.
- Keep the file table current.
