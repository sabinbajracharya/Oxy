# Exercise: Trace a Program on the Stack

The stack VM is gone, but the mental model it gave you — flatten the tree into a linear sequence of
simple instructions, then execute them in a loop — is exactly the lens you need for the register IR
coming up in Part 6. So these exercises use the stack model as a thinking tool rather than asking
you to edit retired code. You'll hand-compile a program to stack bytecode on paper, trace it by
hand, and then dump the *real* register IR for a comparable program and see precisely where the two
models agree and where they part ways. That contrast is the whole point: feel the stack model
clearly enough that, when registers replace it next part, you can see exactly what changed and why
it's a better fit for native code.

## Part A: Hand-compile to stack bytecode

Given this Oxy program:

```rust
fn factorial(n: int) -> int {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}
```

Write out the stack bytecode instructions that a compiler would emit for this function.
Use the `OpCode` variants from the chapter:
- `ConstInt(n)`, `LoadLocal(i)`, `StoreLocal(i)`
- `Mul`, `Sub`, `Le`
- `JumpIfFalse(target)`, `Jump(target)`, `Return`
- `Call { target, arg_count }`

Assume:
- `n` is local slot 0 (the parameter)
- The instruction array starts at index 0
- Fill in jump targets after you know how many instructions each block emits

**Hint:** The `if` condition emits `Le` (less-or-equal) + `JumpIfFalse`. The then-branch
returns 1. The else-branch computes `n * factorial(n-1)`.

---

## Part B: Trace execution of `factorial(3)`

Using the bytecode you wrote in Part A, manually trace the execution stack for `factorial(3)`.
Show the stack contents and frame state at each key instruction.

Expected output: `6` (3 * 2 * 1).

Your trace should show the recursive calls — three frames on the call stack at the deepest
point, then unwinding.

---

## Part C: Compare stack to register IR

Now look at the actual register IR for a simpler program. Run:

```bash
OXY_VM_TRACE=1 docker compose run --rm dev bash -c \
  "echo 'fn main() { let x = 2 + 3; println(x); }' > /tmp/t.ox && cargo run --bin oxy -- run /tmp/t.ox" 2>&1
```

Find the IR for `main`. Compare the IR instructions to what the stack bytecode would look like.

Answer:
1. In the register IR, where does `2` go? Where does `3` go? Where does `2 + 3` go?
2. In the stack bytecode, where does `2` go? Where does `3` go? Where does `2 + 3` go?
3. The register IR has no `StoreLocal` / `LoadLocal` pattern for simple `let` bindings.
   Why not? What does the register IR do instead?

---

## Reflection: the stack VM as a stepping stone

The stack VM existed for 15 days and then was replaced. Was it wasted effort?

Consider:
1. The bytecode compiler proved that AST → flat instruction sequence was feasible
2. The `Value` type, stdlib, and test suite were all built during the bytecode era
3. The bytecode-to-Cranelift attempt (one day) showed that bridging was harder than replacing

Write 2-3 sentences on: what did the stack VM contribute that lived on in the current codebase?
