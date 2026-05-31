# Rust Concepts: Box, Recursive Enums, and the Heap

We just built a tree, and trees are recursive: a node contains other nodes, which contain other
nodes, all the way down. In most languages you'd reach for this without a second thought. In Rust,
the moment you write it, the compiler stops you — because Rust insists on knowing the exact size in
bytes of every type at compile time, and a node that directly contains a node of the same type has,
in a literal sense, infinite size. This is the compiler being pedantic, but it's pedantic for a
good reason: values that live on the stack need a fixed, known size so the machine can lay out
function frames before anything runs. A type whose size depends on itself can't be laid out at all.

The escape hatch is `Box<T>`, and it's the thing that makes Oxy's entire AST possible — you'll see
it on nearly every recursive node. A `Box` is a pointer to a value on the heap, and a pointer is
always the same size no matter how big the thing it points to is. So by putting child nodes behind
a `Box`, the recursive size dependency is broken: the parent has a fixed size (it just holds
pointers), and the children live out on the heap where there's no size constraint to worry about.
Crucially, this costs you nothing in safety — `Box` still owns its value and still cleans it up
automatically. It's a fixed-size handle on a variable-size thing, which is exactly what a tree needs.

## Why Rust requires known sizes

In Rust, every value lives either on the **stack** (fixed-size, fast, automatically cleaned up)
or on the **heap** (variable-size, requires explicit allocation). The stack requires knowing
how large each frame will be before the function runs. That means every type stored on the
stack must have a **known size at compile time**.

For most types this is obvious:
- `i64` → 8 bytes
- `bool` → 1 byte
- `(i64, bool)` → 9 bytes
- A struct → sum of its fields' sizes

But what about a recursive type?

```rust
enum Expr {
    IntLiteral(i64),
    BinaryOp {
        left: Expr,   // ERROR: Expr contains Expr — infinite size
        right: Expr,
        op: BinOp,
    },
}
```

Rust rejects this. The size of `Expr` depends on the size of `Expr` — an infinite loop.
`BinaryOp` contains two `Expr` values, each of which might be a `BinaryOp` containing two
more `Expr` values, etc.

## `Box<T>`: a fixed-size pointer to a heap-allocated value

`Box<T>` is Rust's solution. It is a pointer — always exactly one pointer's worth of memory
(8 bytes on 64-bit systems) — that points to a `T` on the heap:

```rust
enum Expr {
    IntLiteral(i64),
    BinaryOp {
        left: Box<Expr>,   // OK: Box<Expr> is always 8 bytes (a pointer)
        right: Box<Expr>,
        op: BinOp,
    },
}
```

Now `Expr` has a known size:
- `IntLiteral` variant: 8 bytes (the `i64`)
- `BinaryOp` variant: 8 + 8 + size_of(BinOp) bytes (two pointers and an op)

Rust picks the largest variant's size. The actual `Expr` nodes that `left` and `right` point
to are allocated on the heap, where there is no size constraint.

## Creating and using `Box<T>`

To box a value: `Box::new(value)`. To get the value back: dereference with `*`.

```rust
let expr = Expr::BinaryOp {
    left: Box::new(Expr::IntLiteral(2)),
    right: Box::new(Expr::IntLiteral(3)),
    op: BinOp::Add,
};

// Pattern matching automatically dereferences Box
match expr {
    Expr::BinaryOp { left, right, op } => {
        // left and right are Box<Expr>
        // *left and *right are Expr
    }
    _ => {}
}
```

In Oxy's parser, `Box::new(...)` appears wherever an expression node contains child
expression nodes:

```rust
// crates/oxy-core/src/parser/expr.rs
Ok(Expr::BinaryOp {
    left: Box::new(left),
    right: Box::new(right),
    op,
    span,
})
```

## Ownership: what `Box<T>` means for cleanup

When a `Box<T>` is dropped (goes out of scope), Rust automatically frees the heap memory
it points to. This is Rust's ownership system in action: the `Box` owns the heap allocation,
and when the owner is gone, the memory is freed.

For AST nodes, this means: when the `Program` is dropped at the end of a pipeline stage,
the entire tree — all the `Box<Expr>` nodes throughout — is freed automatically, depth-first,
without any manual `free()` calls.

You do not need to understand ownership deeply to read the Oxy codebase. The key insight
is: `Box<T>` means "this thing lives on the heap and will be automatically cleaned up."

## `Vec<T>`: a growable heap-allocated list

`Vec<T>` is Rust's dynamic array. It also lives on the heap and also has a known stack size
(three words: a pointer, a length, and a capacity). It is used throughout the AST:

```rust
pub struct Program {
    pub items: Vec<Item>,   // a list of top-level items
}

pub struct FnDef {
    pub params: Vec<Param>,   // a list of parameters
}

pub enum Expr {
    Call {
        args: Vec<Expr>,      // a list of arguments (no Box needed — Vec itself is a pointer)
    }
}
```

`Vec<Expr>` does not need `Box` because `Vec<T>` is already a pointer — it stores its
elements on the heap. The `Vec` struct on the stack is always three words; the elements
can be as many as needed.

## `Option<T>`: a value that might not exist

`Option<T>` appears constantly in the AST for things that are optional:

```rust
pub struct FnDef {
    pub return_type: Option<TypeAnnotation>,  // None if -> is omitted
}

pub enum Stmt {
    Let {
        type_ann: Option<TypeAnnotation>,  // None if : Type is omitted
        value: Option<Expr>,              // None if = expr is omitted
    }
}
```

`Option<T>` is an enum with two variants:
```rust
enum Option<T> {
    Some(T),  // contains a value
    None,     // no value
}
```

It is how Rust expresses "this might not be there" without null pointers. The compiler
forces you to handle both `Some` and `None` — you cannot accidentally use a value that
is not there.

## The pattern in the AST

Looking at Oxy's `ast/mod.rs`, every recursive node follows the same pattern:

| Contains one child | Contains many children |
|-------------------|----------------------|
| `Box<Expr>` | `Vec<Expr>` |
| `Box<Pattern>` | `Vec<Stmt>` |
| `Option<Box<Expr>>` | `Vec<Param>` |

The rule: if it could be `None`, wrap in `Option`. If it's a single required child, wrap
in `Box`. If it's a list, use `Vec`. That's the entire vocabulary of Oxy's AST types.
