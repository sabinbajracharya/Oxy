# The Simplest Way to Run Code

This is the moment the whole project stops being a parlor trick. Up to now we've had a lexer that
chops text into tokens and a parser that arranges those tokens into a tree — both genuinely useful,
both completely inert. Nothing *runs*. You can feed Oxy a program and it will tell you the program
is well-formed, and then it will do absolutely nothing with it. This chapter changes that. By the
end of it, `2 + 2` produces `4`, and the first time you watch that happen with code you wrote
yourself, it is a little bit magical. You built a thing that computes.

And the way it computes is almost insultingly simple. You already have a tree. To run the program,
you walk the tree: look at a node, recursively evaluate its children, combine the results, return a
value. An addition node evaluates its two sides and adds them. An `if` node evaluates its condition
and picks a branch. A variable node looks itself up. There's no compilation step, no intermediate
format, no machine code — the interpreter's own recursion *is* the program's execution. It's the
most direct path from "I have a tree" to "the tree did something," and it's where almost every
language starts.

It's also where Oxy started, back when it was still called Ferrite. From Phase 4 through Phase 11,
*everything* ran this way: not just arithmetic, but closures, structs, traits, generics, modules,
async/await, even HTTP and JSON. A complete, genuinely usable language, implemented as nothing more
than a big recursive `eval` function. That is a real achievement, and it's worth sitting with how
much you can accomplish with so little machinery.

But — and this is the honest part — the tree-walker is retired now. We're not going to pretend
otherwise or teach it as the destination. Reading this chapter is like studying the original
foundation of a house that's since been renovated: it was load-bearing once, it held up real weight,
and you can't understand why the later additions are shaped the way they are without seeing what
came first. So we walk through it properly, and then, at the end, we'll look at the exact wall it
ran into — the wall that made everything in Parts 5 through 8 necessary.

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
