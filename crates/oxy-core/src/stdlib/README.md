# `stdlib/` — Standard Library Modules

## Purpose

Implements Oxy's standard-library surface (`std::*`, `math::`, `json::`, `rand::`,
`time::`). Dispatch is **table-driven** through `registry.rs`, so adding a module is
"add a file + register it" — no special-casing leaks into the VM.

## Files

| File | Responsibility |
|---|---|
| `mod.rs` | Stdlib registration entry; wires modules into the table-driven registry. |
| `registry.rs` | Table-driven built-in dispatch — the mechanism every module plugs into. |
| `args.rs` | `std::args::parse(spec)` — CLI argument parser. |
| `db.rs` | `std::db` — bundled SQLite client. |
| `env.rs` | `std::env` — `args`, `var`, `vars`, `current_dir`, `home_dir`. |
| `fs.rs` | `std::fs` — file system operations. |
| `http.rs` | `std::http` — HTTP client surface (wraps `crate::http`). |
| `io.rs` | `std::io` — stdin reading. |
| `json.rs` | `json::` — wraps `crate::json`. |
| `math.rs` | `math::` — numeric functions and constants. |
| `net.rs` | `std::net` — TCP/UDP, host lookup. |
| `path.rs` | `std::path` — lexical path manipulation. |
| `process.rs` | `std::process` — `command` + streaming `spawn`. |
| `rand.rs` | `rand::` — random numbers. |
| `regex.rs` | `std::regex` — OOP-style `Regex`. |
| `server.rs` | `std::server` — closure-callback HTTP server. |
| `time.rs` | `time::` — clocks/elapsed. |

## Key types & entry points

- `registry.rs` — the dispatch table; the single integration point for modules.
- `mod.rs` — registers each module.

## Invariants & gotchas

- Register through the table; do **not** add bespoke matches in the VM for a stdlib
  call.
- Native-only modules (`fs`, `net`, `http`, `process`, `db`, `server`) can't run on
  the wasm interpreter — see the `unsupported_on_wasm!` / closure-invoker discussion
  in `vm/README.md` and `CLAUDE.md`.

## When you change this folder

- New module → add `<name>.rs`, register in `mod.rs`/`registry.rs`, add `.ox`
  feature tests, and (if it adds names) update `symbols.rs` (`ALL_MODULES`).
- Keep this file table current.
