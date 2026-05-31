# The Solution: Interpret the Same IR

Here's the realization that makes the whole problem dissolve: most of it was already solved. Look
at what we built for the JIT and ask what actually depends on Cranelift. The IR? No — that's just
data, a `Vec<IrFunction>`, a list of blocks holding instructions. The runtime semantics? No — those
live in the shared `oxy_*` FFI functions, which are plain Rust that compiles anywhere, wasm
included. The value representation, the `JitContext` buffer? Both pure Rust, both already shared.
The *only* thing that genuinely needed Cranelift was the one step of turning IR into machine code.
And if you don't have machine code, you don't have to *compile* the IR — you can just *walk* it.

That's the entire solution, and it's why the interpreter is so small. It is not a second compiler
and it is not a second language implementation; it's a second *executor* for the exact same IR. The
JIT turns `IrOp::Add` into a native `add` instruction; the interpreter sees `IrOp::Add` and calls
`oxy_add` — the very same FFI function, just reached by a normal Rust call instead of compiled code.
Walk the blocks, dispatch each op, follow the terminators. The hard parts were already there,
shared, waiting. What was left to write was the walking loop, and it's about three hundred lines.

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
