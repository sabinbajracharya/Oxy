# `oxy-cli` — The `oxy` Compiler Binary

## Purpose

The `oxy` command-line binary: runs Oxy source files, runs `#[test]` /
`#[compile_error]` functions, starts the REPL, and exposes debugging dumps. It is the
thin front end over `oxy-core`; it owns argument parsing and process exit codes, not
language semantics.

## Files

| File | Responsibility |
|---|---|
| `src/main.rs` | CLI entry: arg parsing, subcommand dispatch (`run`, `test`, `repl`), and the `--dump-*` debug flags. Delegates all real work to `oxy-core`. |

## Commands & options

```
oxy run <file.ox>          Run an Oxy source file
oxy test <file.ox>         Run #[test] and #[compile_error] functions
oxy repl                   Start the interactive REPL

--extern <name>=<path>     Register an external module dependency
--dump-tokens <file>       Lexer output
--dump-ast <file>          Parser AST output
--dump-ir <file>           Lowered register IR  (alias: --dump-bytecode, hidden)
```

## Invariants & gotchas

- `--dump-bytecode` is a **hidden** back-compat alias for `--dump-ir`; Oxy compiles
  through a register IR, not bytecode. Keep it hidden from `--help`.
- Package management lives in `tug`, not here — `oxy` has no `install`/`list`.
  Dependencies arrive via `--extern <name>=<path>`.

## When you change this folder

- New subcommand/flag → update `README.md` (here), the root `README.md` CLI section,
  and `CONTRIBUTING.md` if it's a debugging aid.
