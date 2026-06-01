use super::*;

impl TypeChecker {
    /// Process a use definition (for both `Item::Use` and `Stmt::Use`).
    /// Checks path visibility and registers aliases or glob imports.
    pub(super) fn process_use_def(
        &mut self,
        use_def: &crate::ast::UseDef,
    ) -> Result<(), PipelineError> {
        let base_path = use_def.path.join("::");
        self.check_path_visible(&base_path, use_def.span)?;
        match &use_def.tree {
            UseTree::Simple(alias) => {
                let local_name = alias
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| use_def.path.last().cloned().unwrap_or_default());
                self.use_aliases.insert(local_name, base_path);
            }
            UseTree::Group(items) => {
                for (name, alias) in items {
                    let local_name = alias.as_ref().unwrap_or(name);
                    let qualified = format!("{}::{}", base_path, name);
                    self.check_path_visible(&qualified, use_def.span)?;
                    self.use_aliases.insert(local_name.clone(), qualified);
                }
            }
            UseTree::Glob => {
                self.glob_imports.push(base_path);
            }
        }
        Ok(())
    }

    /// Recursively collect struct defs, type aliases, and use aliases with module prefix.
    pub(super) fn collect_defs(&mut self, items: &[Item], prefix: &str) {
        for item in items {
            match item {
                Item::Struct(s) => {
                    let qualified = if prefix.is_empty() {
                        s.name.clone()
                    } else {
                        format!("{}::{}", prefix, s.name)
                    };
                    self.struct_defs.insert(qualified, s.clone());
                }
                Item::Enum(e) => {
                    let qualified = if prefix.is_empty() {
                        e.name.clone()
                    } else {
                        format!("{}::{}", prefix, e.name)
                    };
                    let variant_names: Vec<String> =
                        e.variants.iter().map(|v| v.name.clone()).collect();
                    for v in &variant_names {
                        self.enum_variant_names.insert(v.clone());
                    }
                    // Keyed under both the qualified and bare enum name so
                    // exhaustiveness lookups work regardless of how the matched
                    // value's type name was resolved.
                    self.enum_variants
                        .insert(e.name.clone(), variant_names.clone());
                    self.enum_variants.insert(qualified.clone(), variant_names);
                    self.enum_defs.insert(qualified);
                }
                Item::Const { name, .. } => {
                    let qualified = if prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{}::{}", prefix, name)
                    };
                    self.const_names.insert(name.clone());
                    self.const_names.insert(qualified);
                }
                Item::TypeAlias { name, target, .. } => {
                    let qualified = if prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{}::{}", prefix, name)
                    };
                    self.type_aliases.insert(qualified, target.clone());
                }
                Item::Module(m) => {
                    let nested_prefix = if prefix.is_empty() {
                        m.name.clone()
                    } else {
                        format!("{}::{}", prefix, m.name)
                    };
                    self.module_vis
                        .insert(nested_prefix.clone(), m.visibility.clone());
                    if let Some(body) = &m.body {
                        self.collect_defs(body, &nested_prefix);
                    }
                }
                _ => {}
            }
        }
    }

    /// Generic-parameter names declared on the struct identified by
    /// `qualified` (or its bare type name). Empty if not a known struct.
    pub(super) fn struct_generic_names(&self, qualified: &str) -> Vec<String> {
        let bare = qualified.rsplit("::").next().unwrap_or(qualified);
        self.struct_defs
            .get(qualified)
            .or_else(|| self.struct_defs.get(bare))
            .map(|def| def.generic_params.iter().map(|p| p.name.clone()).collect())
            .unwrap_or_default()
    }

    /// Resolve each param's declared type to `TypeInfo`. Names that match
    /// either the function's own generic params or `extra_generics` (e.g.
    /// generics from an enclosing impl block) become `TypeInfo::Unknown`
    /// so call-site checks don't false-positive against monomorphic args.
    pub(super) fn resolve_param_types(
        &self,
        f: &FnDef,
        extra_generics: &[String],
    ) -> Vec<TypeInfo> {
        let mut param_names: Vec<String> =
            f.generic_params.iter().map(|p| p.name.clone()).collect();
        for n in extra_generics {
            param_names.push(n.clone());
        }
        // Generic params resolve to Unknown so call-site accepts() passes for
        // any concrete arg; substitution recurses so `Vec<T>` and `Wrapper<T>`
        // also widen their inner T to Unknown.
        let unknowns: Vec<TypeInfo> = param_names.iter().map(|_| TypeInfo::Unknown).collect();
        f.params
            .iter()
            .map(|p| self.substitute_generics(&p.type_ann, &param_names, &unknowns))
            .collect()
    }

    /// Recursively register function return types with module prefix.
    /// Shared logic for `Item::Impl` and `Item::ImplTrait` — both have the same
    /// `FnDef`-based method registration.
    pub(super) fn collect_impl_methods(
        &mut self,
        methods: &[crate::ast::FnDef],
        type_name: &str,
        prefix: &str,
    ) {
        let mutating_names = compute_impl_mutating_methods(methods);
        let base = crate::ast::base_type_name(type_name);
        let type_prefix = if prefix.is_empty() {
            base.to_string()
        } else {
            format!("{}::{}", prefix, base)
        };
        let impl_generics = self.struct_generic_names(&type_prefix);
        for method in methods {
            let qualified = format!("{}::{}", type_prefix, method.name);
            let unqualified = format!("{}::{}", base, method.name);
            let ret_ty = if let Some(ref ann) = method.return_type {
                self.resolve_annotation(ann)
            } else {
                TypeInfo::Unit
            };
            let ret_ty = if method.is_async {
                TypeInfo::Future(Box::new(ret_ty))
            } else {
                ret_ty
            };
            let param_tys = self.resolve_param_types(method, &impl_generics);
            let mut all_gen_names: Vec<String> = impl_generics.clone();
            for p in &method.generic_params {
                all_gen_names.push(p.name.clone());
            }
            if !all_gen_names.is_empty() {
                let param_anns: Vec<TypeAnnotation> =
                    method.params.iter().map(|p| p.type_ann.clone()).collect();
                self.fn_generic_info.insert(
                    qualified.clone(),
                    (
                        all_gen_names.clone(),
                        param_anns.clone(),
                        method.return_type.clone(),
                    ),
                );
                self.fn_generic_info.insert(
                    unqualified.clone(),
                    (all_gen_names, param_anns, method.return_type.clone()),
                );
            }
            self.fn_return_types
                .insert(unqualified.clone(), ret_ty.clone());
            self.fn_return_types.insert(qualified.clone(), ret_ty);
            self.fn_param_types
                .insert(unqualified.clone(), param_tys.clone());
            self.fn_param_types.insert(qualified.clone(), param_tys);
            if mutating_names.contains(&method.name) {
                self.mutating_methods.insert(qualified.clone());
                self.mutating_methods.insert(unqualified);
            }
            self.fn_defs.insert(qualified, method.clone());
        }
    }

    pub(super) fn collect_fn_types(&mut self, items: &[Item], prefix: &str) {
        let saved_stack = self.module_stack.clone();
        self.module_stack = if prefix.is_empty() {
            vec![]
        } else {
            prefix.split("::").map(|s| s.to_string()).collect()
        };
        for item in items {
            match item {
                Item::Function(f) => {
                    let qualified = if prefix.is_empty() {
                        f.name.clone()
                    } else {
                        format!("{}::{}", prefix, f.name)
                    };
                    // Store generic info before resolution so call sites can
                    // enforce cross-param consistency on the same generic param
                    // and substitute concrete types into the return type.
                    if !f.generic_params.is_empty() {
                        let gen_names: Vec<String> =
                            f.generic_params.iter().map(|p| p.name.clone()).collect();
                        let param_anns: Vec<TypeAnnotation> =
                            f.params.iter().map(|p| p.type_ann.clone()).collect();
                        self.fn_generic_info.insert(
                            qualified.clone(),
                            (gen_names, param_anns, f.return_type.clone()),
                        );
                    }
                    let ret_ty = if let Some(ref ann) = f.return_type {
                        let is_generic = match ann {
                            TypeAnnotation::Named { name, .. } => {
                                let generic_names: Vec<&str> =
                                    f.generic_params.iter().map(|p| p.name.as_str()).collect();
                                generic_names.contains(&name.as_str())
                            }
                            TypeAnnotation::Array { .. } => false,
                        };
                        if is_generic {
                            TypeInfo::Unknown
                        } else {
                            self.resolve_annotation(ann)
                        }
                    } else {
                        TypeInfo::Unit
                    };
                    // async fn returns Future<T> — .await unwraps it
                    let ret_ty = if f.is_async {
                        TypeInfo::Future(Box::new(ret_ty))
                    } else {
                        ret_ty
                    };
                    let param_tys = self.resolve_param_types(f, &[]);
                    self.fn_return_types.insert(qualified.clone(), ret_ty);
                    self.fn_param_types.insert(qualified.clone(), param_tys);
                    self.fn_defs.insert(qualified, f.clone());
                }
                Item::Module(m) => {
                    let nested_prefix = if prefix.is_empty() {
                        m.name.clone()
                    } else {
                        format!("{}::{}", prefix, m.name)
                    };
                    if let Some(body) = &m.body {
                        self.collect_fn_types(body, &nested_prefix);
                    }
                }
                Item::Impl(i) => {
                    self.collect_impl_methods(&i.methods, i.base_type_name(), prefix);
                }
                Item::ImplTrait(i) => {
                    self.collect_impl_methods(&i.methods, i.base_type_name(), prefix);
                }
                _ => {}
            }
        }
        self.module_stack = saved_stack;
    }

    /// After `collect_fn_types`, resolve `pub use` re-exports inside modules.
    /// For each `pub use` item, register the re-exported name under the
    /// module-qualified key so external callers can find it.
    pub(super) fn resolve_reexports(&mut self, items: &[Item], prefix: &str) {
        for item in items {
            match item {
                Item::Use(use_def) => {
                    if !matches!(use_def.visibility, Visibility::Pub) {
                        continue;
                    }
                    if prefix.is_empty() {
                        continue;
                    }
                    match &use_def.tree {
                        UseTree::Simple(alias) => {
                            let source_path = use_def.path.join("::");
                            let local_name = alias.as_ref().cloned().unwrap_or_else(|| {
                                use_def.path.last().cloned().unwrap_or_default()
                            });
                            let reexport_key = format!("{prefix}::{local_name}");
                            self.register_reexport(&reexport_key, &source_path);
                        }
                        UseTree::Group(items) => {
                            for (name, alias) in items {
                                let source_path = format!("{}::{}", use_def.path.join("::"), name);
                                let local_name = alias.as_ref().unwrap_or(name);
                                let reexport_key = format!("{prefix}::{local_name}");
                                self.register_reexport(&reexport_key, &source_path);
                            }
                        }
                        UseTree::Glob => {
                            let source_mod = use_def.path.join("::");
                            self.register_glob_reexports(prefix, &source_mod);
                        }
                    }
                }
                Item::Module(m) => {
                    let nested_prefix = if prefix.is_empty() {
                        m.name.clone()
                    } else {
                        format!("{}::{}", prefix, m.name)
                    };
                    if let Some(body) = &m.body {
                        self.resolve_reexports(body, &nested_prefix);
                    }
                }
                _ => {}
            }
        }
    }

    /// Register a single re-export: `reexport_key` (e.g. "public_api::secret")
    /// points to `source_path` (e.g. "inner::secret").
    fn register_reexport(&mut self, reexport_key: &str, source_path: &str) {
        // Clone fn type info if the source is a known function.
        if let Some(ret_ty) = self.fn_return_types.get(source_path).cloned() {
            self.fn_return_types
                .insert(reexport_key.to_string(), ret_ty);
        }
        if let Some(param_tys) = self.fn_param_types.get(source_path).cloned() {
            self.fn_param_types
                .insert(reexport_key.to_string(), param_tys);
        }
        if let Some(fn_def) = self.fn_defs.get(source_path).cloned() {
            self.fn_defs.insert(reexport_key.to_string(), fn_def);
        }
        if let Some(gen_info) = self.fn_generic_info.get(source_path).cloned() {
            self.fn_generic_info
                .insert(reexport_key.to_string(), gen_info);
        }
        // Clone struct defs if the source is a known struct.
        if let Some(sd) = self.struct_defs.get(source_path).cloned() {
            self.struct_defs.insert(reexport_key.to_string(), sd);
        }
        // Record the chain so path visibility checks can follow it.
        self.reexports
            .insert(reexport_key.to_string(), source_path.to_string());
    }

    /// Resolve a glob re-export: scan all known items whose qualified name
    /// starts with `source_mod::` and register them under `prefix::item_name`.
    fn register_glob_reexports(&mut self, prefix: &str, source_mod: &str) {
        let source_prefix = format!("{source_mod}::");
        // Collect keys first to avoid borrow issues.
        let fn_keys: Vec<String> = self
            .fn_return_types
            .keys()
            .filter(|k| k.starts_with(&source_prefix))
            .cloned()
            .collect();
        for key in &fn_keys {
            if let Some(item_name) = key.strip_prefix(&source_prefix) {
                if item_name.contains("::") {
                    continue;
                }
                let reexport_key = format!("{prefix}::{item_name}");
                self.register_reexport(&reexport_key, key);
            }
        }
        let struct_keys: Vec<String> = self
            .struct_defs
            .keys()
            .filter(|k| k.starts_with(&source_prefix))
            .cloned()
            .collect();
        for key in &struct_keys {
            if let Some(item_name) = key.strip_prefix(&source_prefix) {
                if item_name.contains("::") {
                    continue;
                }
                let reexport_key = format!("{prefix}::{item_name}");
                if let Some(sd) = self.struct_defs.get(key).cloned() {
                    self.struct_defs.insert(reexport_key.clone(), sd);
                }
                self.reexports.insert(reexport_key, key.clone());
            }
        }
    }
}

#[derive(Default)]
struct MethodEffects {
    writes_self: bool,
    self_calls: std::collections::HashSet<String>,
}

fn compute_impl_mutating_methods(methods: &[FnDef]) -> std::collections::HashSet<String> {
    let mut effects_by_name: HashMap<String, MethodEffects> = HashMap::new();
    for method in methods {
        effects_by_name.insert(method.name.clone(), analyze_method_effects(method));
    }
    let mut mutating: std::collections::HashSet<String> = effects_by_name
        .iter()
        .filter_map(|(name, effects)| effects.writes_self.then_some(name.clone()))
        .collect();
    loop {
        let mut changed = false;
        for (name, effects) in &effects_by_name {
            if mutating.contains(name) {
                continue;
            }
            if effects
                .self_calls
                .iter()
                .any(|callee| mutating.contains(callee))
            {
                mutating.insert(name.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    mutating
}

fn analyze_method_effects(method: &FnDef) -> MethodEffects {
    let mut effects = MethodEffects::default();
    if !matches!(method.params.first(), Some(param) if param.name == "self") {
        return effects;
    }
    for stmt in &method.body.stmts {
        stmt_collect_effects(stmt, &mut effects);
    }
    effects
}

fn stmt_collect_effects(stmt: &Stmt, effects: &mut MethodEffects) {
    match stmt {
        Stmt::Let { value, .. } => {
            if let Some(value) = value {
                expr_collect_effects(value, effects);
            }
        }
        Stmt::Expr { expr, .. } => expr_collect_effects(expr, effects),
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                expr_collect_effects(value, effects);
            }
        }
        Stmt::While {
            condition, body, ..
        } => {
            expr_collect_effects(condition, effects);
            for stmt in &body.stmts {
                stmt_collect_effects(stmt, effects);
            }
        }
        Stmt::Loop { body, .. } => {
            for stmt in &body.stmts {
                stmt_collect_effects(stmt, effects);
            }
        }
        Stmt::For { iterable, body, .. } => {
            expr_collect_effects(iterable, effects);
            for stmt in &body.stmts {
                stmt_collect_effects(stmt, effects);
            }
        }
        Stmt::Break { value, .. } => {
            if let Some(value) = value {
                expr_collect_effects(value, effects);
            }
        }
        Stmt::WhileLet { expr, body, .. } => {
            expr_collect_effects(expr, effects);
            for stmt in &body.stmts {
                stmt_collect_effects(stmt, effects);
            }
        }
        Stmt::ForDestructure { iterable, body, .. } => {
            expr_collect_effects(iterable, effects);
            for stmt in &body.stmts {
                stmt_collect_effects(stmt, effects);
            }
        }
        Stmt::LetPattern { value, .. } => expr_collect_effects(value, effects),
        Stmt::Continue { .. } | Stmt::Use(_) | Stmt::Item(_) => {}
    }
}

fn expr_collect_effects(expr: &Expr, effects: &mut MethodEffects) {
    match expr {
        Expr::Assign { target, value, .. } | Expr::CompoundAssign { target, value, .. } => {
            if target_roots_self(target) {
                effects.writes_self = true;
            }
            expr_collect_effects(target, effects);
            expr_collect_effects(value, effects);
        }
        Expr::MethodCall {
            object,
            method,
            args,
            ..
        } => {
            if expr_roots_self(object) {
                effects.self_calls.insert(method.clone());
                if is_builtin_mutator_method(method) {
                    effects.writes_self = true;
                }
            }
            expr_collect_effects(object, effects);
            for arg in args {
                expr_collect_effects(arg, effects);
            }
        }
        Expr::Call { callee, args, .. } => {
            expr_collect_effects(callee, effects);
            for arg in args {
                expr_collect_effects(arg, effects);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            expr_collect_effects(left, effects);
            expr_collect_effects(right, effects);
        }
        Expr::UnaryOp { expr, .. } => expr_collect_effects(expr, effects),
        Expr::Block(block) => {
            for stmt in &block.stmts {
                stmt_collect_effects(stmt, effects);
            }
        }
        Expr::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            expr_collect_effects(condition, effects);
            for stmt in &then_block.stmts {
                stmt_collect_effects(stmt, effects);
            }
            if let Some(else_block) = else_block {
                expr_collect_effects(else_block, effects);
            }
        }
        Expr::Match { expr, arms, .. } => {
            expr_collect_effects(expr, effects);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    expr_collect_effects(guard, effects);
                }
                expr_collect_effects(&arm.body, effects);
            }
        }
        Expr::Range { start, end, .. } => {
            if let Some(start) = start {
                expr_collect_effects(start, effects);
            }
            if let Some(end) = end {
                expr_collect_effects(end, effects);
            }
        }
        Expr::Repeat { value, count, .. } => {
            expr_collect_effects(value, effects);
            expr_collect_effects(count, effects);
        }
        Expr::Array { elements, .. } | Expr::Tuple { elements, .. } => {
            for element in elements {
                expr_collect_effects(element, effects);
            }
        }
        Expr::Index { object, index, .. } => {
            expr_collect_effects(object, effects);
            expr_collect_effects(index, effects);
        }
        Expr::FieldAccess { object, .. }
        | Expr::Grouped(object, _)
        | Expr::Try { expr: object, .. }
        | Expr::Await { expr: object, .. } => expr_collect_effects(object, effects),
        Expr::StructInit { fields, base, .. } => {
            for (_, value) in fields {
                expr_collect_effects(value, effects);
            }
            if let Some(base) = base {
                expr_collect_effects(base, effects);
            }
        }
        Expr::PathCall { args, .. } => {
            for arg in args {
                expr_collect_effects(arg, effects);
            }
        }
        Expr::IfLet {
            expr,
            guard,
            then_block,
            else_block,
            ..
        } => {
            expr_collect_effects(expr, effects);
            if let Some(guard) = guard {
                expr_collect_effects(guard, effects);
            }
            for stmt in &then_block.stmts {
                stmt_collect_effects(stmt, effects);
            }
            if let Some(else_block) = else_block {
                expr_collect_effects(else_block, effects);
            }
        }
        Expr::As { expr, .. } => expr_collect_effects(expr, effects),
        Expr::Closure { body, .. } => expr_collect_effects(body, effects),
        Expr::AsyncBlock { body, .. } => {
            for stmt in &body.stmts {
                stmt_collect_effects(stmt, effects);
            }
        }
        Expr::FString { parts, .. } => {
            for part in parts {
                if let FStringPart::Expr(expr) = part {
                    expr_collect_effects(expr, effects);
                }
            }
        }
        Expr::Return { value, .. } => {
            if let Some(value) = value {
                expr_collect_effects(value, effects);
            }
        }
        Expr::IntLiteral(..)
        | Expr::FloatLiteral(..)
        | Expr::BoolLiteral(..)
        | Expr::StringLiteral(..)
        | Expr::CharLiteral(..)
        | Expr::Ident(..)
        | Expr::Path { .. }
        | Expr::SelfRef(..) => {}
    }
}

fn target_roots_self(expr: &Expr) -> bool {
    match expr {
        Expr::SelfRef(..) => true,
        Expr::FieldAccess { object, .. } | Expr::Index { object, .. } => target_roots_self(object),
        Expr::Grouped(inner, ..) => target_roots_self(inner),
        _ => false,
    }
}

fn expr_roots_self(expr: &Expr) -> bool {
    match expr {
        Expr::SelfRef(..) => true,
        Expr::FieldAccess { object, .. } | Expr::Index { object, .. } => expr_roots_self(object),
        Expr::Grouped(inner, ..) => expr_roots_self(inner),
        _ => false,
    }
}

fn is_builtin_mutator_method(method: &str) -> bool {
    matches!(
        method,
        "push"
            | "push_front"
            | "push_back"
            | "pop"
            | "pop_front"
            | "pop_back"
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
