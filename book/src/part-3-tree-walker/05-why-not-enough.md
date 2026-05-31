# It Works! So Why Isn't This Enough?

<!-- OPUS_FILL
Write a 3-paragraph section that is honest and a little bittersweet.

The tree-walker worked. It supported closures, traits, generics, async, HTTP — a full language.
The argument for stopping there is real: lots of useful languages are tree-walkers (Ruby before
YARV, Python before CPython's bytecode, most scripting languages).

But then: why did Oxy move on? The performance gap is real but for a general-purpose language
that targets the same use cases as Rust — systems programming, performance-critical code —
tree-walking is not acceptable. Also, the goal was always native code.

Frame the retirement as a graduation, not a failure. The tree-walker got the language designed.
It proved the semantics. It let features be added quickly without worrying about codegen.
Then it was retired having done its job.
-->

## The concrete performance problem

Consider a simple benchmark: compute the sum of integers from 0 to 10,000,000.

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

On the tree-walking interpreter, this takes several seconds. With Cranelift JIT compilation,
it takes milliseconds — roughly the same as the equivalent C program.

The difference is not mysterious. The tree-walker, for each iteration of the loop:
1. Matches `Stmt::While` → matches `Expr::BinaryOp` (the condition)
2. Matches `Expr::BinaryOp` again for the body assignment
3. Looks up `sum` and `i` in a `HashMap` (two hash lookups)
4. Allocates `Value::Int` structs for intermediate results
5. Checks mutability, updates the `HashMap`

The JIT-compiled version generates a native loop that is approximately:
```asm
loop_top:
    cmp  i, 10000000
    jge  done
    add  sum, i
    add  i, 1
    jmp  loop_top
done:
```

No hash lookups. No allocation. No dispatch. Just integer instructions.

## The allocation problem

Every intermediate value in the tree-walker is a `Value::Int(n)` — a heap-allocated Rust
enum. For `sum + i`, the tree-walker:
1. Evaluates `sum` → allocates `Value::Int(current_sum)`
2. Evaluates `i` → allocates `Value::Int(current_i)`
3. Applies `+` → allocates `Value::Int(result)`
4. Discards the intermediate `Value::Int`s

10,000,000 loop iterations × 3 allocations each = 30,000,000 heap allocations.
The garbage collector (Rust's drop machinery) processes all of them.

JIT compilation avoids this entirely: integer arithmetic operates on machine registers.
No heap allocation. The `Value` enum only appears at the FFI boundary — when values
cross from Oxy code into the Rust runtime (for printing, collection operations, etc.).

## What the tree-walker got right

Despite its retirement, the tree-walker contributed permanently to Oxy:

**It proved the semantics.** Features added in the tree-walking era — closures, generics,
traits, the `?` operator, async/await — were designed and debugged in a context where
execution was transparent and easy to trace. Adding a new expression type meant adding
one `match` arm. There was no codegen to debug simultaneously.

**It was fast to iterate on.** New syntax → new AST node → new `eval` arm → working
feature. The entire cycle took hours, not days. Oxy's feature set in March 2026 (phases
1-11, all in one day) was possible because the interpreter had zero codegen overhead.

**It validated the AST design.** The AST types that evolved during the tree-walking era
are essentially the same AST the current pipeline uses. The tree-walker's needs shaped
the AST, which was then inherited by the IR gen.

## The right time to retire it

The tree-walker was retired in May 2026, when Oxy committed to native compilation via
Cranelift. The decision was not "tree-walkers are bad." The decision was "this language's
goals include native performance, and we cannot achieve that by walking trees."

Many production languages use tree-walking for scripting contexts: Ruby's early VM,
Python before CPython's bytecode compiler, Perl. They made different trade-offs. Oxy's
target use case — a Rust-like language for systems programming — made the trade-off clear.

The next two parts trace what replaced it.
