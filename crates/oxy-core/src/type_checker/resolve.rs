use super::*;

impl TypeChecker {
    /// Resolve a type name through type aliases and module context.
    pub(super) fn resolve_type(&self, name: &str) -> TypeInfo {
        // `Self` resolves to the current impl type
        if name == "Self" {
            if let Some(ref impl_type) = self.current_impl_type {
                return TypeInfo::UserStruct {
                    name: impl_type.clone(),
                    generic_args: Vec::new(),
                };
            }
        }
        if let Some(alias) = self.type_aliases.get(name) {
            return TypeInfo::from_annotation(&Some(alias.clone())).unwrap_or(TypeInfo::Unknown);
        }
        // Try module-qualified type alias
        if !name.contains("::") {
            let module_prefix = self.module_stack.join("::");
            if !module_prefix.is_empty() {
                let qualified = format!("{}::{}", module_prefix, name);
                if let Some(alias) = self.type_aliases.get(&qualified) {
                    return TypeInfo::from_annotation(&Some(alias.clone()))
                        .unwrap_or(TypeInfo::Unknown);
                }
            }
        }
        // Try module-qualified struct name
        let resolved = self.resolve_struct_name(name);
        TypeInfo::from_name(&resolved)
    }

    /// True if `name` refers to a known user-defined type (struct, enum,
    /// type alias, use-alias) or an in-scope generic parameter / `Self`.
    pub(super) fn is_known_user_type(&self, name: &str) -> bool {
        if name == "Self" || name == "_" {
            return true;
        }
        // Strip any inline generic-arg suffix (`Pair<i64, i64>` → `Pair`),
        // which `current_impl_type` and a few legacy parser paths still
        // produce as a single string.
        let bare = match name.find('<') {
            Some(i) => &name[..i],
            None => name,
        };
        if self.current_generics.iter().any(|g| g == bare) {
            return true;
        }
        if self.struct_defs.contains_key(bare) || self.enum_defs.contains(bare) {
            return true;
        }
        if self.type_aliases.contains_key(bare) || self.use_aliases.contains_key(bare) {
            return true;
        }
        // Module-qualified lookup (e.g. `database::Record` when inside `database`).
        let module_prefix = self.module_stack.join("::");
        if !module_prefix.is_empty() {
            let qualified = format!("{}::{}", module_prefix, bare);
            if self.struct_defs.contains_key(&qualified)
                || self.enum_defs.contains(&qualified)
                || self.type_aliases.contains_key(&qualified)
            {
                return true;
            }
        }
        false
    }

    /// Walk a TypeInfo and error on any nested `UserStruct(name)` whose name
    /// isn't a known type. Used at type-annotation sites to surface
    /// `let v: Vec<bogus_name> = …` as a clean "unknown type" diagnostic
    /// instead of a downstream "type mismatch".
    pub(super) fn validate_type_known(&self, ty: &TypeInfo, span: Span) -> Result<(), FerriError> {
        match ty {
            TypeInfo::UserStruct { name, generic_args } => {
                if !self.is_known_user_type(name) {
                    // Specific fix-it for the retired Rust-style width zoo
                    // (i8 .. u64, isize, usize, f32) — point users at the
                    // single replacement they should use instead.
                    let suggestion = match name.as_str() {
                        "i8" | "i16" | "i32" | "i64" | "u16" | "u32" | "u64" | "isize"
                        | "usize" => Some("int"),
                        "u8" => Some("byte"),
                        "f32" | "f64" => Some("float"),
                        _ => None,
                    };
                    let message = match suggestion {
                        Some(repl) => format!(
                            "`{name}` is not an Oxy type — use `{repl}` instead. Oxy has only `int`, `byte`, and `float`."
                        ),
                        None => format!("unknown type `{name}`"),
                    };
                    return Err(FerriError::TypeError {
                        message,
                        line: span.line,
                        column: span.column,
                    });
                }
                for g in generic_args {
                    self.validate_type_known(g, span)?;
                }
                Ok(())
            }
            TypeInfo::Vec(t) | TypeInfo::Option(t) => self.validate_type_known(t, span),
            TypeInfo::HashMap(k, v) | TypeInfo::BTreeMap(k, v) | TypeInfo::Result(k, v) => {
                self.validate_type_known(k, span)?;
                self.validate_type_known(v, span)
            }
            TypeInfo::Array(elem, _) => self.validate_type_known(elem, span),
            TypeInfo::Function { params, ret } => {
                for p in params {
                    self.validate_type_known(p, span)?;
                }
                self.validate_type_known(ret, span)
            }
            _ => Ok(()),
        }
    }

    /// Resolve a TypeAnnotation to TypeInfo, handling Named and Array variants.
    pub(super) fn resolve_annotation(&self, ann: &TypeAnnotation) -> TypeInfo {
        match ann {
            TypeAnnotation::Named {
                name, generic_args, ..
            } => {
                let head = self.resolve_type(name);
                TypeInfo::apply_generics(head, generic_args).unwrap_or(TypeInfo::Unknown)
            }
            TypeAnnotation::Array { inner, size, .. } => {
                let inner_ty = self.resolve_annotation(inner);
                TypeInfo::Array(Box::new(inner_ty), *size)
            }
        }
    }

    /// Type of the trailing semicolon-less expression in a block, or `Unit`
    /// if the block has no producing tail.
    pub(super) fn block_tail_type(&mut self, block: &Block) -> Result<TypeInfo, FerriError> {
        let block_env = TypeEnv::child(&self.env);
        let saved = self.env.clone();
        self.env = block_env;
        let mut result = TypeInfo::Unit;
        let body_result = (|| -> Result<(), FerriError> {
            for stmt in &block.stmts {
                if let Stmt::Expr {
                    expr,
                    has_semicolon,
                } = stmt
                {
                    if !has_semicolon {
                        result = self.infer_expr(expr)?;
                        continue;
                    }
                }
                self.check_stmt(stmt, &TypeInfo::Unknown)?;
            }
            Ok(())
        })();
        self.env = saved;
        body_result?;
        Ok(result)
    }

    /// Combine two branch types from an `if`/`match`. `Unknown` and `Unit`
    /// arms are absorbed into the other side; otherwise the two must be
    /// mutually `accepts`-compatible.
    pub(super) fn unify_branch_types(
        &self,
        a: &TypeInfo,
        b: &TypeInfo,
        kind: &str,
        span: Span,
    ) -> Result<TypeInfo, FerriError> {
        if *a == TypeInfo::Unknown {
            return Ok(b.clone());
        }
        if *b == TypeInfo::Unknown {
            return Ok(a.clone());
        }
        if *a == TypeInfo::Unit {
            return Ok(b.clone());
        }
        if *b == TypeInfo::Unit {
            return Ok(a.clone());
        }
        if a.accepts(b) {
            return Ok(a.clone());
        }
        if b.accepts(a) {
            return Ok(b.clone());
        }
        Err(FerriError::TypeError {
            message: format!(
                "{kind} branches produce incompatible types `{}` and `{}`",
                a.display_name(),
                b.display_name()
            ),
            line: span.line,
            column: span.column,
        })
    }

    /// Substitute generic-parameter names with their resolved types.
    /// `param_names[i]` maps to `arg_types[i]`. The substitution happens on
    /// the raw `TypeAnnotation` so nested generics in field types like
    /// `Vec<T>` get properly recursed through.
    pub(super) fn substitute_generics(
        &self,
        ann: &TypeAnnotation,
        param_names: &[String],
        arg_types: &[TypeInfo],
    ) -> TypeInfo {
        match ann {
            TypeAnnotation::Named {
                name, generic_args, ..
            } => {
                if let Some(idx) = param_names.iter().position(|p| p == name) {
                    return arg_types.get(idx).cloned().unwrap_or(TypeInfo::Unknown);
                }
                let head = self.resolve_type(name);
                let resolved_args: Vec<TypeInfo> = generic_args
                    .iter()
                    .map(|a| self.substitute_generics(a, param_names, arg_types))
                    .collect();
                match head {
                    TypeInfo::Vec(_) if !resolved_args.is_empty() => {
                        TypeInfo::Vec(Box::new(resolved_args[0].clone()))
                    }
                    TypeInfo::Option(_) if !resolved_args.is_empty() => {
                        TypeInfo::Option(Box::new(resolved_args[0].clone()))
                    }
                    TypeInfo::HashMap(..) if resolved_args.len() >= 2 => TypeInfo::HashMap(
                        Box::new(resolved_args[0].clone()),
                        Box::new(resolved_args[1].clone()),
                    ),
                    TypeInfo::BTreeMap(..) if resolved_args.len() >= 2 => TypeInfo::BTreeMap(
                        Box::new(resolved_args[0].clone()),
                        Box::new(resolved_args[1].clone()),
                    ),
                    TypeInfo::Result(..) if resolved_args.len() >= 2 => TypeInfo::Result(
                        Box::new(resolved_args[0].clone()),
                        Box::new(resolved_args[1].clone()),
                    ),
                    TypeInfo::UserStruct { name, .. } if !resolved_args.is_empty() => {
                        TypeInfo::UserStruct {
                            name,
                            generic_args: resolved_args,
                        }
                    }
                    other => other,
                }
            }
            TypeAnnotation::Array { inner, size, .. } => TypeInfo::Array(
                Box::new(self.substitute_generics(inner, param_names, arg_types)),
                *size,
            ),
        }
    }

    /// Builtin method list for a given concrete type, or None if the type
    /// isn't a builtin we track (UserStruct / Unknown / etc).
    pub(super) fn builtin_methods_for(
        &self,
        ty: &TypeInfo,
    ) -> Option<&'static [symbols::MethodInfo]> {
        match ty {
            TypeInfo::Vec(_) => Some(symbols::VEC_METHODS),
            // Fixed-size arrays share Vec's read-only surface but disallow
            // mutators; we reuse VEC_METHODS and reject mutators in the
            // call-site check.
            TypeInfo::Array(..) => Some(symbols::VEC_METHODS),
            TypeInfo::String => Some(symbols::STRING_METHODS),
            TypeInfo::HashMap(..) => Some(symbols::HASHMAP_METHODS),
            TypeInfo::BTreeMap(..) => Some(symbols::BTREEMAP_METHODS),
            TypeInfo::Option(_) => Some(symbols::OPTION_METHODS),
            TypeInfo::Result(..) => Some(symbols::RESULT_METHODS),
            TypeInfo::Char => Some(symbols::CHAR_METHODS),
            t if t.is_integer() || t.is_float() => Some(symbols::NUMERIC_METHODS),
            _ => None,
        }
    }

    /// True if `method` is callable on `ty`, considering both type-specific
    /// methods and the generic methods available on every value.
    pub(super) fn method_exists_on(&self, ty: &TypeInfo, method: &str) -> bool {
        if symbols::GENERIC_METHODS.iter().any(|m| m.name == method) {
            return true;
        }
        if let Some(list) = self.builtin_methods_for(ty) {
            if list.iter().any(|m| m.name == method) {
                return true;
            }
        }
        // Collection-like types accept iterator methods directly (Oxy's
        // shortcut: `v.map(...)` instead of `v.iter().map(...).collect()`).
        if matches!(
            ty,
            TypeInfo::Vec(_)
                | TypeInfo::Array(..)
                | TypeInfo::HashMap(..)
                | TypeInfo::BTreeMap(..)
                | TypeInfo::Option(_)
                | TypeInfo::Result(..)
        ) && symbols::ITERATOR_METHODS.iter().any(|m| m.name == method)
        {
            return true;
        }
        false
    }

    /// Per-method element-type validation for parameterized containers, and
    /// a parameterized return type when known. Returns `Ok(Some(ret))` if the
    /// method was recognised and the caller should use `ret` as the call's
    /// type; `Ok(None)` to fall through to the generic return-type table.
    pub(super) fn check_builtin_method_args(
        &mut self,
        obj_ty: &TypeInfo,
        method: &str,
        args: &[Expr],
        arg_types: &[TypeInfo],
        span: Span,
    ) -> Result<Option<TypeInfo>, FerriError> {
        let check = |expected: &TypeInfo, pos: usize, label: &str| -> Result<(), FerriError> {
            let actual = arg_types.get(pos).cloned().unwrap_or(TypeInfo::Unknown);
            if !expected.accepts(&actual) {
                let aspan = args.get(pos).map(|a| a.span()).unwrap_or(span);
                return Err(FerriError::TypeError {
                    message: format!(
                        "type mismatch in `{method}` {label}: expected `{}`, got `{}`",
                        expected.display_name(),
                        actual.display_name()
                    ),
                    line: aspan.line,
                    column: aspan.column,
                });
            }
            Ok(())
        };
        match obj_ty {
            TypeInfo::Vec(elem) | TypeInfo::Array(elem, _) => match method {
                "push" | "contains" => {
                    check(elem, 0, "argument")?;
                    Ok(Some(if method == "contains" {
                        TypeInfo::Bool
                    } else {
                        TypeInfo::Unit
                    }))
                }
                "insert" => {
                    check(&TypeInfo::I64, 0, "index")?;
                    check(elem, 1, "value")?;
                    Ok(Some(TypeInfo::Unit))
                }
                "pop" | "first" | "last" | "min" | "max" => {
                    Ok(Some(TypeInfo::Option(elem.clone())))
                }
                "get" => {
                    check(&TypeInfo::I64, 0, "index")?;
                    Ok(Some(TypeInfo::Option(elem.clone())))
                }
                "remove" => {
                    check(&TypeInfo::I64, 0, "index")?;
                    Ok(Some(TypeInfo::Option(elem.clone())))
                }
                "iter" => Ok(Some(TypeInfo::Vec(elem.clone()))),
                "len" => Ok(Some(TypeInfo::I64)),
                "is_empty" => Ok(Some(TypeInfo::Bool)),
                "clone" => Ok(Some(obj_ty.clone())),
                _ => Ok(None),
            },
            TypeInfo::HashMap(k, v) | TypeInfo::BTreeMap(k, v) => match method {
                "insert" => {
                    check(k, 0, "key")?;
                    check(v, 1, "value")?;
                    Ok(Some(TypeInfo::Option(v.clone())))
                }
                "get" | "remove" => {
                    check(k, 0, "key")?;
                    Ok(Some(TypeInfo::Option(v.clone())))
                }
                "contains_key" => {
                    check(k, 0, "key")?;
                    Ok(Some(TypeInfo::Bool))
                }
                "keys" => Ok(Some(TypeInfo::Vec(k.clone()))),
                "values" => Ok(Some(TypeInfo::Vec(v.clone()))),
                "len" => Ok(Some(TypeInfo::I64)),
                "is_empty" => Ok(Some(TypeInfo::Bool)),
                _ => Ok(None),
            },
            TypeInfo::Option(inner) => match method {
                "unwrap" | "expect" => Ok(Some((**inner).clone())),
                "unwrap_or" => {
                    check(inner, 0, "default")?;
                    Ok(Some((**inner).clone()))
                }
                "is_some" | "is_none" => Ok(Some(TypeInfo::Bool)),
                _ => Ok(None),
            },
            TypeInfo::Result(t, e) => match method {
                "unwrap" => Ok(Some((**t).clone())),
                "unwrap_err" => Ok(Some((**e).clone())),
                "is_ok" | "is_err" => Ok(Some(TypeInfo::Bool)),
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    /// Fixed-size arrays forbid mutators (push/pop/etc). Returns true if
    /// `method` is a Vec mutator that should be rejected on an Array.
    pub(super) fn is_array_mutator(&self, method: &str) -> bool {
        matches!(
            method,
            "push"
                | "pop"
                | "insert"
                | "remove"
                | "swap_remove"
                | "clear"
                | "truncate"
                | "resize"
                | "extend"
                | "append"
                | "retain"
                | "drain"
                | "sort"
                | "sort_by"
                | "reverse"
                | "dedup"
        )
    }

    /// Check whether a struct field is visible from the current module context.
    pub(super) fn check_field_visible(
        &self,
        struct_name: &str,
        field_name: &str,
        span: Span,
    ) -> Result<(), FerriError> {
        if let Some(struct_def) = self.struct_defs.get(struct_name) {
            if let StructKind::Named(fields) = &struct_def.kind {
                for field in fields {
                    if field.name == field_name {
                        if matches!(field.visibility, Visibility::Private) {
                            let struct_module =
                                struct_name.rsplit_once("::").map(|(m, _)| m).unwrap_or("");
                            let current_module = self.module_stack.join("::");
                            if struct_module != current_module {
                                return Err(FerriError::Runtime {
                                    message: format!(
                                        "field `{}` of struct `{}` is private",
                                        field_name, struct_name
                                    ),
                                    line: span.line,
                                    column: span.column,
                                });
                            }
                        }
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    /// Check that the item at `qualified_name` (function, struct, or intermediate
    /// module) is visible from the current module context.
    pub(super) fn check_path_visible(
        &self,
        qualified_name: &str,
        span: Span,
    ) -> Result<(), FerriError> {
        let current_module = self.module_stack.join("::");

        // 1. Check function visibility.
        if let Some(fn_def) = self.fn_defs.get(qualified_name) {
            if !is_visible_from(
                &fn_def.visibility,
                qualified_name,
                &current_module,
                &self.module_vis,
            ) {
                return Err(FerriError::TypeError {
                    message: format!("function `{qualified_name}` is private"),
                    line: span.line,
                    column: span.column,
                });
            }
            // Even if the function is public, the module(s) containing it
            // might be private — check those too.
            check_module_path_visible(qualified_name, &current_module, &self.module_vis, span)?;
            return Ok(());
        }

        // 2. Check struct visibility.
        if let Some(sd) = self.struct_defs.get(qualified_name) {
            if !is_visible_from(
                &sd.visibility,
                qualified_name,
                &current_module,
                &self.module_vis,
            ) {
                return Err(FerriError::TypeError {
                    message: format!("struct `{qualified_name}` is private"),
                    line: span.line,
                    column: span.column,
                });
            }
            check_module_path_visible(qualified_name, &current_module, &self.module_vis, span)?;
            return Ok(());
        }

        // 3. Not found in fn_defs or struct_defs — still check module visibility.
        check_module_path_visible(qualified_name, &current_module, &self.module_vis, span)?;

        Ok(())
    }

    /// Resolve a struct name through use_aliases (for `use foo::Bar` → `Bar` unqualified).
    pub(super) fn resolve_struct_name(&self, name: &str) -> String {
        // `Self` resolves to the current impl type
        if name == "Self" {
            if let Some(ref impl_type) = self.current_impl_type {
                return impl_type.clone();
            }
        }
        // Check use_aliases for a direct alias
        if let Some(resolved) = self.use_aliases.get(name) {
            if self.struct_defs.contains_key(resolved) {
                return resolved.clone();
            }
        }
        // Try module-qualified name (current module prefix + name)
        if !name.contains("::") {
            let module_prefix = self.module_stack.join("::");
            if !module_prefix.is_empty() {
                let qualified = format!("{}::{}", module_prefix, name);
                if self.struct_defs.contains_key(&qualified) {
                    return qualified;
                }
            }
        }
        name.to_string()
    }
}

/// Walk intermediate path segments and verify module visibility.
fn check_module_path_visible(
    qualified_name: &str,
    current_module: &str,
    module_vis: &HashMap<String, Visibility>,
    span: Span,
) -> Result<(), FerriError> {
    let segments: Vec<&str> = qualified_name.split("::").collect();
    if segments.len() >= 2 {
        for i in 1..segments.len() {
            let module_path = segments[..i].join("::");
            if let Some(vis) = module_vis.get(&module_path) {
                if !is_visible_from(vis, &module_path, current_module, module_vis) {
                    return Err(FerriError::TypeError {
                        message: format!(
                            "module `{module_path}` is private and cannot be accessed \
                             from outside"
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
            }
        }
    }
    Ok(())
}

/// Find the nearest enclosing module for an item at `name` by walking
/// ancestor segments and checking `module_vis`. Returns the empty string
/// for top‑level items.
fn enclosing_module(name: &str, module_vis: &HashMap<String, Visibility>) -> String {
    let segments: Vec<&str> = name.split("::").collect();
    // Walk from right to left (item → parent → grandparent).
    // The name itself might be a module (single-segment), so include
    // the last segment in the scan.
    for i in (0..segments.len()).rev() {
        let candidate = segments[..=i].join("::");
        if module_vis.contains_key(&candidate) {
            return candidate;
        }
    }
    String::new()
}

/// Parent module: the module one level above the item's enclosing module.
fn parent_module(name: &str, module_vis: &HashMap<String, Visibility>) -> String {
    let enc = enclosing_module(name, module_vis);
    if enc.is_empty() {
        return String::new();
    }
    enc.rsplit_once("::")
        .map(|(p, _)| p.to_string())
        .unwrap_or_default()
}

/// Core visibility check: can `current_module` access an item with the given
/// `visibility` that lives at `item_path` (a qualified name like `"api::secret"`)?
fn is_visible_from(
    visibility: &Visibility,
    item_path: &str,
    current_module: &str,
    module_vis: &HashMap<String, Visibility>,
) -> bool {
    match visibility {
        Visibility::Pub => true,
        Visibility::PubCrate => true,
        Visibility::PubSuper => {
            let parent = parent_module(item_path, module_vis);
            current_module.is_empty()
                || current_module == parent
                || current_module.starts_with(&format!("{parent}::"))
        }
        Visibility::Private => {
            // For modules: accessible from parent and siblings.
            // For functions/structs: accessible only from same module
            // and siblings (NOT from parent).
            // We distinguish by checking if `item_path` IS a known module.
            let item_module = enclosing_module(item_path, module_vis);
            if item_module == current_module {
                return true;
            }
            let item_parent = item_module.rsplit_once("::").map(|(p, _)| p).unwrap_or("");
            let cur_parent = current_module
                .rsplit_once("::")
                .map(|(p, _)| p)
                .unwrap_or("");
            let item_is_module = module_vis.contains_key(item_path);

            if item_is_module {
                // Private module: accessible from parent and siblings.
                current_module == item_parent  // parent
                    || (item_parent == cur_parent  // sibling
                        && cur_parent != current_module)
                    || (item_parent.is_empty()
                        && current_module.is_empty()
                        && item_module == item_path) // file-level
            } else {
                // Private function/struct: accessible from siblings only
                // (same module already handled above).
                item_parent == cur_parent && cur_parent != current_module
            }
        }
    }
}
