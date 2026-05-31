# It Works! So Why Isn't This Enough?

Here's the uncomfortable thing: the tree-walker *worked*. Not in a toy, proof-of-concept way — it
ran a full language. Closures, traits, generics, pattern matching, modules, async/await, HTTP,
JSON, a standard library. If you'd stopped there and shipped it, you'd have had a perfectly real
programming language, and you'd be in excellent company. Ruby walked trees before YARV. Python ran
that way before CPython grew a bytecode compiler. A huge fraction of the scripting languages people
use every day are, under the hood, doing exactly what we just described. "It's only a tree-walker"
is not an insult; plenty of beloved languages never became anything else.

So why did Oxy move on? Because of what Oxy is trying to be. The performance gap between walking a
tree and running native code is not a rounding error — it's two or three orders of magnitude on a
tight loop, as the benchmark below shows. For a scripting language gluing together fast native
libraries, that gap is invisible and irrelevant. But Oxy's whole identity is *Rust without the
borrow checker* — it's aimed at the same territory as Rust, where someone might reasonably write a
compute-heavy inner loop and expect it not to crawl. A language that looks like a systems language
but runs hundreds of times slower than one is making a promise its surface can't keep. And anyway,
native code was always the destination; the tree-walker was the scaffolding, not the building.

So think of the retirement as a graduation, not a failure. The tree-walker did the job no other
stage could have done as cheaply: it let us *design* the language. Every feature got prototyped,
debugged, and proven correct in an environment where execution was a transparent recursive function
you could read top to bottom — no codegen to fight, no IR to inspect, just `match` and recurse. By
the time we were ready to compile to native code, the semantics were already settled and battle-
tested, which meant the JIT only had to be *fast*, not also *correct from scratch*. The tree-walker
earned its retirement. It's just that its job is done, and the rest of this book is about what came
next.

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
