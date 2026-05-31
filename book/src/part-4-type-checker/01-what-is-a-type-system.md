# What Is a Type System?

<!-- OPUS_FILL
Write a 3-4 paragraph hook.

Key insight: a type system is a static analysis that proves, at compile time, that certain
classes of errors cannot happen at runtime. It does not prove your program is correct —
it proves specific things are impossible.

Use a concrete example: passing a String to a function that expects an int. Without types,
this crashes at runtime (or silently computes nonsense). With types, the compiler catches it
before the program runs.

The "aha!" moment: a type checker is just another tree-walk. It visits every AST node and
asks "what type does this expression have?" and "does this type make sense in this context?"

End with the honest caveat: Oxy's type system is not Rust-complete. It catches common errors
but does not verify every possible type invariant. It is a pragmatic, useful subset.
-->

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
