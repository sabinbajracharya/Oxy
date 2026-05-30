# `http/` — HTTP Client

## Purpose

A thin HTTP client wrapper (over `ureq`) backing the `std::http` stdlib surface.
Keeps the third-party client behind one module so it can be swapped without
touching call sites.

## Files

| File | Responsibility |
|---|---|
| `mod.rs` | HTTP request/response helpers wrapping `ureq`; the client used by `stdlib/http.rs`. |

## Key types & entry points

- The request helpers consumed by `stdlib/http.rs`.

## Invariants & gotchas

- Network I/O is native-only — it cannot run in the wasm playground. Anything that
  reaches this from the runtime must be gated/avoided on `wasm32`.
- This module isolates the `ureq` dependency; keep the wrapper boundary clean so
  the client stays replaceable.

## When you change this folder

- Update `stdlib/http.rs` and the README table if the surface changes.
