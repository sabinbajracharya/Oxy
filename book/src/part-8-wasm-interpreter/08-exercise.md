# Exercise: Add a New Op to Both Backends

<!-- OPUS_FILL
1-paragraph framing. Adding a feature that touches both backends is the acid test of
understanding Part 8. This exercise walks you through it, guided by the compiler.
"After this exercise, you have added a feature that works correctly on both the JIT and
the wasm interpreter. That is the full pipeline."
-->

## Part A: Add `IrOp::Abs` to both backends

Add a new IR op `Abs(result, operand)` that computes the absolute value of an integer.

**Step 1: Add to `ir.rs`**

```rust
// In the IrOp enum:
/// Absolute value: result = |operand|
Abs(Reg, Reg),
```

**Step 2: Watch the build fail**

```bash
docker compose run --rm dev bash -c "cargo build -p oxy-core 2>&1 | head -30"
```

The compiler will list every exhaustive match that now needs an `Abs` arm. This is Guard 1 in action.

**Step 3: Handle in `interp.rs`**

Find the `exec_op` match in `interp.rs`. Add:
```rust
IrOp::Abs(r, a) => {
    self.unary(ctx, regs, "oxy_abs", *r, *a);
}
```

**Step 4: Add `oxy_abs` to `ffi/mod.rs`**

```rust
#[no_mangle]
extern "C" fn oxy_abs(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { ffi::pop(ctx) };
    let result = match val {
        Value::I64(n) => Value::I64(n.abs()),
        Value::F64(f) => Value::F64(f.abs()),
        other => other,
    };
    unsafe { ffi::push(ctx, result); }
}
```

**Step 5: Add to `ffi_symbols()` and `ffi_decls()`**

Follow the existing pattern. The FFI consistency test will guide you.

**Step 6: Handle in `codegen.rs`**

Add to `compile_op`:
```rust
IrOp::Abs(r, a) => {
    push_reg(&mut builder, ctx, &ffi_refs, *a, &regs, &reg_slot);
    if let Some(abs) = ffi_refs.get("oxy_abs") {
        builder.ins().call(*abs, &[ctx]);
    }
    spill_result(&mut builder, ctx, &ffi_refs, *r, &mut reg_slot, &mut next_spill_slot);
}
```

**Step 7: Add to `ir_snapshot.rs`**

The pretty-printer has a match over `IrOp`. Add:
```rust
IrOp::Abs(r, a) => format!("v{r} = Abs(v{a})"),
```

**Step 8: Wire up in `ir_gen`**

In `ir_gen`, add support for `math::abs(x)` to emit the new op. Or, add an `Expr::Unary` case
that produces `IrOp::Abs` for a new `abs` prefix operator.

**Step 9: Write a test**

```rust
// examples/features/numbers/abs.ox
#[test]
fn test_abs_negative() {
    assert_eq(math::abs(-5), 5);
}

#[test]
fn test_abs_positive() {
    assert_eq(math::abs(5), 5);
}
```

**Step 10: Run the parity test**

```bash
docker compose run --rm dev bash -c "cargo test -p oxy-core --test jit_interp_parity"
```

Both backends should agree on the output.

---

## Part B: Understand the divergence guard in practice

After completing Part A, deliberately break one of the guards:

1. **Break Guard 1:** Remove the `Abs` arm from `interp.rs`. Run `cargo build`. What error do you get?

2. **Break Guard 2:** Remove `oxy_abs` from `ffi_symbols()` but keep it in `ffi_decls()`.
   Run `cargo test -p oxy-core ffi_consistency`. What error do you get?

3. **Break Guard 3:** Make `oxy_abs` return the wrong value on the interpreter path (e.g., always return 0).
   Run the parity test. What output do you get?

Restore each break before moving to the next. The goal: experience each guard catching its specific class of divergence.

---

## Part C: Understanding `unsupported_on_wasm!`

Find the `unsupported_on_wasm!` macro in `interp.rs`. Currently it is defined but unused.

Think of a feature that could genuinely require `unsupported_on_wasm!`. What property
would make a feature impossible to implement in the interpreter?

Hint: consider features that require access to system resources (signals, shared memory,
hardware timers) that a wasm sandbox does not expose.

Write a comment in `interp.rs` (or in your notes) describing what that feature would be
and why the macro would be the right tool.
