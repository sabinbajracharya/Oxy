# What Is a Type System?

Here's a precise way to think about what a type checker does, and it's worth getting precise
because the word "types" gets used loosely. A type system is a static analysis — it runs before
your program does — that *proves* certain classes of error cannot happen at runtime. Note the
careful wording. It does not prove your program is correct; that's far too much to ask, and no
type checker promises it. It proves that *specific things are impossible*. Your logic can still be
wrong, your algorithm can still be backwards — but you will not, at runtime, find yourself trying
to add a number to a function or read a field off something that has no fields. Those particular
disasters are ruled out in advance.

Make it concrete. Suppose `add` expects two integers, and somewhere you call `add("hello", 42)`.
In a language with no type checking, that call sails through to runtime, where the machine
attempts to add a string to an integer and either crashes, panics, or — worse — silently computes
garbage that propagates for another thousand lines before anything looks wrong. A type checker
catches it before a single instruction executes: *expected int, found String, line 4*. The error
moves from "mysterious runtime failure, time and place unknown" to "named mistake, exact location,
before you even ran the thing." That relocation is the entire value proposition.

And here's the satisfying part, the thing that makes a type checker far less intimidating than its
reputation: it's just another tree walk. You already watched the tree-walker visit every AST node
and ask *what value does this produce?* The type checker visits the very same nodes and asks a
parallel question — *what type does this produce, and does that type make sense where it's being
used?* Same tree, same recursion, same shape of code. The only thing that changes is that instead
of computing a `Value`, each node computes a `TypeInfo`. If you understood Part 3, you already
understand the skeleton of Part 4.

One honest caveat before we dive in: Oxy's type system is not Rust's. It is not complete, and it
is not trying to be. It catches the common, costly mistakes — wrong argument types, missing
fields, misused `?`, visibility violations — but it deliberately doesn't attempt to verify every
invariant a fancier system could. It's a pragmatic, useful subset, chosen to be implementable and
to give good error messages, not to be a theorem prover. Knowing where its edges are is part of
understanding it.

## Types as a static proof

A type system is a machine-readable contract: "this variable holds integers," "this function
returns a String," "this collection contains booleans." The type checker verifies these
contracts before the program runs.

When the contract is violated:
```rust
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    println(add("hello", 42));  // type error: expected int, found String
}
```

Without a type checker, this reaches `add` at runtime, attempts to add a string and an
integer, and either crashes or produces nonsense (depending on the language). With a type
checker, it fails before execution: "expected int, found String at line 4."

This is the core value: **catch a class of errors at compile time rather than at runtime**.

## What Oxy's type checker verifies

- Function arguments match declared parameter types
- Assignment values match declared variable types
- Field access is performed on structs that have that field
- The `?` operator is only used in functions that return `Result` or `Option`
- Visibility: private struct fields cannot be accessed outside their defining module
- `break` and `continue` are only used inside loops
- Return type: the function body's type matches the declared return type
- Unused type integers (`i32`, `u64`) are rejected with fix-it suggestions

## What it does not verify

Oxy's type checker is intentionally pragmatic. It does not verify:
- Ownership or lifetimes (Oxy has neither)
- Array bounds (checked at runtime)
- Integer overflow (Oxy wraps on overflow by design)
- Exhaustive match coverage for all cases (currently: warning, not error)

The type checker is not a theorem prover. It is a "catch obvious errors early" tool.

## The key insight: type checking is a tree walk

A type checker is structurally very similar to a tree-walking interpreter. Both visit
every AST node. The difference is what they compute:

| Tree-walker | Type checker |
|------------|-------------|
| `eval(expr)` → `Value` | `infer(expr)` → `TypeInfo` |
| Looks up variable values | Looks up variable types |
| Executes at runtime | Runs at compile time |

For a `BinaryOp` node:
```
# Tree-walker
eval(BinaryOp { left: 2, op: +, right: 3 }) = 5

# Type checker
infer(BinaryOp { left: IntLit(2), op: +, right: IntLit(3) }) = int
```

Both walk the same tree. Both handle the same node types. The difference is the return
value — a `Value` vs a `TypeInfo`.
