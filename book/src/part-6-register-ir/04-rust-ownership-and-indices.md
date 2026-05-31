# Rust Concepts: Ownership, Vec, and Indices

<!-- OPUS_FILL
Write a 1-paragraph intro. The IR code uses index-based references
(BlockId = usize, Reg = usize) rather than pointers, because Rust's ownership rules
make self-referential data structures with pointers painful. Index-based references
are the Rust-idiomatic solution. Frame it as: "Rust's ownership system pushed us toward
a design that is actually cleaner."
-->

## Why indices instead of pointers

In C, a basic block graph might look like:
```c
struct BasicBlock {
    Instruction* ops;
    BasicBlock* successors[2];  // pointers to other blocks
};
```

In Rust, this is problematic. A `BasicBlock` containing a `Box<BasicBlock>` would own
its successors. But the successors might be referenced from multiple blocks (join points
are referenced by both branches of an if/else). Shared ownership in Rust requires `Rc`,
which adds complexity.

The Oxy solution: store all blocks in a `Vec<BasicBlock>` and use `BlockId = usize` as
the reference:

```rust
pub struct IrFunction {
    pub blocks: Vec<BasicBlock>,  // Vec owns all blocks
    pub entry: BlockId,           // just an index
}

pub struct BasicBlock {
    pub terminator: Terminator,
}

pub enum Terminator {
    Jump(BlockId),                         // index into blocks Vec
    Branch { cond: Reg, then_block: BlockId, else_block: BlockId },
}
```

`BlockId` is just `usize`. `blocks[block_id]` gives you the block. No pointers. No
ownership questions. The `Vec` owns all blocks; terminators just say "go to index N."

## Ownership basics: the borrow checker in 3 rules

Even though Oxy the language has no borrow checker, the Oxy compiler is written in Rust
and must satisfy Rust's ownership rules. The three core rules:

1. **Each value has exactly one owner.** When the owner goes out of scope, the value is dropped (freed).
2. **You can have multiple shared references (`&T`) OR one exclusive reference (`&mut T`), but not both.**
3. **References cannot outlive the value they point to.**

For the IR gen, these rules mainly matter in two places:

**Passing IR into the JIT:**
```rust
// gen_program returns owned data
let functions: Vec<IrFunction> = ir_gen.gen_program(&program)?;

// Pass to JIT — `functions` is moved into the JIT
let result = jit.compile_and_run(functions)?;
// After this, `functions` is gone — JIT owns it
```

**Iterating while building:**
```rust
// Correct: collect results into a Vec, then process
let arg_regs: Vec<Reg> = args.iter()
    .map(|arg| self.gen_expr(arg))
    .collect::<Result<Vec<_>, _>>()?;

// Then use arg_regs in the CallBuiltin op
```

## `Vec` and index stability

`Vec` is stable — once you have an index into a `Vec`, the index remains valid as long
as the `Vec` is not reallocated. But `Vec` can reallocate when it grows, invalidating
any pointers into it (pointers, not indices — indices are just numbers).

This is why the IR uses indices everywhere. A `Reg = usize` and `BlockId = usize` remain
valid across `Vec` growth. A hypothetical `*const IrOp` pointer would be invalidated
the moment the `ops: Vec<IrOp>` reallocated.

This is the index-based architecture pattern: store everything in `Vec`s, reference with
indices. It is the standard Rust solution for graph-like data structures.

## `usize` and platform-specific sizes

`BlockId = usize` and `Reg = usize` are `usize` — platform-pointer-sized integers (8 bytes
on 64-bit, 4 bytes on 32-bit). Oxy's IR gen never worries about this: virtual registers
start at 0 and are allocated one at a time, so a `usize` is always sufficient.

The register counter in `IrGen`:
```rust
struct IrGen {
    next_reg: usize,  // monotonically incrementing
    // ...
}

fn alloc_reg(&mut self) -> Reg {
    let r = self.next_reg;
    self.next_reg += 1;
    r
}
```

Each call to `alloc_reg()` returns a fresh, unique register index. The JIT maps these
to Cranelift SSA values; the interpreter uses them as indices into a registers array.

## `drain` and `collect` patterns

IR gen builds argument lists frequently:

```rust
// Evaluate all arguments, collecting results
let arg_regs: Vec<Reg> = args.iter()
    .map(|arg| self.gen_expr(arg))
    .collect::<Result<Vec<Reg>, _>>()?;

// Then use arg_regs in a CallBuiltin
self.emit_op(IrOp::CallBuiltin {
    result: self.alloc_reg(),
    func: "oxy_call",
    args: arg_regs,
    strings: vec![fn_name.clone()],
    immediates: vec![],
});
```

The `collect::<Result<Vec<_>, _>>()` idiom evaluates all expressions and either produces
a `Vec` of all results (if all succeed) or the first error (if any fail). It short-circuits
on the first `Err` — exactly the semantics needed for argument evaluation.
