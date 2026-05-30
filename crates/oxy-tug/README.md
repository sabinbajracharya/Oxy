# `oxy-tug` — The `tug` Package Manager

## Purpose

`tug` is to `oxy` what `cargo` is to `rustc`: it owns project layout, dependency
management, and orchestration of the `oxy` compiler. It scaffolds projects, edits
`tug.toml` / `tug.lock`, manages a package store at `~/.oxy/packages/`, and shells
out to `oxy` with the right `--extern <name>=<path>` flags.

## Files

| File | Responsibility |
|---|---|
| `src/main.rs` | Process entry — collects args, calls `cli::dispatch`, sets exit code. |
| `src/cli.rs` | CLI dispatch: maps subcommands (`new`/`init`/`build`/`run`/`test`/`add`/`remove`/`install`/`uninstall`/`list`) to the modules below. |
| `src/lib.rs` | Library surface re-exporting the modules (so tests and `cli.rs` share them). |
| `src/scaffold.rs` | `tug new` / `tug init` — project scaffolding. |
| `src/manifest.rs` | `tug.toml` parsing/editing. |
| `src/lockfile.rs` | `tug.lock` management. |
| `src/project.rs` | Project resolution (`Project`, root/entry discovery). |
| `src/install.rs` | `tug install` / `uninstall` / `list` — the `~/.oxy/packages/` store. |
| `src/runner.rs` | `tug build` / `run` / `test` — resolves deps and invokes `oxy`. |

## Key types & entry points

- `cli::dispatch(args) -> i32` — the single CLI entry; returns the exit code.
- `Project` (`project.rs`) — resolved project root + entry file.
- `runner::{run_project, test_project, build_project}` — locate `oxy`, thread
  `--extern`, run.

## Invariants & gotchas

- The `oxy` binary is located via `$TUG_OXY_PATH` → sibling of the `tug` binary →
  `PATH` (see `runner.rs` docs).
- `tug build` type-checks by invoking `oxy --dump-ir` and discarding output (there is
  no separate object-file step for a script-style language).
- Manifest/lockfile are both TOML.

## When you change this folder

- New subcommand → wire it in `cli.rs`, implement in the relevant module, update this
  README and the root `README.md` `tug` section.
- Integration tests live in `crates/oxy-tug/tests/`.
