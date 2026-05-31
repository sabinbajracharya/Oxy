# Bytecode: A Language Between Languages

Bytecode is a second language — one that sits in the gap between the source you write and the
machine code a CPU runs, and is designed to please neither humans nor hardware but the two programs
on either side of it. The compiler should find it easy to *generate*: emit a flat list of simple
instructions as you walk the AST. The VM should find it easy to *execute*: read the instructions
in order, jumping around for branches and loops. Nobody is meant to read bytecode for pleasure;
it's a private protocol between the compiler and the runtime, a handshake format that exists only
so those two can stop talking in trees and start talking in sequences.

You've almost certainly run on bytecode without thinking about it. Java compiles to JVM bytecode.
Those `.pyc` files Python leaves lying around are cached bytecode. WebAssembly — which Part 8 leans
on heavily — is, in a real sense, a bytecode too. And the idea is older than any of them: UCSD
Pascal was shipping "p-code" in 1978 on exactly this principle. So bytecode is well-trodden ground,
which is part of why Oxy tried it — and, as the end of this chapter explains, part of why Oxy
eventually walked away from it in favor of something better suited to becoming native code.

## What bytecode is

Bytecode is a serialized sequence of instructions, designed to be:
- **Easy to generate**: the compiler emits it from the AST in one pass
- **Easy to execute**: the VM reads it sequentially, with jump instructions for branches
- **Portable**: the same bytecode runs on any machine that has the VM

"Byte" in bytecode is historical — early VMs used single-byte opcodes to save memory.
Modern bytecodes (including Oxy's) use richer instruction types, but the name stuck.

A bytecode program is essentially: a list of functions, each containing a list of instructions.

## Compiling AST → bytecode

The compiler walks the AST and emits opcodes. For each expression node type, there is
a compilation rule. The rules are simpler than interpretation rules because they only
**emit** instructions, not execute them.

For `BinaryOp { left, op: Add, right }`:
```
compile(left)   → emits instructions that leave left's value on stack
compile(right)  → emits instructions that leave right's value on stack
emit(Add)       → VM will pop both, push sum
```

For `If { condition, then_block, else_block }`:
```
compile(condition)        → leaves bool on stack
emit(JumpIfFalse(else_target))
compile(then_block)
emit(Jump(end_target))
[patch else_target here]
compile(else_block)
[patch end_target here]
```

The "patch" step fills in the jump targets after both branches are compiled — the target
addresses are not known until you've seen how many instructions each branch emits.

For a `while` loop:
```
[loop_top here]
compile(condition)
emit(JumpIfFalse(loop_end))
compile(body)
emit(Jump(loop_top))
[loop_end here]
```

Every control flow construct compiles to a combination of conditional and unconditional jumps.
This flattens the tree structure into a linear instruction sequence — that's the key insight.

## The `Chunk` data structure

Oxy's bytecode VM used a `Chunk` to hold compiled functions:

```rust
// From the retired bytecode era
struct Chunk {
    code: Vec<OpCode>,            // instructions
    fn_table: HashMap<String, usize>,  // name → starting instruction index
    fn_frame_sizes: HashMap<String, usize>,  // name → number of locals
    constants: Vec<Value>,        // constant pool (strings, etc.)
}
```

`fn_table` maps function names to instruction indices: calling `add(2, 3)` emits
`Call { target: fn_table["add"], arg_count: 2 }`.

`fn_frame_sizes` tells the VM how large to make each function's local array when it is called.
The compiler determines this during compilation — it knows how many `let` bindings each function has.

## The compilation of closures

Closures were the hardest part of the bytecode compiler. A closure captures variables
from its enclosing scope, but those variables live in the enclosing function's locals.

The bytecode approach: when a closure is created, snapshot the referenced locals into a
`Value::Closure { captures: Vec<Value>, .. }`. When the closure is called, place the
captures at specific local slots (before the arguments).

```rust
// Closure creation: capture x from outer locals[0]
emit(LoadLocal(0))     // load x
emit(MakeClosure { code_offset, captures: [0] })  // create closure

// Closure call: captures[0] at locals[0], arg at locals[1]
```

This works but is subtle: if the captured variable is later mutated by the enclosing
function, the closure sees the old value (it got a copy, not a reference). This is
"capture by value" semantics, which is what Oxy's `move` closure does.

## Why bytecode is not the end of the road

Bytecode is faster than tree-walking: no recursive `eval` calls, no HashMap lookups for
locals, linear instruction stream. But it still has:

- **`Value` boxing**: every stack slot holds a `Value` — still heap-allocated for non-trivial types
- **Dispatch overhead**: the VM's main loop is `match opcode { ... }` — one dispatch per instruction
- **No machine code**: the VM is interpreted — not compiled to native x86 or arm64

The JIT goes further: it **compiles** the bytecode/IR to native machine code. No VM loop.
No dispatch. No boxing for scalar values. Direct CPU instructions.

Part 6 introduces the register IR — a better target than the stack for compilation to native code.
Part 7 shows how Cranelift turns that IR into machine code.
