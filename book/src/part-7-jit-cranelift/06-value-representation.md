# Value Representation: How Oxy Stores Everything

<!-- OPUS_FILL
Write a 2-paragraph intro.
Every runtime value in Oxy — an integer, a string, a struct, a closure — is represented
by one enum: `Value`. This is the lingua franca between compiled code and the Rust runtime.

The cost: every `Value` carries a tag (which variant it is) and may carry heap-allocated data.
The benefit: a single type that passes through every layer of the system unchanged.
-->

## The `Value` enum

**File:** `crates/oxy-core/src/types/mod.rs`

```rust
pub enum Value {
    I64(i64),                    // int
    U8(u8),                      // byte
    F64(f64),                    // float
    Bool(bool),
    String(String),              // heap-allocated UTF-8
    Char(char),
    Unit,
    Function(Box<FunctionData>), // closure or function pointer
    Range(i64, i64),
    Vec(Rc<RefCell<Vec<Value>>>),           // shared mutable
    Array(Vec<Value>),                       // value type
    Tuple(Vec<Value>),
    Struct { name: String, fields: HashMap<String, Value> },
    EnumVariant { enum_name: String, variant: String, data: Vec<Value> },
    HashMap(Rc<RefCell<HashMap<Value, Value>>>),
    HashSet(Rc<RefCell<HashSet<Value>>>),
    BTreeMap(Rc<RefCell<BTreeMap<Value, Value>>>),
    BTreeSet(Rc<RefCell<BTreeSet<Value>>>),
    BinaryHeap(Rc<RefCell<BinaryHeap<Value>>>),
    VecDeque(Rc<RefCell<VecDeque<Value>>>),
    Iterator(Rc<RefCell<IteratorState>>),
    Future(Box<FutureData>),
    JoinHandle(Box<JoinHandleData>),
    Cell(Rc<RefCell<Value>>),              // captured mutable variable
    FnPointer(String),                     // reference to a named function
    // ... a few more
}
```

28+ variants. Everything Oxy can represent at runtime is one of these.

## Collections use `Rc<RefCell<T>>`

Collections (`Vec`, `HashMap`, `HashSet`, etc.) use `Rc<RefCell<>>` for shared mutation:

```rust
Vec(Rc<RefCell<Vec<Value>>>)
```

This means: cloning a `Value::Vec` does not copy the vector's elements — it copies the
`Rc` pointer (cheap, just increments the reference count). Both the original and the clone
point to the same heap-allocated data.

This gives Python-like collection semantics:
```rust
fn main() {
    let a = vec![1, 2, 3];
    let b = a;          // b and a share the same backing Vec
    b.push(4);
    println(a.len());   // prints 4, because a and b share storage
}
```

`Rc` provides shared ownership; `RefCell` provides interior mutability (mutation through
a shared reference, checked at runtime). Together: shared mutable collections.

## Primitives are value types

`I64(i64)`, `U8(u8)`, `F64(f64)`, `Bool(bool)`, `Char(char)`, `Unit` — these are all
copied on clone:

```rust
let x = 42;
let y = x;    // y is a fresh I64(42); x unchanged
```

`String` is also a value type (clone copies the string data), matching Rust's `String` semantics.

`Struct` is a value type — cloning a struct copies all its fields (recursively). This
differs from Python where objects are always by-reference. Oxy's struct clone semantics
match Rust's `#[derive(Clone)]`.

## `FunctionData`: the closure representation

```rust
pub struct FunctionData {
    pub params: Vec<String>,
    pub body: FunctionBody,
    pub env: Option<Env>,       // captured environment (if closure)
    pub name: Option<String>,   // function name (if named)
    pub captures: Vec<(String, Value)>, // captured variable copies
}

pub enum FunctionBody {
    Compiled { fn_index: usize, local_count: usize }, // JIT-compiled
    Builtin(String),                                    // stdlib function
    Interpreted(Expr),                                  // wasm interpreter
}
```

A `FunctionData` is either:
- **Compiled** — a `fn_index` into the JIT's function table. Calling it invokes the
  native compiled code.
- **Builtin** — a stdlib function name. Calling it dispatches to `stdlib/registry.rs`.
- **Interpreted** — an expression tree (for closures in the wasm interpreter).

The `captures` field holds the values captured from the enclosing scope at the moment
the closure was created. On each invocation, captures are loaded into the function's
local slots before execution starts.

## `Value::Cell`: mutable captured variables

When a `let mut` binding is captured by a closure, it becomes a `Value::Cell`:

```rust
Cell(Rc<RefCell<Value>>)
```

The cell is a `Rc<RefCell<>>` wrapping a single `Value`. The outer scope and the closure
both hold `Rc` pointers to the same cell. Mutations to the variable go through the cell,
so both see the same current value:

```rust
fn make_counter() -> fn() -> int {
    let mut count = 0;   // becomes Value::Cell(Rc<RefCell<Value::I64(0)>>)
    || {
        count = count + 1;   // reads and writes through the shared Cell
        count
    }
}
```

The `MakeCell` IR op and `LoadLocal`/`LoadLocalRaw` distinction handles this transparently:
`LoadLocal` unwraps the cell's current value; `LoadLocalRaw` returns the cell itself (for
passing to a closure that will capture it).

## The memory cost

Each `Value` is at minimum 40 bytes on a 64-bit system (the enum discriminant + the largest
variant). Complex variants (`Struct`, `HashMap`) may add heap allocations on top.

For performance-critical inner loops, the JIT avoids boxing where possible: the `regs` map
in codegen keeps simple values (`I64`, `Bool`) as raw Cranelift SSA values, only boxing
them into `Value` when crossing an FFI boundary. This is the "two-map" strategy from the
codegen walkthrough.
