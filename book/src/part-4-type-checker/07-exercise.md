# Exercise: Add a Type Error

<!-- OPUS_FILL
Write a 1-paragraph framing. The exercises involve making the type checker stricter —
adding a check that currently does not exist. This is the kind of work that makes a
language safer over time. Frame it as: "You are about to make Oxy catch a real bug
that it currently silently ignores."
-->

## Part A: Reject `i32` and other retired type names

Oxy's type checker should reject Rust-style integer widths (`i32`, `u64`, `isize`, etc.)
with a fix-it message suggesting `int`. Currently the type checker accepts any unknown
identifier as `TypeInfo::UserStruct { name: "i32", .. }` — silently allowing code that
should be rejected.

Add this check to `TypeInfo::from_name`:

```rust
pub fn from_name(name: &str) -> TypeInfo {
    // Check for retired Rust-style integer types
    match name {
        "i8" | "i16" | "i32" | "i64" | "u16" | "u32" | "u64" | "isize" | "usize" => {
            // Can't return error here (from_name returns TypeInfo, not Result)
            // but we can return a sentinel that triggers an error in the caller
            // OR: add a check in from_annotation that calls a validation function
        }
        "u8" => { /* suggest byte */ }
        "f32" | "f64" => { /* suggest float */ }
        // ...
    }
}
```

The challenge: `from_name` returns `TypeInfo`, not `Result<TypeInfo>`. You'll need to
either:
1. Change `from_name` to `from_name_checked` that returns `Result<TypeInfo, PipelineError>`, or
2. Add a separate validation pass in `from_annotation` that calls a check function

Look at how the existing check works (grep for `i32` or `i8` in the type checker source
to see if there are already any checks). Write a `#[compile_error]` test:

```rust
// examples/features/types/retired_types.ox
#[compile_error]
fn uses_i32() {
    let x: i32 = 42;
}

#[compile_error]
fn uses_f32() {
    let y: f32 = 3.14;
}
```

---

## Part B: Detect `break` outside a loop

The type checker tracks `loop_depth` but the exercise is to verify that this check
actually works. Write a `#[compile_error]` test:

```rust
#[compile_error]
fn break_outside_loop() {
    break;
}
```

Run the tests. If this already fails at compile time (the test passes), the check is
already implemented. If not, find where `check_stmt` handles `Stmt::Break` and add:

```rust
Stmt::Break { span, .. } => {
    if self.loop_depth == 0 {
        return Err(PipelineError::TypeError {
            message: "break outside of loop".to_string(),
            line: span.line,
            column: span.column,
        });
    }
    Ok(TypeInfo::Unit)
}
```

---

## Part C: Write a `#[compile_error]` test that exercises field visibility

Write an Oxy program in `examples/features/modules/private_field_access.ox` with:

1. A module containing a struct with a private field
2. A `#[test]` function inside the module that reads the private field (should pass)
3. A `#[compile_error]` function outside the module that reads the private field (should fail)

```rust
mod bank {
    pub struct Account {
        pub owner: String,
        balance: int,  // private
    }

    pub fn new(owner: String) -> Account {
        Account { owner, balance: 0 }
    }

    #[test]
    fn can_read_balance_from_inside() {
        let acc = new("Alice");
        assert_eq(acc.balance, 0);  // OK — we're inside bank module
    }
}

#[compile_error]
fn cannot_read_balance_from_outside() {
    let acc = bank::new("Bob");
    println(acc.balance);  // ERROR — private field
}
```

Run: `docker compose run --rm dev bash -c "cargo test -p oxy-core -- feature_examples"`

---

## Reflection: why is `Unknown` dangerous?

Look at the `accepts` method:

```rust
pub fn accepts(&self, other: &TypeInfo) -> bool {
    if *self == TypeInfo::Unknown || *other == TypeInfo::Unknown {
        return true;
    }
    // ...
}
```

Explain in your own words:
1. When does `Unknown` appear as an expression's inferred type?
2. If `fn_return_types.get("greet")` returns `None` and we fall back to `TypeInfo::Unknown`,
   what happens to the call site's type check?
3. This is the CLAUDE.md anti-pattern: "Use `TypeInfo::from_name()` instead of inline
   string matching." What goes wrong if you write `if name == "int" { TypeInfo::I64 } else { TypeInfo::Unknown }`
   and forget to handle `"String"`?
