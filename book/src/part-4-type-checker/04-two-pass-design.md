# The Two-Pass Design: Collect Then Check

There's a problem you hit the moment you let people call functions that are defined later in the
file. If the type checker reads top to bottom and reaches a call to `foo()` before it has ever seen
`foo`'s definition, it has no idea what `foo` returns or what arguments it expects — and so it can't
check the call. Older languages dodged this by *forbidding* it: C makes you write a forward
declaration, declaring the function's signature up top before you're allowed to use it. That works,
but it's a chore, and Oxy chose not to inflict it. In Oxy you can call `greet` from `main` even if
`greet` is defined fifty lines below.

The way to have that without forward declarations is to read the program *twice*. The first pass
doesn't check anything — it just skims the whole program and writes down every name and its
signature: every struct, every function's parameters and return type. Think of it as reading the
table of contents before you read the chapters. Once that index exists, the second pass can walk
the actual bodies and check every call against it, and it no longer matters what order things were
written in, because by then it already knows about all of them. That's the two-pass design, and the
rest of this chapter is how Oxy implements it — including one invariant that, if you get it wrong,
makes the whole type checker silently stop catching errors.

## The problem: forward references

Consider this valid Oxy program:

```rust
fn main() {
    println(greet("world"));  // calls greet before it's defined
}

fn greet(name: String) -> String {
    f"Hello, {name}!"
}
```

If the type checker visits `main` first (top to bottom), it encounters the call to `greet`
before it has seen `greet`'s definition. It does not know what type `greet` returns.

One solution: require `greet` to be declared before `main`. Oxy does not do this.

The actual solution: **two passes**. The first pass collects all definitions (names, types,
return types). The second pass type-checks bodies using the information from the first pass.

## Pass 1: `collect_defs` + `collect_fn_types`

```rust
// crates/oxy-core/src/type_checker/mod.rs
pub fn check_program(&mut self, program: &Program) -> Result<(), PipelineError> {
    // Pass 1a: collect struct defs, type aliases, use aliases
    self.collect_defs(&program.items, "");

    // Pass 1b: collect function and method return types
    self.collect_fn_types(&program.items, "");

    // Pass 2: type-check each item's body
    for item in &program.items {
        self.check_item(item)?;
    }
    Ok(())
}
```

**`collect_defs`** walks all items and registers:
- Struct definitions (field names + types)
- Enum definitions (variant names)
- Type aliases
- `use` aliases (short name → qualified name)
- Module visibility

It does **not** execute any type checking. It just records "these names exist."

**`collect_fn_types`** walks all functions and methods and registers:
- `fn_return_types["greet"] = TypeInfo::String`
- `fn_param_types["greet"] = [TypeInfo::String]`

After these two passes, the type checker knows about every function and struct in the
program, regardless of definition order. Pass 2 can then check any call to `greet` and
know its return type.

## Pass 1 must handle modules and impls

A critical requirement: `collect_fn_types` must recurse into **modules** and **impl blocks**:

```rust
fn collect_fn_types(&mut self, items: &[Item], prefix: &str) {
    for item in items {
        match item {
            Item::Function(fn_def) => {
                let qualified_name = if prefix.is_empty() {
                    fn_def.name.clone()
                } else {
                    format!("{prefix}::{}", fn_def.name)
                };
                let ret = TypeInfo::from_annotation(&fn_def.return_type);
                self.fn_return_types.insert(qualified_name, ret);
            }

            Item::Module(module) => {
                // Recurse with extended prefix
                let new_prefix = if prefix.is_empty() {
                    module.name.clone()
                } else {
                    format!("{prefix}::{}", module.name)
                };
                if let Some(items) = &module.body {
                    self.collect_fn_types(items, &new_prefix);
                }
            }

            Item::Impl(impl_block) => {
                // Register methods under "TypeName::method_name"
                for method in &impl_block.methods {
                    let key = format!("{}::{}", impl_block.type_name, method.name);
                    let ret = TypeInfo::from_annotation(&method.return_type);
                    self.fn_return_types.insert(key, ret.clone());
                    if !prefix.is_empty() {
                        let prefixed = format!("{prefix}::{}::{}", impl_block.type_name, method.name);
                        self.fn_return_types.insert(prefixed, ret);
                    }
                }
            }

            Item::ImplTrait(impl_trait) => {
                // Same as Impl — must not be skipped
                for method in &impl_trait.methods {
                    // register under "TypeName::method_name"
                }
            }
            _ => {}
        }
    }
}
```

**The anti-pattern to avoid:** stopping at `Item::Function` and ignoring `Item::Impl`.
If impl methods are not registered in pass 1, then calls to those methods in pass 2
return `TypeInfo::Unknown` — which silently passes type checking because `Unknown.accepts(anything)`.
This would mean type errors in method calls go undetected. The CLAUDE.md explicitly flags
this as a critical invariant: **do not skip `Item::Impl` / `Item::ImplTrait` in `collect_fn_types`**.

## Pass 2: `check_item`

After the two collection passes, pass 2 type-checks each item's body:

```rust
for item in &program.items {
    self.check_item(item)?;
}
```

`check_item` dispatches to `check_fn`, `check_struct`, `check_impl`, etc. Each of these
walks the body — statements and expressions — calling `check_stmt` and `infer_expr`.

For a function call `greet("world")`:
1. Look up `"greet"` in `fn_return_types` → `TypeInfo::String`
2. Look up `"greet"` in `fn_param_types` → `[TypeInfo::String]`
3. Infer the type of each argument: `"world"` → `TypeInfo::String`
4. Check: `TypeInfo::String.accepts(TypeInfo::String)` → `true`
5. Return `TypeInfo::String` as the inferred type of the call expression

Because `fn_return_types` was populated in pass 1, step 1 succeeds even though `greet`
appears after `main` in the source.

## Why not three passes?

One pass for definitions, one pass for signatures, one pass for bodies — could three passes
handle more complex cases?

Oxy's design does not need a third pass because:
- Struct fields are fully typed from their declaration (no inference needed)
- Generic functions use `TypeInfo::Unknown` for generic params (inference placeholder)
- Circular dependencies (A calls B, B calls A) are handled because both return types are
  registered before either body is checked

A third pass would be needed for, say, inferring function return types without annotations.
Oxy requires explicit return type annotations, which eliminates this need.
