# The Divergence Guards: How You Stop Backends from Drifting

"One feature, two backends, zero divergence" is a lovely promise, and the previous chapter made it
sound like it falls out of the architecture for free. It doesn't, quite. Sharing the runtime makes
divergence *unlikely*, but nothing about it makes divergence *impossible* — and the failure mode is
nasty precisely because it's silent. You add an op to the IR and teach the JIT about it; the native
tests pass; you ship. Months later someone opens the browser playground, hits a program that uses
that op, and gets garbage, because the interpreter never learned the new op and nobody noticed. The
gap between "works on my machine" and "works in the browser" is exactly where backends rot apart.

So Oxy treats divergence the way a body treats infection: with an immune system. Three layers of
defense, each tuned to a different class of drift, each designed to turn a silent runtime
divergence into a loud, early, unmissable failure. The first catches a missing op at *compile time*.
The second catches a mismatched FFI surface at *test time*. The third runs the real corpus through
both backends and catches any behavioral disagreement at *runtime*. These are not bureaucratic
box-ticking tests — they are the actual mechanism that makes the previous chapter's promise true in
practice rather than just in spirit. Take them away and "one feature, two backends" quietly becomes
a lie.

## Guard 1: Exhaustive match (compile time)

The interpreter's `exec_op` function has a `match op { ... }` over all `IrOp` variants
with **no wildcard arm**:

```rust
// vm/interp.rs
fn exec_op(&self, ctx: &mut JitContext, op: &IrOp, regs: &mut ...) {
    match op {
        IrOp::ConstInt(r, n) => { ... }
        IrOp::ConstFloat(r, n) => { ... }
        IrOp::ConstBool(r, b) => { ... }
        // ... every variant
        IrOp::Phi(r, a, b) => { ... }
        // NO _ => {} ARM
    }
}
```

Rust's exhaustive match checker verifies that all variants are handled. If you add
`IrOp::NewOp` to `ir.rs` without adding a case to `exec_op`, the compiler error is:

```
error[E0004]: non-exhaustive patterns: `IrOp::NewOp` not covered
  --> src/vm/interp.rs:272:9
```

Because `interp.rs` is compiled on all targets (including native), this error appears
on every `cargo build` — not just on wasm builds. You cannot ship a native binary that
silently breaks the interpreter.

The same exhaustive match is applied to `Terminator`:

```rust
match &block.terminator {
    Terminator::Return(_) => { ... }
    Terminator::Jump(_) => { ... }
    Terminator::Branch { .. } => { ... }
    Terminator::Halt => { ... }
    Terminator::Panic(_) => { ... }
    // NO wildcard
}
```

Every new `Terminator` variant must be handled in the interpreter before the code compiles.

## Guard 2: FFI surface consistency (test time)

Two independent lists describe the FFI:

```rust
// codegen.rs - what the JIT declares to Cranelift
pub fn ffi_decls() -> Vec<(&'static str, Vec<types::Type>, Option<types::Type>)> {
    vec![
        ("oxy_push_int", vec![I64, I64], None),
        ("oxy_add", vec![I64], None),
        // ...
    ]
}

// ffi/mod.rs - what the interpreter can dispatch to
pub fn ffi_symbols() -> HashMap<&'static str, (*const u8, FfiRet)> {
    let mut m = HashMap::new();
    m.insert("oxy_push_int", (oxy_push_int as *const u8, FfiRet::Void));
    m.insert("oxy_add", (oxy_add as *const u8, FfiRet::Void));
    // ...
}
```

The consistency test:

```rust
// jit/mod.rs tests
#[test]
fn ffi_consistency() {
    let decl_names: HashSet<&str> = ffi_decls().into_iter().map(|(n, ..)| n).collect();
    let sym_names: HashSet<&str> = ffi_symbols().keys().copied().collect();
    assert_eq!(decl_names, sym_names,
        "ffi_decls and ffi_symbols must list the same functions"
    );
}
```

Scenarios:
- Add `oxy_new_fn` to `ffi_decls()` but not `ffi_symbols()` → test fails. The JIT would
  declare the function but the interpreter couldn't dispatch to it.
- Add to `ffi_symbols()` but not `ffi_decls()` → test fails. The interpreter could dispatch
  but the JIT would crash at declaration time.
- Add to both → test passes. Both backends can use the function.

## Guard 3: JIT↔interpreter parity test (runtime)

```bash
docker compose run --rm dev bash -c "cargo test -p oxy-core --test jit_interp_parity"
```

This test runs every `.ox` file in `examples/features/**` through both backends and diffs
the output:

```rust
// tests/jit_interp_parity.rs (simplified)
for test_file in glob("examples/features/**/*.ox") {
    let jit_output = run_on_jit(source);
    let interp_output = run_on_interpreter(source);
    assert_eq!(jit_output, interp_output,
        "{}: JIT and interpreter disagree", test_file
    );
}
```

If a feature works differently on the two backends, this test catches it.

Current status: all tests pass. The closure-invoker hook (next chapter) enabled the last
remaining gaps (higher-order builtins, async) to reach parity.

## The `unsupported_on_wasm!` macro (guard 3.5)

For features that genuinely cannot work on wasm even with the interpreter:

```rust
macro_rules! unsupported_on_wasm {
    ($ctx:expr, $feature:expr) => {{
        ffi::set_error($ctx, format!("{}: not supported on wasm", $feature));
        Value::Unit
    }};
}
```

Usage (example, currently no cases):
```rust
IrOp::SomeNativeOnlyOp(..) => {
    unsupported_on_wasm!(ctx, "native-only feature");
}
```

This produces a clear error message instead of silent wrong output or a crash. It is the
"safe no" — acknowledging that the feature is not supported and telling the user clearly.

Currently, `unsupported_on_wasm!` is not used anywhere — all features that go through the
shared FFI work on both backends. The macro remains available for future use.

## The three guards working together

| Divergence type | Caught by |
|----------------|-----------|
| New `IrOp` added, interpreter not updated | Guard 1 (compile time) |
| New `oxy_*` added to JIT but not interpreter | Guard 2 (test time) |
| Feature works differently on two backends | Guard 3 (runtime) |
| Feature genuinely unsupported on wasm | Guard 3.5 (explicit marker) |

Together, these three guards enforce the invariant: the interpreter is always up-to-date
with the JIT. Divergence cannot be silent.
