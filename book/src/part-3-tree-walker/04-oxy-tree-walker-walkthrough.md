# Oxy's Original Interpreter: A Walkthrough

<!-- OPUS_FILL
Write a 1-2 paragraph intro. Frame it as archaeology — we're looking at retired code.
Reference commit 3849173 as the birth of the tree-walker, and note it ran Oxy from
March to May 2026. The code is gone from main but lives in git history.
Make it feel like reading old letters: valuable, instructive, and a little bittersweet.
-->

## Finding the retired code

The tree-walking interpreter was removed in May 2026 when the register IR + JIT replaced it.
To read the actual implementation:

```bash
# See Phase 4 — the birth of the interpreter
git show 3849173

# See the full tree-walker before it was retired
git log --oneline | grep -i "bytecode\|remove\|retire\|interpreter" | head -10
git show <the-removal-commit>:crates/oxy-core/src/vm/interp.rs
```

What follows is a reconstruction based on the architecture that existed — the patterns
were consistent across the entire tree-walking era.

## The interpreter structure

The tree-walker was an `Interpreter` struct with an environment and a function table:

```rust
struct Interpreter {
    env: Env,                                    // current scope chain
    functions: HashMap<String, FnDef>,           // user-defined functions
    builtins: HashMap<String, Box<dyn Callable>>, // built-in functions
}
```

The main method was `eval(&mut self, expr: &Expr) -> Result<Value, RuntimeError>`.
Statements were handled by `exec(&mut self, stmt: &Stmt) -> Result<ControlFlow, RuntimeError>`,
where `ControlFlow` was an enum capturing normal execution, early return, break, and continue.

## Evaluating expressions

For a `BinaryOp`:

```rust
Expr::BinaryOp { left, op, right, .. } => {
    let l = self.eval(left)?;
    let r = self.eval(right)?;
    match op {
        BinOp::Add => match (l, r) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Str(a), Value::Str(b)) => Ok(Value::Str(a + &b)),
            _ => Err(RuntimeError::TypeMismatch { op: "+", .. })
        },
        BinOp::Eq => Ok(Value::Bool(l == r)),
        // ...
    }
}
```

For a function call:

```rust
Expr::Call { callee, args, .. } => {
    // Evaluate all arguments first
    let arg_vals: Vec<Value> = args.iter()
        .map(|a| self.eval(a))
        .collect::<Result<_, _>>()?;

    // Evaluate the callee to get the function value
    let fn_val = self.eval(callee)?;

    match fn_val {
        Value::Closure { params, body, env } => {
            // Create a new scope with the captured environment as parent
            let call_env = Environment::child(&env);
            for (param, arg) in params.iter().zip(arg_vals) {
                call_env.borrow_mut().define(param.clone(), arg, false);
            }
            let saved_env = std::mem::replace(&mut self.env, call_env);
            let result = self.eval(&body);
            self.env = saved_env;
            result
        }
        Value::Builtin(name) => {
            self.builtins[&name].call(arg_vals)
        }
        _ => Err(RuntimeError::NotCallable)
    }
}
```

## Executing statements

For `Stmt::Let`:

```rust
Stmt::Let { name, mutable, value, .. } => {
    let val = if let Some(expr) = value {
        self.eval(expr)?
    } else {
        Value::Unit
    };
    self.env.borrow_mut().define(name.clone(), val, *mutable);
    Ok(ControlFlow::Normal)
}
```

For `Stmt::Return`:

```rust
Stmt::Return { value, .. } => {
    let val = match value {
        Some(expr) => self.eval(expr)?,
        None => Value::Unit,
    };
    Ok(ControlFlow::Return(val))  // propagates up through exec_block
}
```

`ControlFlow` was the mechanism for early exit. Each call to `exec` returns it, and
callers check: if it is `Return`, stop executing statements and propagate the value up.
This is how `return` in the middle of a function worked — a result that "unwinds" the
call to `exec_block`.

## Closures: capturing the environment

When a closure was created:

```rust
Expr::Closure { params, body, .. } => {
    // Capture the current environment by cloning the Rc (cheap — just a pointer copy)
    let captured_env = Rc::clone(&self.env);
    Ok(Value::Closure {
        params: params.iter().map(|p| p.name.clone()).collect(),
        body: *body.clone(),
        env: captured_env,
    })
}
```

The `Rc::clone` is not a deep copy — it is just an increment of the reference count.
The closure holds a reference to the same environment chain the current function is
executing in. When the closure is later called, it creates a new child of *that* environment,
restoring the captured scope.

This is why closures in the tree-walker correctly "close over" their surrounding variables —
they hold a pointer to the scope that existed when they were created.

## The pattern in the current codebase

The tree-walker's architecture is gone, but its ideas live on:

| Tree-walker | Current Oxy |
|-------------|-------------|
| `eval(expr)` → `Value` | IR gen emits `IrOp`s; FFI executes them |
| `HashMap<String, Value>` environments | Registers in a flat buffer |
| `ControlFlow::Return(v)` enum | `Terminator::Ret` in the IR |
| `Rc<RefCell<Environment>>` for closures | Closure object capturing register values |
| Operator dispatch in `eval` | `oxy_add`, `oxy_eq`, etc. in `jit/ffi.rs` |

The concepts are the same. The mechanism is faster.
