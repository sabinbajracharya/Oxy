# `oxy-lsp` — Language Server

## Purpose

The Oxy language server (built on `tower-lsp`): diagnostics, completions, hover, and
go-to-definition for editors (the VS Code extension is the primary client). It reads
language symbols from `oxy-core`'s `symbols.rs` at runtime, so it never hardcodes
keywords/types/methods and stays in sync automatically.

## Files

| File | Responsibility |
|---|---|
| `src/main.rs` | The LSP server: request handlers (completion/hover/diagnostics/goto), document state, and the `oxy-core` integration that powers them. |

## Key behaviors

- **Diagnostics** — surfaces lexer/parser/type-checker errors from `oxy-core`.
- **Completions / hover** — driven by `symbols.rs` (keywords, types, methods,
  modules, macros).
- **Goto-definition** — AST-aware resolution.

## Invariants & gotchas

- Never hardcode a keyword/type/method name here — read it from `oxy_core::symbols`.
  This is what keeps the editor experience consistent with the compiler.
- When `oxy-core` adds AST nodes, keywords, or built-ins, verify the LSP still
  compiles and that completions/hover reflect them (`cargo test -p oxy-lsp`).

## When you change this folder

- Update completions/hover/diagnostics for new user-facing language features.
- Keep the file table current if `main.rs` is split.
