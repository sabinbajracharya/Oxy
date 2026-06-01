# `diagnostics/` — Structured Diagnostic Model

## Purpose

Defines a first-class diagnostics domain model shared across compiler stages
and frontends (CLI/LSP), so errors are not just free-form strings.

## Files

| File | Responsibility |
|---|---|
| `mod.rs` | Core diagnostic types (`Diagnostic`, labels, notes/help, fix-its) and builder helpers. |
| `codes.rs` | Stable diagnostic code constants (`LEX*`, `PAR*`, `TYP*`, `RUN*`). |

## Key types

- `Diagnostic`: code + severity + category + message + labels + notes + fix-its.
- `Label`: primary/secondary highlighted span with optional message.
- `Note`: `note:` / `help:` context.
- `FixIt`: structured source edits for suggested fixes.

## Invariants

- Every user-facing error should eventually map to one `Diagnostic`.
- Primary span should be present when source location is known.
- Error code strings are stable identifiers for docs/search/tooling.

## When you change this folder

- Add new code constants in `codes.rs` rather than scattering literals.
- Keep `Diagnostic` additive/backward-compatible where possible so CLI/LSP
  adapters do not churn.
