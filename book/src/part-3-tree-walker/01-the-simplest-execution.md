# The Simplest Way to Run Code

<!-- OPUS_FILL
Write a 3-4 paragraph hook. This is the chapter where we first make the language *run*.

The key emotional beat: this is the moment. You have a parser. You have an AST. Now you
can actually execute code. For the first time, typing `2 + 2` and getting `4` feels magical.

But also set up the honesty: this approach is retired. We used it, it worked, and then we
moved on. Reading this chapter is like looking at the foundation of a house that has been
renovated — it was load-bearing once. Understanding it makes the later chapters make sense.

Reference the Ferrite era: this was how the language ran from Phase 4 through Phase 11 —
closures, modules, async, HTTP — all implemented as tree-walking evaluation. That's impressive.
That's also why it eventually had to go.
-->

## The idea: walk the tree, evaluate each node

A tree-walking interpreter does exactly what the name says. Given an AST node, it:

1. Looks at the node type
2. Recursively evaluates child nodes
3. Computes and returns a result

No compilation. No bytecode. No IR. Just: match the node, do the thing.

For a `BinaryOp` node:
```
evaluate(BinaryOp { left, op, right }) =
    let l = evaluate(left)
    let r = evaluate(right)
    apply(op, l, r)
```

For an `If` node:
```
evaluate(If { condition, then_block, else_block }) =
    if evaluate(condition) is truthy:
        evaluate(then_block)
    else:
        evaluate(else_block)
```

For an `Ident` node (variable reference):
```
evaluate(Ident(name)) =
    look up name in current environment
    return the value
```

That's it. Each node type has an evaluation rule. The entire interpreter is one big
recursive function that dispatches on node type and applies the rule.

## What it actually looked like

The Oxy tree-walker (now retired — `git show 3849173` to see Phase 4) was structured
as a single `eval` method on an interpreter struct:

```rust
// Simplified reconstruction of the retired interpreter
fn eval(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
    match expr {
        Expr::IntLiteral(n, _, _) => Ok(Value::Int(*n)),
        Expr::StringLiteral(s, _) => Ok(Value::Str(s.clone())),
        Expr::BoolLiteral(b, _) => Ok(Value::Bool(*b)),

        Expr::Ident(name, _) => {
            self.env.get(name)
                .ok_or_else(|| RuntimeError::UndefinedVariable(name.clone()))
        }

        Expr::BinaryOp { left, op, right, .. } => {
            let l = self.eval(left)?;
            let r = self.eval(right)?;
            self.apply_binop(op, l, r)
        }

        Expr::If { condition, then_block, else_block, .. } => {
            let cond = self.eval(condition)?;
            if cond.is_truthy() {
                self.eval_block(then_block)
            } else if let Some(else_branch) = else_block {
                self.eval(else_branch)
            } else {
                Ok(Value::Unit)
            }
        }

        Expr::Call { callee, args, .. } => {
            let fn_val = self.eval(callee)?;
            let arg_vals: Result<Vec<_>, _> = args.iter().map(|a| self.eval(a)).collect();
            self.call_function(fn_val, arg_vals?)
        }

        // ... 30+ more arms
    }
}
```

The recursion is the execution. `eval(BinaryOp)` calls `eval(left)` and `eval(right)`,
which may each recursively call `eval` on their subtrees. The call stack of the Oxy
interpreter *is* the execution stack of the Oxy program.

## The `Value` type

At runtime, the interpreter works with `Value`s — a Rust enum representing every possible
Oxy runtime value:

```rust
enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Unit,           // ()
    Vec(Vec<Value>),
    HashMap(HashMap<Value, Value>),
    Struct { name: String, fields: HashMap<String, Value> },
    Enum { variant: String, data: Box<Value> },
    Closure { params: Vec<String>, body: Expr, env: Env },
    // ...
}
```

This `Value` type is still used in the current Oxy — both the JIT and the wasm interpreter
use it as the representation for every runtime value. It evolved significantly but the
core idea (one enum for all runtime types) remains unchanged.

## What the tree-walker could do

By the time the tree-walking interpreter was retired (May 2026), it supported:
- Integers, floats, strings, booleans, unit
- Variables and scoping
- Functions and recursion
- Closures and higher-order functions
- Structs and enums with pattern matching
- Traits and operator overloading
- Generics (via runtime monomorphization)
- Modules and use statements
- `Option<T>`, `Result<T, E>`, the `?` operator
- Async/await and `spawn`
- HTTP requests, JSON, file I/O, standard library

That is a complete, usable language — all implemented as recursive AST evaluation.

## The speed wall

The problem is evaluation cost. For every expression execution, the interpreter:
1. Matches on the `Expr` variant (a Rust `match` — cheap)
2. Recursively calls `eval` for each child (function call overhead — less cheap)
3. Works with heap-allocated `Value` objects (allocation — expensive)
4. Looks up variables in a `HashMap` environment (hashing — non-trivial)

For a tight loop like:
```rust
let mut sum = 0;
for i in 0..1_000_000 {
    sum = sum + i;
}
```

The tree-walker must: match the `For` node, match the `Range`, evaluate the loop body
1,000,000 times, each time matching `BinaryOp`, calling `eval` twice, looking up `sum`
and `i` in the environment, allocating the result `Value::Int`.

Modern JITs do this loop in nanoseconds. Tree-walkers take milliseconds.

The next part shows the first step away from this: compiling to bytecode for a stack VM.
Part 6 shows the step after that: register IR. Part 7 shows where we landed: native code
via Cranelift.
