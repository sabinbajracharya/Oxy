# Basic Blocks and Control Flow Graphs

<!-- OPUS_FILL
Write a 2-paragraph hook.
The key insight: a basic block is a sequence of instructions with no branches in the middle
— control enters at the top, executes straight through, exits at the bottom.
An if/else creates three blocks: the condition block, the then block, and the else block.
A while loop creates three blocks: the condition block, the body block, and the continuation.

The CFG is the graph connecting these blocks. It's the structure that lets compilers
analyze control flow, detect dead code, and optimize loops.

Make it concrete with a simple if/else diagram.
-->

## What is a basic block?

A **basic block** is a maximal sequence of instructions that:
1. Has exactly one entry point (no jumps into the middle)
2. Has exactly one exit point — a **terminator** that either jumps to another block or returns

Every program can be decomposed into basic blocks. The connections between blocks form a
**control flow graph (CFG)** — a directed graph where nodes are blocks and edges are possible
control transfers.

For an `if/else`:

```
            ┌──────────────┐
            │ Block 0      │
            │ v0 = x > 0   │
            │ Branch(v0)   │
            └──────────────┘
              /            \
      (true) /              \ (false)
    ┌────────────┐    ┌────────────┐
    │ Block 1    │    │ Block 2    │
    │ v1 = x * 2 │    │ v1 = 0    │
    │ Jump(3)    │    │ Jump(3)   │
    └────────────┘    └────────────┘
              \              /
               \            /
            ┌──────────────┐
            │ Block 3      │
            │ (continuation)│
            └──────────────┘
```

Block 0 computes the condition and branches. Blocks 1 and 2 are the then/else arms. Block 3 is
the join point — both arms jump here after they finish.

## Oxy's `BasicBlock` type

```rust
// crates/oxy-core/src/vm/jit/ir.rs
pub(crate) struct BasicBlock {
    pub id: BlockId,           // index into the function's blocks array
    pub ops: Vec<IrOp>,        // straight-line register operations
    pub terminator: Terminator, // how control leaves this block
    pub predecessors: Vec<BlockId>, // incoming edges (reserved for future use)
}
```

`ops` are the straight-line operations — constants, arithmetic, loads, stores.
`terminator` is always last — it determines what happens next.

## Oxy's `Terminator` type

```rust
pub(crate) enum Terminator {
    Return(Reg),          // return the value in this register
    Jump(BlockId),        // unconditional jump to another block
    Branch {              // conditional branch
        cond: Reg,
        then_block: BlockId,
        else_block: BlockId,
    },
    Halt,                 // end program
}
```

Only four variants. The entire control flow structure of any Oxy program reduces to
these four terminators:
- Linear code → `Jump` from one block to the next
- `if`/`else` → `Branch` splitting into two blocks
- Loop back-edges → `Jump` to an earlier block
- Function end → `Return`

## The `IrFunction` wraps a CFG

```rust
pub(crate) struct IrFunction {
    pub name: String,
    pub blocks: Vec<BasicBlock>,  // all blocks in this function
    pub entry: BlockId,           // which block to start at (usually 0)
    pub local_count: usize,       // number of local variable slots
    pub return_type: TypeInfo,
    pub params: Vec<(String, TypeInfo)>,
    pub captures: Vec<(String, usize)>,
    pub is_async: bool,
    // ...
}
```

`blocks` is a flat `Vec` of `BasicBlock`s. The CFG is implicit in the `Jump`/`Branch`
terminators — they reference other blocks by their `BlockId` index into the `Vec`.

`entry` is the starting block — always block 0 for non-special functions.

`local_count` is the number of local variable slots needed. Unlike registers (infinite,
virtual), locals correspond to actual memory slots. The JIT allocates `local_count`
slots in its local buffer per function invocation.

## How a `while` loop becomes blocks

For `while cond { body }`:

```
Block 0 (entry):
  ... (setup code) ...
  Jump(1)             ← jump to condition block

Block 1 (condition):
  v_cond = [evaluate condition]
  Branch(v_cond, then=2, else=3)

Block 2 (body):
  [execute body]
  Jump(1)             ← back-edge! loops to condition

Block 3 (after loop):
  [code after the while]
```

The back-edge `Jump(1)` in block 2 creates the loop. The CFG has a cycle. Both the JIT
and the interpreter handle this: the JIT's Cranelift turns the back-edge into a native
jump instruction; the interpreter just resets its block pointer to block 1.

## Why basic blocks matter for execution

Both backends (JIT and wasm interpreter) execute basic blocks the same way:

1. Enter a block at its first op
2. Execute ops in order (each writes to a register)
3. Execute the terminator:
   - `Return(r)` → done, result is `registers[r]`
   - `Jump(b)` → continue from block `b`
   - `Branch { cond, then, else }` → check `registers[cond]`, jump to `then` or `else`

The wasm interpreter's main loop is literally:
```rust
'outer: loop {
    for op in &block.ops {
        // execute op
    }
    match &block.terminator {
        Terminator::Jump(b) => { block = &function.blocks[*b]; }
        Terminator::Branch { cond, then_block, else_block } => {
            let go_to = if registers[*cond].is_truthy() { then_block } else { else_block };
            block = &function.blocks[*go_to];
        }
        Terminator::Return(r) => return Ok(registers[*r].clone()),
        Terminator::Halt => break 'outer,
    }
}
```

A simple loop over operations, then a branch on the terminator. This is the entire execution model.
