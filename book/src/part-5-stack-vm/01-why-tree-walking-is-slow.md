# Why Tree-Walking Is Slow

<!-- OPUS_FILL
Write a 2-paragraph hook. This is the "why we changed" chapter.
The key problem with tree-walking is indirection: every operation requires a recursive call,
a match on the AST variant, and heap-allocated Value objects. These are individually cheap
but compound catastrophically in loops.

Use the benchmark: the tight summation loop that takes seconds on the tree-walker.
Make the reader feel the ceiling: "Ferrite worked. And then we hit the wall."
-->

## The three costs that compound

The tree-walking interpreter has three sources of overhead that compound in tight loops:

**1. Function call overhead per AST node**

Every `eval(expr)` is a Rust function call. For `a + b`, that's:
- `eval(BinaryOp)` calls
- `eval(left)` → returns `Value::Int(a)` 
- `eval(right)` → returns `Value::Int(b)`
- match on the operator
- allocate `Value::Int(a + b)` and return

For a single `+` operation: 3 function calls, 1 allocation, multiple pattern matches.

**2. HashMap lookups for every variable access**

Every `eval(Ident("x"))` does a HashMap lookup. For a 10-iteration `while` loop accessing
two variables per iteration: 20 HashMap lookups, each requiring a hash computation.

**3. Heap-allocated `Value` objects for every intermediate result**

`Value::Int(42)` is a Rust enum on the heap (because `Value` contains `String` and `Vec`
variants that are heap-allocated, and all enum variants are the same size — the largest).
Every intermediate arithmetic result is allocated, then dropped immediately after use.

## The benchmark

```rust
fn main() {
    let mut sum = 0;
    let mut i = 0;
    while i < 10_000_000 {
        sum = sum + i;
        i = i + 1;
    }
    println(sum);
}
```

**Tree-walker execution for one loop iteration:**

1. `exec(While { condition, body })` — check condition
2. `eval(BinaryOp(i, <, 10_000_000))` — 2 sub-evals + alloc
3. `exec_block(body)` — 2 statements
4. `exec(Let/Assign { sum = sum + i })` — eval BinaryOp(sum, +, i) — 2 lookups + 1 alloc
5. `exec(Assign { i = i + 1 })` — eval BinaryOp(i, +, 1) — 1 lookup + 1 alloc
6. HashMap set for `sum`, HashMap set for `i`

Per iteration: ~8 recursive `eval` calls, 6 HashMap operations, 3+ heap allocations.
10,000,000 iterations: ~80M function calls, 60M HashMap operations, 30M+ heap allocations.

On modern hardware, this loop takes 5-10 seconds. The equivalent C loop: ~10 milliseconds.

## Why this is inherent to tree-walking

The costs above are not bugs — they are the direct consequence of the tree-walking approach:
- The AST is a tree → evaluation must recurse
- Variables live in a runtime dictionary → access requires hashing
- All values are tagged unions → arithmetic requires boxing

There is no way to eliminate these costs while still walking the AST. The fix requires
not walking the AST at runtime.

The insight that drives compilation: **do the tree traversal once at compile time, not once per execution.**

Compile the AST → a flat sequence of simple instructions. Execute those instructions.
The tree is gone. What remains is a sequence that a machine (virtual or real) can execute
fast.

This is bytecode compilation. The next chapters show how.
