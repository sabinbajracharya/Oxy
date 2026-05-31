# Inference vs Annotation

<!-- OPUS_FILL
Write a 2-paragraph intro.
The tension: annotations are explicit (easy to check, verbose) vs inference
(concise, requires a smarter algorithm). Oxy does both — annotations where declared,
inference for expressions.
Make it concrete: `let x: int = 42` (annotation), `let x = 42` (inference from literal).
Both are valid Oxy. The type checker handles both.
-->

## Type annotation: explicit declaration

When you write a type annotation, you tell the type checker exactly what type to expect:

```rust
let x: int = 42;          // annotation: x is int
let name: String = "hi";  // annotation: name is String
fn add(a: int, b: int) -> int { a + b }  // parameters and return annotated
```

Checking annotations is straightforward: infer the type of the right-hand side expression,
then verify it matches the declared type. For `let x: int = "hello"`, the inferred type is
`String`, the declared type is `int`, and `int.accepts(String)` is false → type error.

Function parameters in Oxy **must** be annotated. This is a deliberate choice: function
signatures are the contract between callers and the function body. Requiring explicit types
on parameters means the type checker can verify callers without looking at the function body.

## Type inference: deducing from context

When you omit the type annotation, the type checker infers it:

```rust
let x = 42;          // inferred: int (from integer literal)
let y = 3.14;        // inferred: float (from float literal)
let s = "hello";     // inferred: String (from string literal)
let v = Vec::new();  // inferred: Vec<Unknown> (no element type yet)
```

Inference rules for each expression type:

| Expression | Inferred type |
|-----------|---------------|
| `42` | `int` |
| `3.14` | `float` |
| `true` / `false` | `bool` |
| `"hello"` | `String` |
| `'a'` | `char` |
| `x` (variable) | whatever type `x` was declared/inferred with |
| `a + b` | same as `a` (or `float` if either operand is `float`) |
| `a == b` | `bool` |
| `foo(args)` | the declared return type of `foo` |
| `obj.field` | the declared type of `field` in obj's struct |
| `if c { t } else { e }` | the type of `t` (must match `e`) |

## The `Unknown` type: inference placeholder

When the type checker cannot determine a type (usually because a generic container has no
elements yet), it returns `TypeInfo::Unknown`.

`Unknown` is propagated safely through `accepts()`:

```rust
pub fn accepts(&self, other: &TypeInfo) -> bool {
    if *self == TypeInfo::Unknown || *other == TypeInfo::Unknown {
        return true;  // Unknown accepts anything, anything accepts Unknown
    }
    // ...
}
```

This means `let v: Vec<int> = Vec::new()` works: `Vec::new()` returns `Vec<Unknown>`,
and `Vec<int>.accepts(Vec<Unknown>)` is true. The vector's element type is pinned
to `int` by the annotation.

`Unknown` is **not** a user-visible type — it is an internal inference placeholder. It
cannot appear in function signatures or struct fields.

## Oxy's approach: local inference, explicit signatures

Oxy uses **local type inference**: the type of each expression is determined from the
expression itself and its immediate context, without global constraint solving.

This is simpler to implement than Hindley-Milner (which infers types for all expressions
simultaneously using constraint unification), and it produces better error messages
(the error points at the specific expression that disagrees, not at a constraint that
was generated far away).

The trade-off: some things that Hindley-Milner would accept, Oxy requires explicit annotations for:

```rust
// This requires a type annotation because Vec::new() alone gives Vec<Unknown>
let v: Vec<int> = Vec::new();

// Without annotation, pushing an int later would be fine, but
// the type of v would remain Vec<Unknown> throughout
let v = Vec::new();
v.push(42);  // v is now Vec<Unknown>, 42 is int — accepts() returns true
```

This is an intentional simplification: explicit annotations on collections are required,
reducing the type checker's complexity significantly.
