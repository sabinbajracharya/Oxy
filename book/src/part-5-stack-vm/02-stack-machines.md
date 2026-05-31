# Stack Machines: The Mental Model

If you ever used one of those old Hewlett-Packard calculators — the ones where you typed `4`, then
`3`, then `+`, in that order, and `7` appeared — you have already used a stack machine, and you
already understand the model we're about to build. It's called Reverse Polish Notation, and the
mechanism behind it is the simplest virtual machine there is: there's one stack, you push operands
onto it, and an operator pops the operands it needs, computes, and pushes the result back. `4 3 +`
means *push 4, push 3, now `+` pops both and pushes their sum*. That's the entire idea. There is no
tree to walk and nothing to recurse into — just a flat stream of "push this, do that."

This is not a toy model, either. HP shipped calculators on it in the 1970s, the Java Virtual
Machine runs on it, CPython's bytecode interpreter runs on it, and for about fifteen days in May
2026, so did Oxy. So when the realization lands — *wait, this is just a reverse-Polish calculator?*
— lean into it, because that is genuinely most of what a stack VM is. The rest of this chapter is
the handful of additions that turn the calculator into something that can run real programs: a
place to put variables, a way to call functions, and an instruction set to encode it all.

## The stack machine model

A stack machine maintains one data structure: a stack. Instructions operate on the stack:

- **Push instructions** put values onto the stack
- **Pop instructions** take values off the stack and compute results

For `2 + 3 * 4`:

The compiler emits:
```
ConstInt(2)    # stack: [2]
ConstInt(3)    # stack: [2, 3]
ConstInt(4)    # stack: [2, 3, 4]
Mul            # pops 3 and 4, pushes 12 → stack: [2, 12]
Add            # pops 2 and 12, pushes 14 → stack: [14]
```

After execution, the result (14) is on top of the stack. No recursion. No AST traversal.
Just: push, push, push, operate.

## Variables: locals alongside the stack

Variables need storage that outlasts a single expression. Stack machines use **locals** —
a separate indexed array (not the operand stack):

```
StoreLocal(0)   # pop stack top → locals[0]   (let x = ...)
LoadLocal(0)    # locals[0] → push onto stack  (read x)
```

For `let x = 5; let y = x + 3`:

```
ConstInt(5)    # stack: [5]
StoreLocal(0)  # locals[0] = 5; stack: []

LoadLocal(0)   # stack: [5]
ConstInt(3)    # stack: [5, 3]
Add            # stack: [8]
StoreLocal(1)  # locals[1] = 8; stack: []
```

The stack shrinks back to empty between statements — this is the operand stack discipline.

## Function calls: frames

When a function is called, a new **frame** is created:

```
struct Frame {
    locals: Vec<Value>,          // this function's locals
    return_ip: usize,            // where to return to
    caller_op_stack_len: usize,  // to clean up on return
}
```

Arguments are popped from the caller's operand stack and placed in the new frame's locals.
On return: pop the result, restore the caller's frame, push the result onto the caller's stack.

This is the same call stack model CPUs use for native function calls — except it is
implemented in software, in a `Vec<Frame>`.

## Oxy's stack VM `OpCode` enum

The retired Oxy bytecode VM used this instruction set:

```rust
// From the retired vm/mod.rs (commit fa87d96^)
pub enum OpCode {
    // Constants
    ConstInt(i64, IntegerWidth),
    ConstFloat(f64, FloatWidth),
    ConstBool(bool),
    ConstString(String),
    ConstUnit,

    // Variables
    LoadLocal(usize),   // push locals[index] onto stack
    StoreLocal(usize),  // pop stack → store in locals[index]

    // Binary operations (pop two, push result)
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Gt, Le, Ge,
    And, Or,

    // Unary
    Neg, Not,

    // Control flow
    Jump(usize),          // unconditional jump to instruction index
    JumpIfFalse(usize),   // pop; jump if falsy
    JumpIfTrue(usize),    // pop; jump if truthy

    // Functions
    Call { target: usize, arg_count: usize },
    Return,

    // Built-ins
    CallBuiltin(String, usize),  // name, arg_count
    // ...many more
}
```

A `Chunk` was an array of `OpCode`s plus metadata (local counts, constants). The compiler
walked the AST and emitted opcodes into a `Chunk`. The VM executed the `Chunk`.

## Why this is faster than tree-walking

For the 10,000,000 iteration loop:

**Stack VM execution per iteration:**
1. `LoadLocal(1)` — array index, O(1)
2. `ConstInt(10_000_000)` — no allocation (small literal optimization possible)
3. `Lt` — pop two, compare, push bool
4. `JumpIfFalse(end)` — pop, check, set instruction pointer
5. `LoadLocal(0)` + `LoadLocal(1)` + `Add` + `StoreLocal(0)` — integer arithmetic
6. `LoadLocal(1)` + `ConstInt(1)` + `Add` + `StoreLocal(1)` — increment

Per iteration: ~10 array-index operations, no HashMap lookups, no recursive calls.
The `Value::Int` boxing still happens (the stack contains `Value` objects), but allocation
rates are dramatically lower.

Result: 10x-50x faster than tree-walking for compute-intensive code.
