# Oxy's AST: Every Node Explained

<!-- OPUS_FILL
Write a 1-paragraph intro. Something like: "The AST is the shared language between the
parser and everything downstream. Every chapter from here on operates on these types.
Let's learn them."
Keep it short — this chapter is reference material, not narrative.
-->

**File:** `crates/oxy-core/src/ast/mod.rs`

Open it now. This chapter walks through every major type and explains why it is shaped the way it is.

---

## The hierarchy

```
Program
└── Vec<Item>
    ├── FnDef (function)
    │   ├── Vec<Param>
    │   └── Block
    │       └── Vec<Stmt>
    │           └── Expr (recursive tree)
    ├── StructDef
    ├── EnumDef
    ├── ImplBlock
    ├── TraitDef
    ├── ImplTraitBlock
    ├── ModuleDef
    ├── UseDef
    ├── TypeAlias
    └── Const
```

---

## `Program`

```rust
pub struct Program {
    pub items: Vec<Item>,
    pub span: Span,
}
```

The root. A program is a flat list of top-level items. There is no nesting at this level —
even nested modules are items that contain other items.

---

## `Item`

```rust
pub enum Item {
    Function(FnDef),
    Struct(StructDef),
    Enum(EnumDef),
    Impl(ImplBlock),
    Trait(TraitDef),
    ImplTrait(ImplTraitBlock),
    Module(ModuleDef),
    Use(UseDef),
    TypeAlias { name, target, span },
    Const { name, type_ann, value, is_static, span },
}
```

Everything that can appear at the top level of a file is an `Item`. Each variant wraps
a dedicated struct that carries that item's specific fields.

`Item::span()` is a method that extracts the span from whichever variant this is — useful
when error messages need to point at an item without knowing which kind it is.

---

## `FnDef`

```rust
pub struct FnDef {
    pub name: String,
    pub is_async: bool,
    pub generic_params: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub span: Span,
}
```

A function definition. Notable fields:

- `is_async`: `true` for `async fn`. Handled in IR gen by emitting the async prologue.
- `generic_params`: `<T: Display>` becomes `[GenericParam { name: "T", bounds: ["Display"] }]`.
- `return_type`: `None` means `-> ()` (unit return). The type checker treats missing
  return type as `()`.
- `attributes`: `#[test]`, `#[compile_error]`, `#[derive(...)]` — stored as `Attribute` values,
  processed by the test runner and IR gen.
- `visibility`: `pub`, `pub(crate)`, `pub(super)`, or private.

---

## `Param`

```rust
pub struct Param {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub is_mut: bool,
    pub span: Span,
}
```

`is_mut` corresponds to `mut param: T` in the source. In Oxy there are no reference types —
`self` and `mut self` are both just `self` at the type level. The `is_mut` flag on `Param`
only controls whether the binding can be reassigned within the function body.

---

## `Block`

```rust
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}
```

A brace-enclosed list of statements. The last statement without a semicolon is the block's
value (tail expression). The IR gen handles this by emitting the tail expression as the
return value.

---

## `Stmt`

```rust
pub enum Stmt {
    Let { name, mutable, type_ann, value, span },
    Return { value, span },
    While { label, condition: Box<Expr>, body: Block, span },
    Loop { label, body: Block, span },
    For { label, name, iterable: Box<Expr>, body: Block, span },
    Break { label, value: Option<Box<Expr>>, span },
    Continue { label, span },
    WhileLet { label, pattern: Box<Pattern>, expr: Box<Expr>, body: Block, span },
    ForDestructure { label, names: Vec<String>, iterable: Box<Expr>, body: Block, span },
    LetPattern { pattern: Box<Pattern>, mutable, value: Expr, span },
    Use(UseDef),
    Item(Box<Item>),
    Expr { expr: Expr, has_semicolon: bool },
}
```

The `has_semicolon` field on `Stmt::Expr` distinguishes:
- `foo();` → `has_semicolon: true` → result is discarded
- `foo()` → `has_semicolon: false` → result is the block's return value (tail expression)

Labels (`'outer: while ...`) are `Option<String>` — present when a label was written,
`None` otherwise. `break 'outer` refers to the label by name.

`Stmt::Item` allows functions, structs, and enums to be defined inside a function body.
These are hoisted by the parser to synthetic top-level names (`outer__inner`) and a local
alias is added.

---

## `Expr` — the big one

`Expr` has 30+ variants. The full list is in `ast/mod.rs:462`. Here are the most important:

| Variant | Example | Notes |
|---------|---------|-------|
| `IntLiteral(i64, suffix, span)` | `42` | Suffix is always `None` in current Oxy |
| `FloatLiteral(f64, suffix, span)` | `3.14` | Same |
| `BoolLiteral(bool, span)` | `true` | |
| `StringLiteral(String, span)` | `"hello"` | |
| `Ident(String, span)` | `x` | Variable reference |
| `BinaryOp { left, op, right, span }` | `a + b` | Both sides are `Box<Expr>` |
| `UnaryOp { op, expr, span }` | `-x`, `!b` | |
| `Call { callee, args, .. }` | `foo(a, b)` | `callee` is `Box<Expr>` |
| `MethodCall { object, method, args, .. }` | `v.push(x)` | `object` is `Box<Expr>` |
| `FieldAccess { object, field, .. }` | `p.x` | |
| `Index { object, index, .. }` | `a[i]` | |
| `If { condition, then_block, else_block, .. }` | `if x { } else { }` | else is `Option<Box<Expr>>` |
| `Match { expr, arms, .. }` | `match x { }` | |
| `Closure { params, body, .. }` | `\|x\| x + 1` | `body` is `Box<Expr>` |
| `Try { expr, .. }` | `foo()?` | The `?` operator |
| `As { expr, type_name, .. }` | `x as float` | Cast |
| `StructInit { name, fields, base, .. }` | `Point { x: 1, y: 2 }` | |
| `PathCall { path, args, .. }` | `Vec::new()` | Qualified call |
| `Path { segments, .. }` | `Option::None` | Qualified path, no call |
| `FString { parts, .. }` | `f"hello {name}"` | Interpolated string |
| `Await { expr, .. }` | `x.await` | Async |
| `Spawn { expr, .. }` | `spawn(task)` | Async task |

---

## `TypeAnnotation`

```rust
pub enum TypeAnnotation {
    Named {
        name: String,
        generic_args: Vec<TypeAnnotation>,
        span: Span,
    },
    Array {
        inner: Box<TypeAnnotation>,
        size: usize,
        span: Span,
    },
}
```

Type annotations appear in `let x: int = ...`, `fn foo(x: Vec<String>) -> int`, etc.

`Named` covers everything: `int`, `String`, `Vec<int>`, `HashMap<String, Vec<int>>`.
The `generic_args` are themselves `TypeAnnotation`s, so they compose recursively.

`Array` covers `[T; N]` — fixed-size arrays. These are different from `Vec<T>`: a fixed-size
array has its size known at compile time.

---

## `StructDef` and `StructKind`

```rust
pub struct StructDef {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub kind: StructKind,
    pub visibility: Visibility,
    pub span: Span,
}

pub enum StructKind {
    Named(Vec<StructField>),    // struct Point { x: float, y: float }
    Tuple(Vec<TypeAnnotation>), // struct Pair(int, int);
    Unit,                       // struct Marker;
}
```

All three Rust struct forms are supported. Tuple structs (`struct Pair(int, int)`) are
handled specially in IR gen — their fields are accessed as `.0`, `.1`, etc.

---

## `EnumDef` and `EnumVariantKind`

```rust
pub enum EnumVariantKind {
    Unit,                    // None
    Tuple(Vec<TypeAnnotation>), // Some(int)
    Struct(Vec<StructField>),   // Ok { value: int, .. }
}
```

Enum variants can be unit (no data), tuple (positional data), or struct (named fields).
All three are common in Oxy programs — `Option<T>` uses unit (`None`) and tuple (`Some(T)`).

---

## `Visibility`

```rust
pub enum Visibility {
    Pub,       // pub
    PubCrate,  // pub(crate)
    PubSuper,  // pub(super)
    Private,   // default (no keyword)
}
```

Visibility is checked by the type checker at field access and function call sites.
Private items are only accessible within their defining module. The type checker
enforces this at compile time using the module stack.
