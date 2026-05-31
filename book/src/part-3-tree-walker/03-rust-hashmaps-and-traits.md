# Rust Concepts: HashMap, Box<dyn Trait>, and Recursion

<!-- OPUS_FILL
Write a 1-paragraph intro. Frame it as: the tree-walker needs three Rust tools that come
up everywhere in the codebase: HashMap for environments, trait objects for extensibility,
and comfortable recursion. These are not exotic features — they are everyday Rust.
-->

## `HashMap<K, V>`: the environment's backbone

`HashMap<String, Value>` stores variable bindings. Unlike Rust's `BTreeMap`, `HashMap` is
unordered but O(1) average lookup — the right trade-off for variable environments where
key ordering does not matter.

```rust
use std::collections::HashMap;

let mut env: HashMap<String, Value> = HashMap::new();
env.insert("x".to_string(), Value::Int(42));

// Get: returns Option<&V>
if let Some(val) = env.get("x") {
    println!("{:?}", val);  // Value::Int(42)
}

// Entry API: insert-if-absent
env.entry("y".to_string()).or_insert(Value::Int(0));
```

`HashMap` is a generic type: `HashMap<K, V>` works with any key type that implements
`Hash + Eq`. Strings implement both, so `HashMap<String, V>` is the standard for name
tables.

The `HashMap` in `Environment` stores `(Value, bool)` tuples — the value plus a mutability
flag. Tuple types in Rust are `(T1, T2, T3, ...)` and accessed by index: `.0`, `.1`, `.2`.

## `Vec<T>` and iterators

`Vec<T>` is used everywhere: argument lists, statement sequences, struct fields, match arms.
The key operations:

```rust
let mut v: Vec<Value> = Vec::new();
v.push(Value::Int(1));
v.push(Value::Int(2));

// Iterator: produces each element in order
for val in &v {
    // val: &Value
}

// map: transform each element
let doubled: Vec<i64> = v.iter()
    .filter_map(|val| if let Value::Int(n) = val { Some(n * 2) } else { None })
    .collect();

// Collecting a Result<Vec<_>>: stops on first error
let results: Result<Vec<Value>, Error> =
    exprs.iter().map(|e| self.eval(e)).collect();
```

The last pattern — `exprs.iter().map(|e| self.eval(e)).collect()` — evaluates a list of
expressions and collects results, stopping at the first error. This is the standard Rust
idiom for evaluating argument lists in an interpreter.

## Traits and `impl Trait`

Traits define shared behavior. The `Value` type implements several:

```rust
trait Display {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result;
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{n}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::Bool(b) => write!(f, "{b}"),
            // ...
        }
    }
}
```

Implementing `Display` for `Value` lets you print it with `println!("{}", value)` and
format it with `format!("{}", value)` — the same as any built-in Rust type.

## `Box<dyn Trait>`: runtime polymorphism

When you need to store different types that share a trait, use `Box<dyn Trait>`:

```rust
trait Callable {
    fn call(&self, args: Vec<Value>) -> Result<Value, Error>;
}

struct BuiltinFn { ... }
struct UserFn { ... }

impl Callable for BuiltinFn { ... }
impl Callable for UserFn { ... }

// Store either in the same slot
let callable: Box<dyn Callable> = if user_defined {
    Box::new(UserFn { ... })
} else {
    Box::new(BuiltinFn { ... })
};

callable.call(args)?;
```

`dyn Callable` is a "trait object" — a fat pointer (data pointer + vtable pointer) that
dispatches method calls at runtime. The `Box` provides heap allocation for the concrete type.

In the tree-walker, this was used for the built-in function registry: all builtins implement
a common callable interface, stored as `Box<dyn Callable>`. The current Oxy uses a different
approach (table-driven dispatch in `stdlib/registry.rs`) but the trait object pattern appears
in other places in the codebase.

## Recursive functions in Rust

The tree-walker is inherently recursive — `eval` calls itself. Rust handles recursion on the
stack like any other language, but with one constraint: deeply nested programs can overflow
the stack. The tree-walker is susceptible to this for very deep expression trees.

The IR-based backends avoid this: the IR is a flat list of instructions in basic blocks.
There is no recursion in the main execution loop — `for op in block.ops { ... }` is a
flat iteration. Recursive calls in the *program* become `CallBuiltin` IR ops that call the
next function, which runs in a fresh frame.

## `Rc<RefCell<T>>`: shared mutable state

The environment uses `Rc<RefCell<T>>` to allow closures to share and mutate an environment:

- `Rc<T>` — reference-counted pointer. Multiple `Rc`s can point to the same data.
  When the last `Rc` is dropped, the data is freed. No garbage collector needed.
- `RefCell<T>` — enables "interior mutability": mutation through a shared (`&`) reference,
  checked at runtime. Panics if you try to borrow mutably while already borrowed.

```rust
let env = Rc::new(RefCell::new(Environment::new()));

// Clone the Rc (cheap — just increments reference count)
let closure_env = Rc::clone(&env);

// Borrow immutably
let val = env.borrow().get("x");

// Borrow mutably
env.borrow_mut().define("y".to_string(), Value::Int(5), false);
```

`Rc<RefCell<T>>` is the tree-walker's answer to "how do closures share mutable state?"
The JIT's answer is different: closures in the IR capture specific register values by
copying them into the closure object at creation time.
