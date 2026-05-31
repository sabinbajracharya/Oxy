# Oxy Documentation

Project-level docs. For per-folder code docs, see the `README.md` inside each source
folder (e.g. `crates/oxy-core/src/vm/README.md`). The single source of truth for the
build/test workflow and project conventions is the root [`CLAUDE.md`](../CLAUDE.md).

## Architecture

| Doc | What it covers |
|---|---|
| [`execution-model.md`](execution-model.md) | **Start here.** The pipeline, the register IR, the two backends (Cranelift JIT + wasm IR interpreter), the shared `oxy_*` FFI, and the divergence guards. |
| [`architecture/exhaustive-ast-walkers.md`](architecture/exhaustive-ast-walkers.md) | Why match statements over `Expr`/`Stmt`/`IrOp` are exhaustive (a correctness guard). |
| `crates/oxy-core/src/vm/jit/IR_DESIGN.md` | Canonical register-IR design & invariants. |
| `crates/oxy-core/src/vm/jit/IR_SNAPSHOT_FORMAT.md` | IR snapshot serialization format (implemented). |
| [`history/ir-test-coverage-plan.md`](history/ir-test-coverage-plan.md) | Retired: the (now-fulfilled) IR snapshot test coverage plan. |

## Decision records (ADRs)

| ADR | Decision |
|---|---|
| [`adr/001-unified-jit-calling.md`](adr/001-unified-jit-calling.md) | Unified JIT function calling. |
| [`adr/002-eliminate-global-jit-tables.md`](adr/002-eliminate-global-jit-tables.md) | Per-engine `JitTables` (no globals). |

## Planning

| Doc | What it covers |
|---|---|
| [`REFACTOR_PLAN.md`](REFACTOR_PLAN.md) | The active codebase refactor & documentation plan (phased). |

## History (retired — provenance only)

These describe subsystems that **no longer exist** (the tree-walking interpreter and
the stack-based bytecode VM). Kept for context; do not treat as current architecture.

- [`history/bytecode-vm`](history/) (removed) → superseded by `execution-model.md`
- [`history/vm-locals-stack-separation.md`](history/vm-locals-stack-separation.md)
- [`history/pattern-stack-contract.md`](history/pattern-stack-contract.md)
- [`history/closure-capture-dense-renumbering.md`](history/closure-capture-dense-renumbering.md)
- [`history/ir-cfg-jit-fix-log.md`](history/ir-cfg-jit-fix-log.md) — the work log for the IR/CFG/JIT migration.
