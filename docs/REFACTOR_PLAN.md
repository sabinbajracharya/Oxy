# Oxy Codebase Refactor & Documentation Plan

**Status:** proposed (2026-05-30)
**Goal:** improve readability, extensibility, separation of concerns, naming, and
discoverability across the whole repo — code, examples, tests, docs — **without
changing language behavior**. Every phase is behavior-preserving and gated on the
full pre-commit checklist (fmt + clippy + tests on all crates, plus the
JIT↔interpreter parity suite).

This plan is intentionally **phased and independently shippable**: each phase is a
self-contained PR that leaves the tree green. Order is chosen so low-risk,
high-leverage work (hygiene + docs) lands first and de-risks the larger
structural changes.

---

## Baseline (what's actually there today)

The architecture is already reasonably clean — prior work split `parser/`,
`type_checker/`, and `vm/` into focused submodules, and there is a real `docs/`
tree (ADRs + architecture notes). The problems are concentrated, not pervasive.

**Mega-files (the real pain):**

| File | Lines | Symptom |
|---|---|---|
| `crates/oxy-core/tests/vm_tests.rs` | 6130 | one giant test file, no categorization |
| `vm/jit/ir_gen/mod.rs` | 4156 | 218 fns in **one file** inside a dir built to hold many; `gen_expr` alone ≈ 860 lines |
| `vm/jit/ffi.rs` | 3128 | 107 `oxy_*` runtime fns, all domains in one file |
| `type_checker/check_expr.rs` | 1844 | only 14 fns → several very large functions |
| `parser/mod.rs` | 1742 | Pratt core + misc |
| `ast/mod.rs` | 1657 | every node in one module |
| `types/mod.rs` | 1502 | `Value` + type system + helpers |
| `lexer/mod.rs` | 1467 | tokenizer |
| `vm/jit/codegen.rs` | 1181 | IR → CLIF |
| `vm/interp.rs` | 1141 | IR interpreter |
| `parser/expr.rs` | 1141 | expression parsing |

**Stale docs (describe the removed bytecode/stack VM):**
- `docs/bytecode-vm.md`
- `docs/architecture/pattern-stack-contract.md`
- `docs/architecture/vm-locals-stack-separation.md`
- `docs/architecture/closure-capture-dense-renumbering.md`
- `docs/architecture/exhaustive-ast-walkers.md` (verify — may still apply)
- `IR_CFG_JIT_FIX_PLAN.md` (root) — completed work-tracking log, merged.

**Root clutter:** `core` (0-byte core dump), `.DS_Store`, `todo.ox`.

**Missing:** per-folder docs in source dirs (only `vm/jit/` has `IR_*.md`).

**Healthy:** only 27 `#[allow(...)]` markers and 5 TODO/FIXME — dead code is **not**
a big problem. Don't manufacture churn here.

---

## A tension to respect (do NOT "fix" this)

The brief asks for "loosely coupled, components independently replaceable." Most of
the codebase should move that way — **except one deliberate coupling**: the two
execution backends (Cranelift JIT + wasm IR interpreter) are *intentionally* tightly
bound by the divergence guards documented in `CLAUDE.md`:

1. `interp.rs`'s `match` over `IrOp`/`Terminator` has **no wildcard** — adding an op
   breaks the build until both backends handle it.
2. `ffi_decls()` vs `ffi_symbols()` consistency test.
3. The shared `oxy_*` FFI is the single runtime for both backends.

That coupling is load-bearing — it's what makes divergence *impossible by
construction*. **The refactor must not loosen it.** Where we introduce traits/seams,
they go around this core, not through it. Any change here follows the CLAUDE.md
push-back protocol.

---

## Phase 0 — Repo hygiene (tiny, do first)

Low risk, immediate clarity.

- Remove tracked junk: `core`, `.DS_Store`; add both + `*.DS_Store` to `.gitignore`
  if absent. Confirm `core` isn't referenced anywhere.
- Move `todo.ox` → `examples/` or delete (confirm with owner; it looks like a scratch file).
- Archive `IR_CFG_JIT_FIX_PLAN.md` → `docs/history/ir-cfg-jit-fix-log.md` (it's a
  finished log; keep for provenance, out of the root).
- Audit `docs/`: delete or rewrite the bytecode-era docs (see Phase 1).

**Gate:** `cargo build` + full test suite still green (no code touched).

---

## Phase 1 — Documentation system (per-folder docs + the AI/human rule)

This is the backbone the user asked for: *every folder has a markdown file telling
humans and AI what the code does*, and CLAUDE.md mandates keeping it current.

**Convention (decision needed — see end):** one `README.md` per source folder.
GitHub renders it inline; it's the universal "what is this folder" convention.

Each folder `README.md` contains:
- **Purpose** — one paragraph: what this module owns.
- **Files** — table: each file → one-line responsibility.
- **Key types / entry points** — the structs/fns a newcomer starts from.
- **Invariants / gotchas** — only the non-obvious (e.g. ir_gen's forward-ref
  resolution, the divergence guards).
- **When you change this folder** — what else must stay in sync.

Folders to document (15): `lexer/`, `parser/`, `ast/`, `type_checker/`, `types/`,
`env/`, `json/`, `http/`, `stdlib/`, `vm/`, `vm/builtins/`, `vm/jit/`,
`vm/jit/ir_gen/`, plus crate roots `oxy-cli/`, `oxy-lsp/`, `oxy-tug/`.

Reconcile the existing `vm/jit/IR_DESIGN.md`, `IR_SNAPSHOT_FORMAT.md`,
`IR_TEST_COVERAGE.md` — link them from `vm/jit/README.md` rather than duplicating.

Rewrite/replace stale `docs/`:
- Replace `docs/bytecode-vm.md` → `docs/execution-model.md` (register IR + two
  backends + shared FFI). This is the single canonical architecture doc.
- Re-validate each `docs/architecture/*.md` against current code; rewrite the ones
  that still describe stack/bytecode mechanics, delete the obsolete.

**CLAUDE.md + CONTRIBUTING.md updates:**
- New rule (CLAUDE.md, near top): *"When you add, remove, or change the
  responsibility of a file in a folder, update that folder's `README.md` in the same
  change. A PR that restructures a folder without updating its README is
  incomplete."*
- Add the per-folder README convention to the architecture map.
- CONTRIBUTING.md: fix the remaining stale "bytecode/stack-based VM" wording
  (lines ~29–32, 44, 105–107, 206 reference the old pipeline/`compiler/`).

**Gate:** docs only; no code. Verify links resolve.

---

## Phase 2 — Split the mega-files (behavior-preserving)

Pure module extraction: move code into focused files, fix `use`s, no logic changes.
Verify with snapshots + parity after each split.

**2a. `vm/jit/ir_gen/mod.rs` (4156 → ~6 files).** The dir already exists for exactly
this. Proposed split by lowering domain, keeping `IrGen` impl blocks across files:
- `mod.rs` — `IrGen` struct, state, `gen_program`, module-walk, public surface.
- `functions.rs` — `gen_fn` / `gen_method` / `gen_fn_named` / params/locals.
- `statements.rs` — `gen_stmt`, `gen_block_stmts`, `gen_store_lvalue`.
- `expressions.rs` — `gen_expr` (the 860-line monster) + helpers; consider further
  splitting calls/struct-init/operators into `expr_call.rs` if still > ~800 lines.
- `control_flow.rs` — `gen_if` / `gen_match` / `gen_while` / `gen_loop` / `gen_for_*`
  / `gen_if_let*` / short-circuit.
- `patterns.rs` — `gen_pattern_check` / `gen_pattern_bind`.
- `closures.rs` — `gen_closure` + capture handling.

**2b. `vm/jit/ffi.rs` (3128 → split by runtime domain).** Into a `vm/jit/ffi/` dir:
`arith.rs`, `collections.rs`, `strings.rs`, `structs_enums.rs`, `closures.rs`,
`async_rt.rs`, `io.rs`, plus `mod.rs` keeping `ffi_decls()`/`ffi_symbols()` (those
two lists **stay together** — the consistency test depends on them being co-located
and exhaustive).

**2c. `type_checker/check_expr.rs` (1844, 14 fns).** Identify the oversized fns and
extract by expr family (calls/paths, operators, literals/collections, match/if).

**2d. `tests/vm_tests.rs` (6130). ✅ DONE.** Split into `tests/vm_tests/` with a
`main.rs` harness (Cargo auto-discovers `tests/<dir>/main.rs` as the single `vm_tests`
target) plus 14 topic submodules (`basics`, `functions`, `control_flow`, `collections`,
`strings`, `structs_enums`, `traits_generics`, `error_handling`, `closures`, `modules`,
`stdlib`, `diagnostics`, `patterns`, `reference_syntax`). `main.rs` keeps the shared
imports + `run_and_capture`/`run_and_get_value`; submodules reach them via `use super::*`.
All 406 tests preserved (chunked by the existing `// === Section ===` dividers — no
brace-counting, since raw-string bodies hold col-0 `}`). One test target = compile time
unchanged. See `crates/oxy-core/tests/vm_tests/README.md`.

**2e. Stretch (only if they still hurt after 2a–2d):** `parser/mod.rs`,
`ast/mod.rs` (split node families into `ast/{expr,stmt,item,pattern,ty}.rs`),
`types/mod.rs` (`Value` vs type-system vs display), `codegen.rs`, `interp.rs`.

**Hard rule for this phase:** no behavior change. `UPDATE_SNAPSHOTS` must produce a
**zero diff**. If a snapshot changes, the "pure move" wasn't pure — revert and
re-split.

**Gate per sub-phase:** `cargo fmt` + `clippy -D warnings` (all crates) + `cargo
test -p oxy-core` + `--test ir_snapshot` (zero diff) + `--test jit_interp_parity`.

---

## Phase 3 — Decoupling & separation of concerns (the architectural work)

Done **after** files are small enough to reason about. Targeted, not sweeping.

- **Backend seam:** introduce a small `trait ExecutionBackend` (or similar) that
  both `JitEngine` and `InterpEngine` implement for the `api.rs` entry points, so
  backend selection is one polymorphic call instead of `#[cfg]`-scattered branches.
  *Constraint:* this wraps the backends; it does **not** touch the shared-IR / shared-FFI
  coupling or the exhaustive-match guard.
- **stdlib registry:** confirm every stdlib module plugs in purely through
  `stdlib::registry` (table-driven) with no special-casing leaking into the VM. Close
  any gaps so adding a stdlib module is "add a file + register," nothing else.
- **Symbol source-of-truth:** verify nothing bypasses `symbols.rs` (the anti-pattern
  list already forbids raw method-name strings — audit for stragglers).
- **Error types:** `FerriError` is a leftover from the Ferrite era (see CHANGELOG
  v0.2.1). **Decision: rename it**, but to a **language-name-independent** identifier
  so a future language rename doesn't drag the type name with it. Recommended:
  idiomatic `Error` (exposed as `errors::Error`) or `PipelineError` — confirm at
  rename time. Do **not** name it after "Oxy". Mechanical, wide diff → its own commit.

Each item is its own commit with a clear before/after rationale.

---

## Phase 4 — Naming, single-responsibility, dead code

Incremental, file-by-file as we touch each in Phases 2–3 (don't do a separate
mega-sweep — fold it into the splits to avoid double-churn).

- Functions that do >1 thing → extract named helpers; the name states the one thing.
- Variable/field/type names that don't say what they hold → rename (e.g. audit
  single-letter non-loop names, `tmp`, `data`, `val2`).
- Remove genuinely dead code behind `#[allow(dead_code)]` (8 sites) *if* truly unused;
  keep the ones annotated "used in later phases" with their comment intact.
- Resolve the 5 TODO/FIXME or convert to tracked issues.

**Rule:** match surrounding style; renames are mechanical and reviewed in isolation
from logic changes.

---

## Phase 5 — examples / tests / docs folder structure

- `examples/features/` is already well-categorized (18 categories). Add a short
  `examples/features/README.md` explaining the `#[test]` / `#[compile_error]`
  convention and how the harness globs them.
- `examples/leetcode/` and `examples/showcase/` — add brief READMEs.
- Consolidate the `tests/` story in one place (`crates/oxy-core/tests/README.md`):
  what each test binary covers (feature_examples, vm, ir_snapshot, parity, symbol
  consistency, extern_modules, leetcode, server_e2e).
- Top-level `docs/README.md` as the index/table-of-contents for all docs (ADRs,
  architecture, execution-model, history).

---

## Phase 6 — Enforcement & wrap-up

- Add a CI/pre-commit check (or a `tests/` doc-coverage test) asserting every source
  folder has a `README.md` — so the convention can't silently rot. (Mirrors the
  existing symbol-consistency-test philosophy.)
- Final pass over CLAUDE.md / CONTRIBUTING.md / README.md so all three agree with the
  post-refactor layout.
- Update CHANGELOG `## Unreleased` with an "Internals" entry summarizing the refactor.

---

## Execution guardrails (apply to every phase)

- **One concern per PR/commit**, conventional-commit prefixed (`refactor:`, `docs:`,
  `test:`, `chore:`). No co-author trailers.
- **Branch off `main`** (already on `ir-cfg-jit`; new work branches per phase).
- **Behavior-preserving phases prove it:** zero IR-snapshot diff + green parity.
- **Never edit a `.ox` test to make it pass.**
- **No magic offsets/guards** introduced to paper over a move (per JIT anti-patterns).
- Stop and re-plan if a "pure move" changes a snapshot — that means hidden coupling.

---

## Rough sequencing / size

| Phase | Risk | Leverage | Note |
|---|---|---|---|
| 0 Hygiene | trivial | low | minutes |
| 1 Docs | low | **high** | creates the map; do early |
| 2 Split files | medium | **high** | the bulk of the work |
| 3 Decouple | medium-high | medium | needs Phase 2 first |
| 4 Naming/dead | low | medium | folded into 2–3 |
| 5 Example/test docs | low | medium | |
| 6 Enforcement | low | medium | locks it in |
