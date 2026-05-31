# The Divergence Guards: How You Stop Backends from Drifting

<!-- OPUS_FILL
Write a 2-paragraph intro.
The guards are the system's immune response to divergence. Without them, backends drift
silently: the JIT gets a feature, the interpreter misses it, and you only find out when
someone runs the browser playground.

Frame it as: three layers of defense, each catching a different class of divergence.
These are not bureaucratic tests — they are the mechanisms that make "one feature,
two backends" actually work in practice.
-->

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
