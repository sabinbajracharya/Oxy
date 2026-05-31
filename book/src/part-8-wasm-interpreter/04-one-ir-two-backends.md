# One IR, Two Backends: The Elegance of the Design

<!-- OPUS_FILL
Write a 3-paragraph narrative. This is the chapter where you step back and appreciate
the architecture as a whole.

The elegance: you designed the system so that adding a new language feature requires
exactly one implementation (in the FFI), and both backends get it for free.

Compare to the alternative (two separate interpreters, or a JIT + tree-walker):
in those approaches, every feature needs two implementations. Bugs diverge. Tests
are backend-specific. Maintenance doubles.

End with: "This is the payoff for the shared-runtime discipline. Every hard design
choice (register IR, FFI-mediated runtime, the JitContext buffer) was made to enable
exactly this: one place to implement features, two places to run them."
-->

## The architecture table

| Layer | Lives in | Used by |
|-------|---------|---------|
| **IR gen** (AST → `IrFunction`) | `vm/jit/ir_gen/` | Both backends |
| **Register IR types** (`IrOp`, `Terminator`) | `vm/jit/ir.rs` | Both backends |
| **Runtime (FFI)** (`oxy_*` functions) | `vm/jit/ffi/` | Both backends |
| **Value representation** | `types/mod.rs` | Both backends |
| **JitContext buffer** | `vm/jit/context.rs` | Both backends |
| **Cranelift JIT** (compile IR → native code) | `vm/jit/codegen.rs` | Native only |
| **IR interpreter** (walk IR → Value) | `vm/interp.rs` | wasm32 + parity tests |

The split: everything above the line is shared. Below the line is backend-specific.
The shared layer is where Oxy's language semantics live. The backend-specific layer is
just "how do we execute the IR" — and that answer differs by platform.

## What "adding a feature" looks like

Suppose you add `Vec::retain(closure)` — a method that keeps only elements satisfying a predicate.

**What you do:**
1. Add `retain` to `symbols.rs` (method name constant)
2. Add `oxy_vec_retain` to `ffi/collections.rs` (the implementation)
3. Add `oxy_vec_retain` to `ffi_symbols()` and `ffi_decls()`
4. Write a `#[test]` in `examples/features/collections/vec_retain.ox`

**What you get for free:**
- Native execution via the JIT (calls `oxy_vec_retain` from compiled code)
- wasm32 execution via the interpreter (calls `oxy_vec_retain` via the FFI table)
- Parity between both backends (they call the same function)
- LSP completion (reads from `symbols.rs`)

**What you do NOT do:**
- Write a JIT codegen for `Vec::retain` (not needed — it's a `CallBuiltin`)
- Write an interpreter case for `Vec::retain` (not needed — the interpreter dispatches to FFI)
- Write separate tests for the interpreter path (the parity test runs the same `.ox` test on both)

This is the payoff: one implementation, two backends, zero divergence.

## Comparing to the alternative

What if we had used a tree-walker for wasm32 instead of the IR interpreter?

```
Native path: AST → type check → ir_gen → JIT → native code
Wasm path:   AST → type check → tree-walk → result
```

Every time you add `Vec::retain`:
- JIT path: add `oxy_vec_retain`, wire into IR gen as `CallBuiltin`
- Wasm path: add a separate `eval_vec_retain` to the tree-walker
- Test: one test suite per backend, or a complex comparison harness

Two implementations. Two places to have bugs. Two places to update when the semantics change.

The actual design — one IR, one FFI runtime, two executors — has none of these problems.
The tree-walker was retired partly because it blocked this convergence. The IR interpreter
was written specifically to enable it.

## The constraint that enforced the design

The decision to compile `interp.rs` on **all** targets (not just wasm32) was the architectural
commitment that made this design work. It means: you cannot write IR-gen code that "works on
native but is silently broken on wasm." The interpreter's exhaustive match catches it at build time.

The alternative — compile `interp.rs` only on wasm32 — would mean native builds never
see the interpreter, and wasm-only bugs silently accumulate until someone tests the playground.

Compiling on all targets costs almost nothing (a few extra seconds of compilation). It buys:
the interpreter is always tested, always type-checked, always up-to-date with the IR.
