# Exercise: Trace a Program Through the IR

<!-- OPUS_FILL
Write a 1-paragraph framing. The exercise bridges theory and practice — you run real
programs and read their IR. The goal is: after this exercise, IR dumps are not scary.
They are just a slightly unusual but fully readable form of "what your program does."
-->

## Part A: Read the IR for a recursive function

Write this Oxy program and save it as `/tmp/fib.ox`:

```rust
fn fib(n: int) -> int {
    if n <= 1 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

fn main() {
    println(fib(10));
}
```

Run with trace:
```bash
OXY_VM_TRACE=1 docker compose run --rm dev bash -c \
  "cargo run --bin oxy -- run /tmp/fib.ox" 2>&1
```

Find the IR for `fib`. Answer:
1. How many basic blocks does `fib` have? Draw the CFG (just boxes with arrows).
2. How is the recursive call `fib(n - 1)` represented? What `CallBuiltin` args does it use?
3. How is `fib(n-1) + fib(n-2)` implemented? Which registers hold the two intermediate results?

---

## Part B: Add an IR snapshot test

IR snapshot tests compare IR output against a golden file. Look at an existing test in
`crates/oxy-core/tests/ir_snapshot_tests.rs`.

Add a new snapshot test for this program:
```rust
fn double(x: int) -> int {
    x * 2
}
```

Steps:
1. Add a test case in `ir_snapshot_tests.rs` following the existing pattern
2. Run with `UPDATE_SNAPSHOTS=1` to generate the golden file:
   ```bash
   UPDATE_SNAPSHOTS=1 docker compose run --rm dev bash -c \
     "cargo test -p oxy-core ir_snapshot"
   ```
3. Inspect the generated golden file in `tests/snapshots/ir/`. Verify it looks correct.
4. Run without `UPDATE_SNAPSHOTS` to confirm the test passes:
   ```bash
   docker compose run --rm dev bash -c "cargo test -p oxy-core ir_snapshot"
   ```

---

## Part C: Add a new IR op (guided)

Add a `Square(result, operand)` IR op that squares a value. This is for learning purposes —
`x * x` accomplishes the same thing, but the exercise teaches you the mechanics of adding
an op.

**Steps:**

1. In `ir.rs`, add `Square(Reg, Reg)` to the `IrOp` enum.

2. The build will break everywhere `IrOp` is exhaustively matched. Fix each one:
   - In `interp.rs`: add `IrOp::Square(r, a) => { registers[*r] = square(registers[*a]) }`
   - In `codegen.rs`: add a Cranelift instruction for it (hint: `ins.imul(a, a)`)
   - In `ir_snapshot.rs`: add a pretty-print case

3. In `ir_gen/`, add a place to emit `Square`. For example, if you want
   `x²` syntax, you'd add a parse rule. For now, just emit `Square` instead
   of `Mul(r, a, a)` for the case `BinaryOp { op: Mul, left: x, right: x }` where
   both sides are the same variable.

4. Write a `.ox` test and verify the feature works.

**Note:** The compiler will tell you every place you missed. Follow its guidance. This is
the exhaustive-match property of the IR — you cannot add an op without implementing it
in all backends.

---

## Part D: Understanding `local_count`

Look at the IR trace for any function. Count the `StoreLocal` and `LoadLocal` instructions.

In `ir_gen/mod.rs`, find the `local_count` field. Answer:
1. How is `local_count` determined? Is it set per-parameter or per-binding?
2. What is the relationship between `local_count` in the `IrGen` and `local_count` in `IrFunction`?
3. Why is it important that `local_count` is accurate? What breaks if it underestimates
   the number of slots needed? (Hint: read the comment in `jit/mod.rs` or codegen about buffer sizing.)
