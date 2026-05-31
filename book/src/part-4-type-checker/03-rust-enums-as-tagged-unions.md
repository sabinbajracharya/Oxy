# Rust Concepts: Enums as Tagged Unions, Pattern Matching

You already know what a Rust enum is — we met them back in Part 1, where `TokenKind` was a label
with an occasional payload. Now we go deeper, because the type checker shows enums doing the thing
they're really for: representing "this *or* that *or* that," a closed set of mutually exclusive
possibilities, each carrying exactly the data its case needs. `TypeInfo` — the enum that answers
"what type is this?" — is the best example in the whole codebase, because an Oxy type genuinely *is*
one-of-many (it's an int, *or* a Vec of something, *or* a user struct named X) and several of those
cases nest recursively. Pair that with pattern matching and you get code that takes a type apart as
naturally as you'd describe it in English. You know the syntax; this chapter is about the power.

## `TypeInfo` as a tagged union

The type checker's central type is `TypeInfo` — an enum representing every possible Oxy type:

```rust
pub enum TypeInfo {
    I64,      // int
    U8,       // byte
    F64,      // float
    Bool,
    String,
    Char,
    Unit,
    Vec(Box<TypeInfo>),
    HashMap(Box<TypeInfo>, Box<TypeInfo>),
    Option(Box<TypeInfo>),
    Result(Box<TypeInfo>, Box<TypeInfo>),
    UserStruct { name: String, generic_args: Vec<TypeInfo> },
    Function { params: Vec<TypeInfo>, ret: Box<TypeInfo> },
    Future(Box<TypeInfo>),
    Array(Box<TypeInfo>, usize),
    Unknown,
}
```

This is a **tagged union**: at any moment, a `TypeInfo` value is exactly one of these
variants, and the variant tag tells you which one. Pattern matching extracts the data:

```rust
fn describe(ty: &TypeInfo) -> String {
    match ty {
        TypeInfo::I64 => "int".to_string(),
        TypeInfo::Vec(inner) => format!("Vec<{}>", describe(inner)),
        TypeInfo::Option(inner) => format!("Option<{}>", describe(inner)),
        TypeInfo::UserStruct { name, generic_args } if generic_args.is_empty() => {
            name.clone()
        }
        TypeInfo::UserStruct { name, generic_args } => {
            format!("{}<{}>", name, generic_args.iter().map(describe).collect::<Vec<_>>().join(", "))
        }
        TypeInfo::Unknown => "?".to_string(),
        // ... etc
    }
}
```

## Guards: `if` conditions in match arms

Pattern arms can have guards — conditions that must also be true for the arm to match:

```rust
match ty {
    TypeInfo::UserStruct { name, generic_args } if generic_args.is_empty() => {
        // matches UserStruct only when generic_args is empty
        format!("{name}")
    }
    TypeInfo::UserStruct { name, generic_args } => {
        // matches UserStruct when generic_args is not empty
        format!("{name}<...>")
    }
    _ => "other".to_string()
}
```

Guards allow discriminating within a single variant based on field values.

## Nested pattern matching

Patterns can nest. To match `Option<Vec<int>>`:

```rust
match ty {
    TypeInfo::Option(inner) => match inner.as_ref() {
        TypeInfo::Vec(element) => match element.as_ref() {
            TypeInfo::I64 => println!("Option<Vec<int>>"),
            _ => println!("Option<Vec<something else>>"),
        },
        _ => println!("Option<something else>"),
    },
    _ => println!("not an Option"),
}
```

In practice, the type checker uses `matches!` for simple checks and `if let` for
single-variant extraction, reserving full `match` for multi-variant dispatch:

```rust
// Simple check: is this an integer type?
if matches!(ty, TypeInfo::I64 | TypeInfo::U8) { ... }

// Single-variant extraction:
if let TypeInfo::Vec(inner) = ty {
    // inner: &Box<TypeInfo>
    check_element_type(inner)?;
}

// Multi-variant dispatch:
match ty {
    TypeInfo::I64 | TypeInfo::U8 => handle_integer(),
    TypeInfo::F64 => handle_float(),
    TypeInfo::String => handle_string(),
    TypeInfo::Vec(inner) => handle_vec(inner),
    TypeInfo::UserStruct { name, .. } => handle_struct(name),
    _ => return Err(/* unexpected type */),
}
```

## The `accepts` method: type compatibility

The most important method on `TypeInfo` is `accepts`:

```rust
pub fn accepts(&self, other: &TypeInfo) -> bool {
    if *self == TypeInfo::Unknown || *other == TypeInfo::Unknown {
        return true;
    }
    if self == other {
        return true;
    }
    // int accepts byte (both are integer types)
    if self.is_integer() && other.is_integer() {
        return true;
    }
    // float accepts int (widening)
    if self.is_float() && other.is_integer() {
        return true;
    }
    // ... structural compatibility for containers
    false
}
```

`self.accepts(other)` means "a variable of type `self` can hold a value of type `other`."
This is used everywhere: function argument checking, assignment checking, return type checking.

The `Unknown` short-circuit is what makes inference placeholders safe — `Unknown` is
compatible with everything, so the type checker does not reject partially-inferred types.

## `PartialEq` derived for structural equality

`TypeInfo` derives `PartialEq`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TypeInfo { ... }
```

This makes `ty1 == ty2` work and compares structurally: two `Vec(Box(I64))` values
are equal because their inner types are equal, even though they are separate heap allocations.
Rust derives `PartialEq` recursively through all fields and variants.

This is used in the type checker constantly: `if inferred == declared { ... }`.
