# `lexer/` — Tokenizer

## Purpose

Turns Oxy source text into a flat `Vec<Token>` for the parser. This is the first
stage of the pipeline (`parse → type_check → ir_gen → codegen`). It also emits the
fix-it errors that enforce Oxy's "dynamic Rust" identity at the token level (e.g.
rejecting `&`, lifetimes, and suffixed integer literals where they can be caught
early).

## Files

| File | Responsibility |
|---|---|
| `mod.rs` | The tokenizer: scans characters, produces `Token`s, tracks line/column spans, handles strings/numbers/comments/operators. |
| `token.rs` | `Token`, `TokenKind`, and `Span` (1-indexed line/column). The token vocabulary lives here. |

## Key types & entry points

- `TokenKind` — the full token vocabulary; keep it aligned with `symbols::KEYWORDS`.
- `Span` — **1-indexed** line/column; downstream errors and the LSP rely on this.
- The public tokenize entry point in `mod.rs`.

## Invariants & gotchas

- Spans are **1-indexed**. Off-by-one here corrupts every downstream diagnostic.
- Keyword recognition must stay consistent with `symbols.rs` — never hardcode a
  keyword string that isn't also in `symbols::KEYWORDS`.

## When you change this folder

- New keyword/operator → update `symbols.rs` and the VS Code grammar
  (`editors/vscode/syntaxes/oxy.tmLanguage.json`).
- Update this README's file table if you split `mod.rs`.
