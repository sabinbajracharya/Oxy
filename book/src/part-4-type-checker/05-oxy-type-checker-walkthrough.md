# Oxy's Type Checker: A Full Walkthrough

Enough theory — let's open the type checker and push a real program through it. Like the parser,
the type checker is split by responsibility across several files: `mod.rs` holds `TypeInfo` and the
`TypeChecker` struct, `collect.rs` is the two collection passes, `check_item.rs` and `check_stmt.rs`
and `check_expr.rs` are the second-pass walkers for items, statements, and expressions, and
`resolve.rs` handles module and visibility resolution. This chapter is code-heavy and trace-driven:
open `mod.rs` and follow along as we walk a few representative expressions from source down to the
`TypeInfo` each produces.

**Files:**
- `crates/oxy-core/src/type_checker/mod.rs` — `TypeInfo`, `TypeChecker` struct, `check_program`
- `crates/oxy-core/src/type_checker/collect.rs` — pass 1: `collect_defs`, `collect_fn_types`
- `crates/oxy-core/src/type_checker/check_item.rs` — pass 2: `check_item`, `check_fn`
- `crates/oxy-core/src/type_checker/check_expr.rs` — expression type inference
- `crates/oxy-core/src/type_checker/check_stmt.rs` — statement type checking
- `crates/oxy-core/src/type_checker/resolve.rs` — module + visibility resolution

Open `mod.rs` now. This chapter walks through the key paths.

---

## Entry point: `check_program`

```rust
// crates/oxy-core/src/type_checker/mod.rs
pub fn check_program(&mut self, program: &Program) -> Result<(), PipelineError> {
    self.collect_defs(&program.items, "");        // pass 1a
    self.collect_fn_types(&program.items, "");    // pass 1b
    for item in &program.items {
        self.check_item(item)?;                   // pass 2
    }
    Ok(())
}
```

Three lines, two phases. After this returns `Ok(())`, the program is type-safe.

---

## The `TypeChecker` struct: what it tracks

The `TypeChecker` struct has 20 fields. The important ones:

| Field | Type | Purpose |
|-------|------|---------|
| `env` | `Rc<RefCell<TypeEnv>>` | Current scope's variable types |
| `struct_defs` | `HashMap<String, StructDef>` | All struct definitions |
| `fn_return_types` | `HashMap<String, TypeInfo>` | Return type per function |
| `fn_param_types` | `HashMap<String, Vec<TypeInfo>>` | Param types per function |
| `use_aliases` | `HashMap<String, String>` | `use` import short names |
| `module_stack` | `Vec<String>` | Current module nesting |
| `current_impl_type` | `Option<String>` | `Self` resolution in impl blocks |
| `current_fn_return` | `TypeInfo` | For `?` operator validation |
| `loop_depth` | `usize` | For `break`/`continue` validation |

The `TypeEnv` (scope chain for variables) mirrors the `Environment` from the runtime —
same chain model, but storing types instead of values.

---

## Tracing `let x: int = 42 + 1`

**In `check_stmt` for `Stmt::Let`:**

```rust
Stmt::Let { name, mutable, type_ann, value, .. } => {
    // 1. If there's a type annotation, resolve it to TypeInfo
    let declared = TypeInfo::from_annotation(type_ann.as_ref());

    // 2. If there's a value expression, infer its type
    let inferred = if let Some(expr) = value {
        self.infer_expr(expr)?
    } else {
        TypeInfo::Unit
    };

    // 3. Check compatibility
    if !declared.accepts(&inferred) {
        return Err(type_error("type mismatch", declared, inferred, span));
    }

    // 4. Register the binding in the current scope
    let ty = if declared != TypeInfo::Unknown { declared } else { inferred };
    self.env.borrow_mut().define_mut(name, ty, *mutable);
    Ok(())
}
```

For `let x: int = 42 + 1`:
1. `declared = TypeInfo::I64` (from `int`)
2. `inferred = infer_expr(BinaryOp(42, +, 1))` = `TypeInfo::I64`
3. `TypeInfo::I64.accepts(TypeInfo::I64)` = `true` ✓
4. `x` is registered as `TypeInfo::I64` in scope

---

## Tracing `infer_expr` for `42 + 1`

**In `check_expr.rs`, `infer_expr` for `Expr::BinaryOp`:**

```rust
Expr::BinaryOp { left, op, right, .. } => {
    let lt = self.infer_expr(left)?;
    let rt = self.infer_expr(right)?;
    match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
            // If either side is float, result is float
            if lt.is_float() || rt.is_float() {
                Ok(TypeInfo::F64)
            } else if lt.is_integer() && rt.is_integer() {
                Ok(TypeInfo::I64)
            } else {
                Err(type_error_binary(op, &lt, &rt, span))
            }
        }
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
            Ok(TypeInfo::Bool)  // comparisons always produce bool
        }
        BinOp::And | BinOp::Or => {
            // Both sides must be bool
            if !matches!(lt, TypeInfo::Bool) || !matches!(rt, TypeInfo::Bool) {
                Err(/* expected bool */)
            } else {
                Ok(TypeInfo::Bool)
            }
        }
    }
}
```

For `42 + 1`:
- `infer_expr(42)` → `TypeInfo::I64`
- `infer_expr(1)` → `TypeInfo::I64`
- Both integers → result is `TypeInfo::I64`

---

## Tracing a function call: `add(x, y)`

**In `infer_expr` for `Expr::Call`:**

```rust
Expr::Call { callee, args, .. } => {
    // 1. Get the function name from the callee expression
    let fn_name = self.resolve_callee(callee)?;

    // 2. Look up the return type
    let ret_type = self.fn_return_types.get(&fn_name)
        .cloned()
        .unwrap_or(TypeInfo::Unknown);

    // 3. Look up the parameter types
    if let Some(param_types) = self.fn_param_types.get(&fn_name) {
        if args.len() != param_types.len() {
            return Err(/* wrong number of args */);
        }
        for (arg, expected) in args.iter().zip(param_types.iter()) {
            let actual = self.infer_expr(arg)?;
            if !expected.accepts(&actual) {
                return Err(/* type mismatch in argument */);
            }
        }
    }

    // 4. Return the function's return type
    Ok(ret_type)
}
```

For `add(x, y)` where `add: fn(int, int) -> int`:
1. fn_name = `"add"`
2. ret_type = `TypeInfo::I64`
3. Infer `x` → `TypeInfo::I64`, check `I64.accepts(I64)` ✓. Same for `y`.
4. Return `TypeInfo::I64`

---

## The "did you mean?" suggestion

When a variable lookup fails, the type checker collects all visible names and finds
the closest match:

```rust
fn suggest_similar(name: &str, visible: &[String]) -> Option<String> {
    visible.iter()
        .filter(|n| edit_distance(name, n) <= 2)
        .min_by_key(|n| edit_distance(name, n))
        .cloned()
}
```

This is why Oxy says "undefined variable 'lenght', did you mean 'length'?" instead
of just "undefined variable 'lenght'."

The visible names come from `TypeEnv::collect_names()`, which walks up the scope chain
and collects every binding name visible from the current position.
