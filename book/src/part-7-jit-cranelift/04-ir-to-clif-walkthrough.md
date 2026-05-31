# From IR Ops to CLIF: The Codegen Walkthrough

This is the chapter where it all pays off — where Oxy's register IR finally crosses over into real
machine code. Everything so far has been preparation: the IR was designed to lower cleanly, and
`codegen.rs` is where that lowering happens, op by op, translating each `IrOp` and `Terminator`
into Cranelift instructions that Cranelift then turns into x86 or arm64. It's a code-heavy chapter,
so open `crates/oxy-core/src/vm/jit/codegen.rs` and read along; the central thing to keep in mind
is the two-map register strategy, which is the first thing we'll unpack.

**File:** `crates/oxy-core/src/vm/jit/codegen.rs`

---

## The two-map register strategy

The codegen maintains two maps for Oxy registers:

```rust
let mut regs: HashMap<Reg, cranelift_codegen::ir::Value> = HashMap::new();
let mut reg_slot: HashMap<Reg, usize> = HashMap::new();
```

- **`regs`**: registers whose values are Cranelift SSA values — simple scalars (integers,
  booleans, floats) that live in CPU registers. Fast, no memory access.
- **`reg_slot`**: registers whose values went through the `JitContext` buffer — complex
  types (strings, structs, closures) that need full `Value` boxing. These live in local slots.

When codegen translates an `IrOp`, it places the result in one of the two maps depending
on whether the result is a simple scalar or a complex value.

## Compiling `ConstInt`

```rust
IrOp::ConstInt(r, n) => {
    let v = builder.ins().iconst(types::I64, *n);
    regs.insert(*r, v);  // simple scalar → stays in regs
}
```

`builder.ins().iconst(I64, n)` emits a Cranelift "integer constant" instruction and returns
a CLIF `Value` (Cranelift's SSA value type). The result is stored in `regs[r]`.

## Compiling `ConstBool`

```rust
IrOp::ConstBool(r, b) => {
    // Must go through FFI push/spill because Bool is a Value::Bool, not raw i64
    let b_val = builder.ins().iconst(types::I8, if *b { 1 } else { 0 });
    if let Some(push_bool) = ffi_refs.get("oxy_push_bool") {
        builder.ins().call(*push_bool, &[ctx, b_val]);
    }
    spill_result(&mut builder, ctx, &ffi_refs, *r, &mut reg_slot, &mut next_spill_slot);
}
```

`ConstBool` cannot live as a raw `i64` in `regs` — downstream code that reads this register
and passes it to Rust functions expects a `Value::Bool`, not a raw integer. So it is pushed
onto the operand stack as a proper `Value::Bool` via `oxy_push_bool`, then spilled into a
local slot. `reg_slot` maps `r` to that slot.

This was the root cause of **Cluster 1** in the war stories: `ConstBool` was incorrectly
leaving raw `i64(1)` in `regs`, which was later tagged as `Value::I64(1)` instead of `Value::Bool(true)`.

## Compiling `Add`

```rust
IrOp::Add(r, a, b) => {
    if let (Some(va), Some(vb)) = (regs.get(a), regs.get(b)) {
        // Both in regs: emit native add
        let result = builder.ins().iadd(*va, *vb);
        regs.insert(*r, result);
    } else {
        // At least one spilled: go through FFI
        push_reg(&mut builder, ctx, &ffi_refs, *a, &regs, &reg_slot);
        push_reg(&mut builder, ctx, &ffi_refs, *b, &regs, &reg_slot);
        if let Some(add) = ffi_refs.get("oxy_add") {
            builder.ins().call(*add, &[ctx]);
        }
        spill_result(&mut builder, ctx, &ffi_refs, *r, &mut reg_slot, &mut next_spill_slot);
    }
}
```

If both operands are in `regs` (fast path): emit a single Cranelift `iadd` instruction.
If either operand is spilled (came from an FFI call): fall back to calling `oxy_add`.

The fast path for integer arithmetic emits literally one CPU instruction. The slow path
calls a Rust function. Most arithmetic in typical programs stays on the fast path.

## Compiling `CallBuiltin`

```rust
IrOp::CallBuiltin { result, func, args, immediates, strings } => {
    // Push all Value arguments onto the operand stack
    for arg in args {
        push_reg(&mut builder, ctx, &ffi_refs, *arg, &regs, &reg_slot);
    }

    // Call the FFI function with ctx + immediates + string pointers
    let fref = ffi_refs[func];
    let mut call_args = vec![ctx];
    for imm in immediates {
        call_args.push(builder.ins().iconst(types::I64, *imm as i64));
    }
    for s in strings {
        // Pass string as (ptr, len) pair — two I64 args
        let (ptr, len) = string_to_ptr_len(&mut builder, s);
        call_args.push(ptr);
        call_args.push(len);
    }
    builder.ins().call(fref, &call_args);

    // Spill the result into a local slot
    spill_result(&mut builder, ctx, &ffi_refs, *result, &mut reg_slot, &mut next_spill_slot);
}
```

`CallBuiltin` is the codegen's most complex path. It pushes all Value arguments onto the
operand stack (so the FFI function can `pop` them), calls the Rust function with `ctx` +
immediate metadata + string pointers, then spills the result from the stack into a local slot.

## Compiling terminators

The `Branch` terminator:

```rust
Terminator::Branch { cond, then_block, else_block } => {
    // Get the boolean condition — might be in regs (CLIF i8) or reg_slot (Value::Bool)
    let c_bool = if let Some(clif_val) = regs.get(cond).copied() {
        builder.ins().icmp_imm(IntCC::NotEqual, clif_val, 0)
    } else if let Some(slot) = reg_slot.get(cond).copied() {
        // Load the boolean from the local slot as an i64, compare to 0
        let slot_val = builder.ins().iconst(types::I64, slot as i64);
        let inst = builder.ins().call(ffi_refs["oxy_read_local_i64"], &[ctx, slot_val]);
        let i64_val = builder.func.dfg.inst_results(inst)[0];
        builder.ins().icmp_imm(IntCC::NotEqual, i64_val, 0)
    };
    builder.ins().brif(c_bool, cl_blocks[then_block], &[ctx], cl_blocks[else_block], &[ctx]);
}
```

`brif` (branch-if) is Cranelift's conditional branch instruction. It takes the condition,
the then-block (with its block params), and the else-block. This is how Oxy IR's `Branch`
terminator becomes a native conditional jump.

The `ctx` parameter is threaded through every block jump — every block takes the context
pointer as its first parameter, so it is always available for FFI calls within the block.

## The full compilation flow

1. `gen_program()` → `Vec<IrFunction>` (IR gen, Part 6)
2. `Codegen::compile(functions)`:
   - For each `IrFunction`: `compile_fn` → Cranelift CLIF
   - `module.finalize_definitions()` → machine code emitted to mmap'd memory
   - `module.get_finalized_function(fid)` → function pointer
3. Function pointers stored in `JitTables.fn_table`
4. `invoke_jit_fn(fn_index, ctx)` → call through the pointer

After step 3, the `JitTables` has a pointer for every compiled function.
Calling `main` is: look up `fn_index` for `"main"`, call `fn_table[fn_index]`.
