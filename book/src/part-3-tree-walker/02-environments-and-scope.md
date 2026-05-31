# Environments and Scope

<!-- OPUS_FILL
Write a 2-paragraph intro.
The key insight: scope is just a stack of dictionaries. When you write `let x = 5` in a
function, `x` goes into the innermost dictionary. When you read `x`, you look in the
innermost dictionary, then the next one out, then the next, until you find it or hit the
top level.

This model — the "environment chain" — is one of the most elegant ideas in language
implementation. It handles closures, nested functions, modules, and shadowing all the
same way. Reference that Lisp discovered this in the 1960s and it has not been improved upon.
-->

## The environment chain model

Every running Oxy program has an **environment** — a mapping from variable names to values.
When code enters a new scope (a function call, an `if` body, a `for` loop), a new
child environment is created. When it exits, the child is discarded.

```
Global environment:   { println: [builtin] }
  │
  └── Function "main":  { x: 42, y: "hello" }
        │
        └── If-body:  { temp: 7 }
```

Variable lookup walks up the chain: look in the innermost environment first. If not found,
try the parent. If not found there, try the grandparent. If you reach the top with no
result, it is an undefined variable error.

This chain model handles shadowing automatically:

```rust
let x = 1;
{
    let x = 2;  // new binding in inner scope
    println(x); // prints 2 — found in inner scope first
}
println(x);     // prints 1 — inner scope gone, outer binding visible again
```

## Oxy's `Environment` type

The `env/mod.rs` module implements this — and it is still used today by the wasm IR
interpreter for runtime variable lookup:

```rust
// crates/oxy-core/src/env/mod.rs
pub struct Environment {
    values: HashMap<String, (Value, bool)>,  // name → (value, is_mutable)
    parent: Option<Env>,
}

pub type Env = Rc<RefCell<Environment>>;
```

Two fields: the current scope's bindings, and an optional parent scope.

`Rc<RefCell<...>>` is Rust's way of expressing "shared, mutable ownership." This is
needed because closures capture their defining environment — multiple closures might
share the same parent environment. `Rc` provides shared ownership; `RefCell` provides
interior mutability (allowing mutation through a shared reference).

## `define`, `get`, `set`

Three operations:

```rust
// Define a new variable in the current scope
pub fn define(&mut self, name: String, value: Value, mutable: bool) {
    self.values.insert(name, (value, mutable));
}

// Look up a variable — searches up the chain
pub fn get(&self, name: &str) -> Result<Value, PipelineError> {
    if let Some((value, _)) = self.values.get(name) {
        Ok(value.clone())
    } else if let Some(parent) = &self.parent {
        parent.borrow().get(name)   // recurse into parent
    } else {
        Err(/* undefined variable */)
    }
}

// Assign to an existing variable — searches up the chain
pub fn set(&mut self, name: &str, value: Value) -> Result<(), PipelineError> {
    if let Some((existing, mutable)) = self.values.get_mut(name) {
        if !*mutable {
            return Err(/* cannot assign to immutable variable */);
        }
        *existing = value;
        Ok(())
    } else if let Some(parent) = &self.parent {
        parent.borrow_mut().set(name, value)  // recurse into parent
    } else {
        Err(/* undefined variable */)
    }
}
```

`define` always creates in the current scope. `get` and `set` walk up the chain.
The distinction is important: `let x = 5` in an inner scope creates a new `x` in that
scope (shadowing any outer `x`), while `x = 10` (reassignment) modifies the existing
`x` wherever it lives in the chain.

## Closures and environment capture

Closures are the stress-test of the environment model. A closure captures its defining
environment — not a copy of it, but a reference to the actual environment:

```rust
fn make_counter() -> fn() -> int {
    let mut count = 0;
    || {
        count = count + 1;
        count
    }
}

fn main() {
    let counter = make_counter();
    println(counter()); // 1
    println(counter()); // 2
    println(counter()); // 3
}
```

When the closure `|| { count = count + 1; count }` is created, it captures the environment
that contains `count`. When `make_counter()` returns, `count` would normally be gone —
but `Rc` keeps it alive as long as the closure holds a reference.

Each call to `counter()` runs the closure body in an environment where `count` is visible,
modifies it via `set`, and returns the new value. The `Rc<RefCell<...>>` type is what
makes this work safely without garbage collection.

## How the JIT replaced this

The register IR and JIT compilation do not use the `Environment` type at all for compiled
functions. Instead:
- Local variables are **registers** in the IR (named `v0`, `v1`, etc.)
- The JIT assigns each register to a stack slot in a fixed-size local buffer
- Variable lookup is an array index — O(1) and zero allocation

The environment chain model is O(n) per lookup in the worst case (n = nesting depth),
and allocates a `HashMap` per scope entry. For interactive use (the wasm playground),
this is fine. For performance-critical code compiled to native, it would be a bottleneck.

The wasm IR interpreter still uses `Env` for dynamic variable storage in some edge cases,
but the main execution path for compiled functions uses the register model.
