# Field Visibility and Module Boundaries

Visibility checking is one of the quieter, subtler jobs the type checker does, and it's the one
that makes encapsulation real. When you write a struct and mark a field private, you're making a
promise to yourself: callers outside this module cannot touch this field, so I'm free to change how
it's represented without breaking them. That promise is worth exactly nothing unless something
enforces it — and the type checker is what enforces it, at compile time, before any code runs. This
chapter is about how it knows where "inside" and "outside" a module are, and how it turns a private
field into a hard compile error the moment someone reaches for it from the wrong place.

## The visibility model

Oxy follows Rust's visibility model:
- `pub` — visible everywhere
- `pub(crate)` — visible within the crate
- `pub(super)` — visible in the parent module
- private (default) — visible only within the defining module and its descendants

The type checker enforces this at three points:
1. **Field access**: `point.x` — is `x` visible from the current module?
2. **Struct initialization**: `Point { x: 1, y: 2 }` — are all named fields visible?
3. **Function/path calls**: `module::private_fn()` — is `private_fn` visible?

## How the type checker tracks "where we are"

The `module_stack` field in `TypeChecker` tracks the current module nesting:

```rust
pub struct TypeChecker {
    module_stack: Vec<String>,
    // ...
}
```

When checking items in `mod geometry { ... }`, the type checker pushes `"geometry"` onto
`module_stack`. When checking nested `mod geometry::shapes { ... }`, it pushes `"shapes"`.
When done with the module, it pops.

The current module path is `module_stack.join("::")`.

## Checking field access

When the type checker sees `point.x`:

```rust
Expr::FieldAccess { object, field, span } => {
    let obj_type = self.infer_expr(object)?;

    // Get the struct definition
    let struct_name = match &obj_type {
        TypeInfo::UserStruct { name, .. } => name.clone(),
        _ => return Err(/* not a struct */),
    };

    let struct_def = self.struct_defs.get(&struct_name)
        .ok_or_else(|| /* unknown struct */)?;

    // Find the field
    let field_def = struct_def.find_field(field)
        .ok_or_else(|| /* no such field */)?;

    // Check visibility
    self.check_field_visible(field_def, &struct_def.name, *span)?;

    Ok(TypeInfo::from_annotation(&Some(field_def.type_ann.clone())))
}
```

`check_field_visible` compares the field's visibility against the current `module_stack`:

```rust
fn check_field_visible(&self, field: &StructField, struct_module: &str, span: Span)
    -> Result<(), PipelineError>
{
    match field.visibility {
        Visibility::Pub => Ok(()), // always visible
        Visibility::Private => {
            // Visible only if current module matches the struct's defining module
            let current = self.module_stack.join("::");
            if current == struct_module || current.starts_with(&format!("{struct_module}::")) {
                Ok(())
            } else {
                Err(PipelineError::TypeError {
                    message: format!("field '{}' is private", field.name),
                    line: span.line,
                    column: span.column,
                })
            }
        }
        // pub(crate) and pub(super) handled similarly
    }
}
```

## The forward-reference problem in visibility

There is a subtle ordering problem. Consider:

```rust
mod geometry {
    pub struct Point {
        pub x: float,
        pub y: float,
    }

    pub fn origin() -> Point {
        Point { x: 0.0, y: 0.0 }
    }
}

fn main() {
    let p = geometry::origin();
    println(p.x);  // is Point::x visible here?
}
```

When checking `p.x` in `main`, the type checker must know that `Point` is defined in
`geometry` and that `x` is `pub`. This works because:
1. `collect_defs` registered `"geometry::Point"` with its field visibility
2. The field `x` has `Visibility::Pub`
3. `check_field_visible` with `Visibility::Pub` returns `Ok(())` unconditionally

If `x` were private, the error would fire at `p.x` in `main` — the correct location.

## The `pub_vis` prescan requirement

When checking a forward reference to a function defined later in the file, the type
checker must have seen that function's visibility in pass 1. The `collect_defs` pass
does this — it registers function visibility alongside their definitions.

If a function visibility is not registered in the prescan, then `is_visible()` returns
`true` for untracked items (the fallback is "assume visible"). This is safe-default behavior
but means private functions would accidentally be accessible. The CLAUDE.md anti-patterns
section flags: **register `pub_vis` in prescan** — not doing so causes this silent bug.

## Testing visibility

Visibility is tested with `#[compile_error]` tests in `.ox` files:

```rust
// examples/features/modules/visibility.ox

mod private_mod {
    struct Secret {
        data: String,
    }
    pub fn make() -> Secret { Secret { data: "classified" } }
}

#[compile_error]
fn cannot_access_private_field() {
    let s = private_mod::make();
    println(s.data);  // ERROR: field 'data' is private
}
```

The `#[compile_error]` annotation marks a test that must fail at type-check time. If the
type checker does not reject it, the test fails — "expected compile error, got none."
