# The Solution: Interpret the Same IR

<!-- OPUS_FILL
Write a 2-paragraph hook.
The solution is genuinely elegant. You already have an IR. The IR is just data.
You can walk it. The runtime (FFI functions) is already shared.
The only thing you need to write is the walking loop — and that's about 300 lines.

Frame it as: "What if the problem was mostly already solved? What if the hard part
(the FFI, the value representation, the IR gen) was already there, shared?"
The interpreter is not a second compiler — it is a second executor for the same IR.
-->

## The interpreter in 30 lines

The heart of the wasm IR interpreter is a loop:

```rust
// crates/oxy-core/src/vm/interp.rs (simplified)
fn interpret(&self, ctx: &mut JitContext, func: &IrFunction) -> Disc {
    let mut regs: HashMap<Reg, Value> = HashMap::new();
    let mut block_id = func.entry;

    loop {
        let block = &func.blocks[block_id];

        // Execute each operation in the block
        for op in &block.ops {
            self.exec_op(ctx, op, &mut regs, ...);
            if has_real_error(ctx) { return discriminant(ctx); }
        }

        // Follow the terminator
        match &block.terminator {
            Terminator::Return(r) => {
                ctx.result = regs[r].clone();
                return discriminant(ctx);
            }
            Terminator::Jump(target) => { block_id = *target; }
            Terminator::Branch { cond, then_block, else_block } => {
                let truthy = self.truthy(ctx, regs.get(cond));
                block_id = if truthy { *then_block } else { *else_block };
            }
            Terminator::Halt => return discriminant(ctx),
        }
    }
}
```

That's it. A `loop`, a `match` on the terminator, and one `exec_op` call per operation.
No compilation. No register allocation. No machine code. Just: walk the blocks, execute
each operation, follow the terminators.

## `exec_op`: one op at a time

```rust
fn exec_op(&self, ctx: &mut JitContext, op: &IrOp, regs: &mut HashMap<Reg, Value>, ...) {
    match op {
        IrOp::ConstInt(r, n) => {
            regs.insert(*r, Value::I64(*n));  // no FFI needed for constants
        }
        IrOp::ConstBool(r, b) => {
            regs.insert(*r, Value::Bool(*b)); // direct creation of the right type
        }
        IrOp::Add(r, a, b) => {
            self.binary(ctx, regs, "oxy_add", *r, *a, *b); // same FFI as JIT
        }
        IrOp::LoadLocal(r, slot) => {
            let v = self.call_collect(ctx, "oxy_load_local", &[], &[], &[*slot]);
            regs.insert(*r, v);
        }
        IrOp::CallBuiltin { result, func, args, immediates, strings } => {
            // Push args onto the operand stack
            for arg in args {
                unsafe { ffi::push(ctx, regs[arg].clone()); }
            }
            // Call the FFI function
            self.call_named(ctx, func, &[], immediates, strings_as_bytes(strings));
            // Pop the result
            let v = unsafe { ffi::pop(ctx) };
            regs.insert(*result, v);
        }
        // ... exhaustive match, no wildcard arm
    }
}
```

The key property: for `IrOp::Add`, the interpreter calls `oxy_add` — the same FFI function
the JIT calls via native machine code. The interpreter is not a reimplementation of
arithmetic. It is a caller of the shared FFI.

For constants (`ConstInt`, `ConstBool`), the interpreter short-circuits the FFI and creates
`Value` instances directly — this is fine because the value type is known at interpretation
time (it's encoded in the IR op variant).

## What the interpreter does not need

The interpreter does not need:
- **A compiler**: no Cranelift, no CLIF construction, no register allocation
- **Executable memory**: it walks data structures; no memory mapping
- **Platform-specific code**: pure Rust, compiles everywhere including wasm32

The interpreter needs:
- **The IR**: `Vec<IrFunction>` — the same output of `ir_gen` that the JIT uses
- **The FFI table**: a `HashMap<&str, *const u8>` from `ffi_symbols()`
- **The `JitContext` buffer**: the same context the FFI functions operate on
- **The `Value` type**: the same runtime representation

All four are shared infrastructure. The interpreter adds: a walking loop and a dispatch
on `IrOp` variants.

## Why this is the right design

An alternative: build a separate interpreter for Oxy source code (tree-walker), run
on wasm. This would duplicate the runtime semantics. Every feature would need to be
implemented twice: once in the JIT path, once in the wasm interpreter. Bugs would
diverge independently.

The chosen design: one runtime (the FFI), two executors (JIT compiler, IR walker).
Features are implemented once in `ffi/mod.rs`. Both backends call those implementations.
Semantic divergence is structurally impossible — both backends call the same code.
