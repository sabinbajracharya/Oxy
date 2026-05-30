//! Program / module / function lowering for `IrGen` — part of the AST → register IR
//! lowering pass. See `mod.rs` for the `IrGen` struct and state.

use super::*;

impl IrGen {
    /// Generate IR for an entire program.
    pub fn gen_program(&mut self, program: &Program) {
        // Pre-pass: register generic function templates so call sites with
        // turbofish can monomorphize regardless of definition order.
        self.register_generic_fns(&program.items, "");
        for item in &program.items {
            match item {
                Item::Function(f) => self.gen_fn(f, None),
                Item::Impl(imp) => {
                    // Dispatch key is the base type name (generics stripped):
                    // runtime resolves methods by a value's base struct name.
                    let prefix = imp.base_type_name().to_string();
                    for method in &imp.methods {
                        self.gen_method(method, &prefix);
                    }
                }
                Item::Trait(t) => {
                    self.trait_defs.insert(t.name.clone(), t.clone());
                }
                Item::ImplTrait(imp) => {
                    let prefix = imp.base_type_name().to_string();
                    // Compile provided methods
                    for method in &imp.methods {
                        self.gen_method(method, &prefix);
                    }
                    // Compile trait default methods that weren't overridden
                    let default_fns: Vec<crate::ast::FnDef> = self
                        .trait_defs
                        .get(&imp.trait_name)
                        .map(|td| {
                            let provided: std::collections::HashSet<&str> =
                                imp.methods.iter().map(|m| m.name.as_str()).collect();
                            td.default_methods
                                .iter()
                                .filter(|df| !provided.contains(df.name.as_str()))
                                .cloned()
                                .collect()
                        })
                        .unwrap_or_default();
                    for default_fn in &default_fns {
                        self.gen_method(default_fn, &prefix);
                    }
                }
                Item::Const { name, value, .. } => {
                    self.global_consts.insert(name.clone(), value.clone());
                }
                Item::Module(m) => {
                    if let Some(ref items) = m.body {
                        self.gen_module_items(items, &m.name);
                    }
                }
                Item::Use(use_def) => self.register_use(use_def),
                Item::Enum(e) => self.register_enum(e),
                Item::Struct(s) => match &s.kind {
                    crate::ast::StructKind::Tuple(types) => {
                        self.tuple_structs.insert(s.name.clone(), types.len());
                    }
                    crate::ast::StructKind::Unit => {
                        self.unit_structs.insert(s.name.clone());
                    }
                    crate::ast::StructKind::Named(_) => {}
                },
                // Other type items don't generate IR directly
                _ => {}
            }
        }
    }

    /// Recursively compile items inside a module with the given qualified prefix.
    pub(super) fn gen_module_items(&mut self, items: &[Item], prefix: &str) {
        let saved_prefix = self.current_module_prefix.clone();
        self.current_module_prefix = prefix.to_string();
        for item in items {
            match item {
                Item::Function(f) => self.gen_fn(f, Some(prefix)),
                Item::Enum(e) => {
                    // Register enum with its fully-qualified module prefix.
                    let full_name = format!("{prefix}::{}", e.name);
                    for variant in &e.variants {
                        self.variant_to_enum
                            .insert(variant.name.clone(), full_name.clone());
                    }
                }
                Item::Module(m) => {
                    let nested = format!("{prefix}::{}", m.name);
                    if let Some(ref items) = m.body {
                        self.gen_module_items(items, &nested);
                    }
                }
                Item::Impl(imp) => {
                    let qualified_type = format!("{prefix}::{}", imp.base_type_name());
                    for method in &imp.methods {
                        self.gen_method(method, &qualified_type);
                    }
                }
                Item::Trait(t) => {
                    let full_name = format!("{prefix}::{}", t.name);
                    self.trait_defs.insert(full_name, t.clone());
                }
                Item::ImplTrait(imp) => {
                    let qualified_type = format!("{prefix}::{}", imp.base_type_name());
                    for method in &imp.methods {
                        self.gen_method(method, &qualified_type);
                    }
                    // Compile trait default methods that weren't overridden.
                    let full_trait_name = format!("{prefix}::{}", imp.trait_name);
                    let default_fns: Vec<crate::ast::FnDef> = self
                        .trait_defs
                        .get(&imp.trait_name)
                        .or_else(|| self.trait_defs.get(&full_trait_name))
                        .map(|trait_def| {
                            let provided: std::collections::HashSet<&str> =
                                imp.methods.iter().map(|m| m.name.as_str()).collect();
                            trait_def
                                .default_methods
                                .iter()
                                .filter(|df| !provided.contains(df.name.as_str()))
                                .cloned()
                                .collect()
                        })
                        .unwrap_or_default();
                    for default_fn in &default_fns {
                        self.gen_method(default_fn, &qualified_type);
                    }
                }
                Item::Use(use_def) => self.register_use(use_def),
                Item::Const { name, value, .. } => {
                    self.global_consts.insert(name.clone(), value.clone());
                }
                Item::Struct(s) => {
                    // Qualified key matches the name produced by
                    // resolve_module_path at in-module reference sites.
                    let full_name = format!("{prefix}::{}", s.name);
                    match &s.kind {
                        crate::ast::StructKind::Tuple(types) => {
                            self.tuple_structs.insert(full_name, types.len());
                        }
                        crate::ast::StructKind::Unit => {
                            self.unit_structs.insert(full_name);
                        }
                        crate::ast::StructKind::Named(_) => {}
                    }
                }
                _ => {}
            }
        }
        self.current_module_prefix = saved_prefix;
    }

    /// Register a `use` declaration's aliases, glob imports, and — for a
    /// `pub use` inside a module — re-exports. Path prefixes (`self` / `super`
    /// / `crate`) are resolved against the current module context, so this must
    /// run with `current_module_prefix` set to the module containing the `use`.
    /// Shared by the three sites that handle `use` (top-level items, module
    /// items, and in-function statements) so glob handling and prefix
    /// resolution stay identical across all of them.
    pub(super) fn register_use(&mut self, use_def: &crate::ast::UseDef) {
        let base = self.resolve_use_path(&use_def.path).join("::");
        let prefix = self.current_module_prefix.clone();
        // A `pub use` inside a module re-exports the item at `prefix::local`.
        let reexport =
            matches!(use_def.visibility, crate::ast::Visibility::Pub) && !prefix.is_empty();
        match &use_def.tree {
            crate::ast::UseTree::Simple(alias) => {
                let local = alias
                    .clone()
                    .unwrap_or_else(|| use_def.path.last().cloned().unwrap_or_default());
                self.use_aliases.insert(local.clone(), base.clone());
                if reexport {
                    self.fn_aliases.insert(format!("{prefix}::{local}"), base);
                }
            }
            crate::ast::UseTree::Group(items) => {
                for (name, alias) in items {
                    let local = alias.as_ref().unwrap_or(name);
                    let qualified = format!("{base}::{name}");
                    self.use_aliases.insert(local.clone(), qualified.clone());
                    if reexport {
                        self.fn_aliases
                            .insert(format!("{prefix}::{local}"), qualified);
                    }
                }
            }
            crate::ast::UseTree::Glob => {
                self.glob_mods.push(base.clone());
                if reexport {
                    self.register_glob_fn_aliases(&prefix, &base);
                }
            }
        }
    }

    /// Register glob re-exports: for every function whose name starts with
    /// `source_mod::`, register it under `prefix::item_name`. Iterates
    /// `fn_names` (fully populated by the pre-pass) rather than the
    /// incrementally-built `functions`, so re-exports resolve regardless of
    /// whether the source module is defined before or after the `pub use`.
    pub(super) fn register_glob_fn_aliases(&mut self, prefix: &str, source_mod: &str) {
        let source_prefix = format!("{source_mod}::");
        let known_names: Vec<String> = self
            .fn_names
            .iter()
            .filter(|n| n.starts_with(&source_prefix))
            .cloned()
            .collect();
        for full_name in &known_names {
            if let Some(item_name) = full_name.strip_prefix(&source_prefix) {
                if item_name.contains("::") {
                    continue;
                }
                let reexport_key = format!("{prefix}::{item_name}");
                self.fn_aliases.insert(reexport_key, full_name.clone());
            }
        }
    }

    /// Generate IR for one function.
    /// Walk items (recursing into modules) and record every generic free /
    /// module function under its qualified name, paired with the module prefix
    /// active where it's defined. Impl methods are not monomorphized here.
    pub(super) fn register_generic_fns(&mut self, items: &[Item], prefix: &str) {
        for item in items {
            match item {
                Item::Function(f) => {
                    let qualified = if prefix.is_empty() {
                        f.name.clone()
                    } else {
                        format!("{prefix}::{}", f.name)
                    };
                    self.fn_names.insert(qualified.clone());
                    if !f.generic_params.is_empty() {
                        self.generic_fns
                            .insert(qualified, (f.clone(), prefix.to_string()));
                    }
                }
                Item::Module(m) => {
                    if let Some(ref body) = m.body {
                        let nested = if prefix.is_empty() {
                            m.name.clone()
                        } else {
                            format!("{prefix}::{}", m.name)
                        };
                        self.register_generic_fns(body, &nested);
                    }
                }
                Item::TypeAlias { name, target, .. } => {
                    // Only simple named targets can be used as a path prefix
                    // (`Alias::Variant`). Compound targets (arrays, generics)
                    // have no associated items, so they're irrelevant here.
                    if let TypeAnnotation::Named {
                        name: target_name, ..
                    } = target
                    {
                        let alias_q = if prefix.is_empty() {
                            name.clone()
                        } else {
                            format!("{prefix}::{name}")
                        };
                        // A bare target name resolves within the same module;
                        // an already-qualified one is taken as written.
                        let target_q = if prefix.is_empty() || target_name.contains("::") {
                            target_name.clone()
                        } else {
                            format!("{prefix}::{target_name}")
                        };
                        self.type_aliases.insert(alias_q, target_q);
                    }
                }
                _ => {}
            }
        }
    }

    /// If `fname` names a generic function and `turbofish` supplies concrete
    /// types for all its parameters, emit a monomorphized copy (once) and return
    /// its mangled name. Otherwise return `fname` unchanged so the caller falls
    /// back to the generic template (used by argument-inferred calls).
    pub(super) fn monomorphize_if_generic(
        &mut self,
        fname: &str,
        turbofish: &[TypeAnnotation],
    ) -> String {
        let Some((fdef, mod_prefix)) = self.generic_fns.get(fname).cloned() else {
            return fname.to_string();
        };
        if turbofish.len() != fdef.generic_params.len() || turbofish.is_empty() {
            return fname.to_string();
        }
        let concretes: Vec<String> = turbofish.iter().map(|t| t.name().to_string()).collect();
        let mono_name = format!("{fname}${}", concretes.join("$"));
        if self.mono_emitted.insert(mono_name.clone()) {
            let subst: std::collections::HashMap<String, String> = fdef
                .generic_params
                .iter()
                .map(|gp| gp.name.clone())
                .zip(concretes.iter().cloned())
                .collect();
            self.gen_fn_named(&fdef, mono_name.clone(), mod_prefix, subst);
        }
        mono_name
    }

    pub(super) fn gen_fn(&mut self, f: &FnDef, struct_prefix: Option<&str>) {
        let name = match struct_prefix {
            Some(prefix) => format!("{prefix}::{}", f.name),
            None => f.name.clone(),
        };
        self.gen_fn_named(
            f,
            name,
            self.current_module_prefix.clone(),
            Default::default(),
        );
    }

    /// Lower an impl-block method. Identical to `gen_fn` except it records the
    /// impl's type name as `current_self_type` for the duration of the body, so
    /// `Self` literals/paths inside resolve to the concrete struct. `type_prefix`
    /// is the same value used to qualify the method name (the base type name, or
    /// its module-qualified form), so the value's struct tag matches what method
    /// dispatch later looks up.
    pub(super) fn gen_method(&mut self, f: &FnDef, type_prefix: &str) {
        let saved = self.current_self_type.replace(type_prefix.to_string());
        self.gen_fn(f, Some(type_prefix));
        self.current_self_type = saved;
    }

    /// Lower a function body under an explicit final name, module prefix, and
    /// (possibly empty) type-parameter substitution. The substitution is what
    /// makes monomorphization work: `T::zero()` resolves to `int::zero()` while
    /// it's active. Reentrant — saves and restores all per-function state.
    pub(super) fn gen_fn_named(
        &mut self,
        f: &FnDef,
        name: String,
        module_prefix: String,
        subst: std::collections::HashMap<String, String>,
    ) {
        let ret_ty = f
            .return_type
            .as_ref()
            .map(Self::type_ann_to_type_info)
            .unwrap_or(TypeInfo::Unit);
        // Save current state. fn_index set at push time after body generation.
        let saved = std::mem::replace(&mut self.current, IrFunction::new(name, 0, 0, usize::MAX));
        self.current.return_type = ret_ty;
        self.current.is_async = f.is_async;
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_local_types = std::mem::take(&mut self.local_types);
        let saved_mut_slots = std::mem::take(&mut self.mut_slots);
        let saved_celled_slots = std::mem::take(&mut self.celled_slots);
        let saved_subst = std::mem::replace(&mut self.type_subst, subst);
        let saved_module_prefix = std::mem::replace(&mut self.current_module_prefix, module_prefix);
        let saved_local_count = self.local_count;
        let saved_break = self.break_target;
        let saved_continue = self.continue_target;
        self.local_count = 0;
        self.next_reg = 0;
        self.next_block = 0;
        self.break_target = None;
        self.continue_target = None;

        // Create entry block
        let entry = self.alloc_block();
        self.current.entry = entry;
        self.start_block(entry);

        // Allocate locals for params and record explicit metadata on IrFunction.
        // A `byte` parameter must wrap its incoming value to 0..=255 at entry,
        // mirroring the boundary coercion applied to `byte` return values and
        // typed `let` bindings — otherwise a value like `300` would flow through
        // the body as `int(300)`, silently erasing the declared width. Recording
        // the param type in `local_types` also makes reassignment to a `mut byte`
        // param re-wrap, consistent with `let mut x: byte`.
        for param in &f.params {
            let slot = self.alloc_local(&param.name);
            if let TypeAnnotation::Named { name, .. } = &param.type_ann {
                self.local_types.insert(slot, name.clone());
                if name == "byte" {
                    let loaded = self.alloc_reg();
                    self.emit(IrOp::LoadLocal(loaded, slot));
                    let coerced = self.coerce_reg(loaded, &param.type_ann);
                    self.emit(IrOp::StoreLocal(slot, coerced));
                }
            }
        }
        self.current.params = f
            .params
            .iter()
            .map(|p| (p.name.clone(), Self::type_ann_to_type_info(&p.type_ann)))
            .collect();

        // Generate body
        let result_reg = self.gen_block_stmts(&f.body);
        // If no explicit return, add implicit return of tail expression
        if !matches!(
            self.current.blocks[self.current_block].terminator,
            Terminator::Return(_) | Terminator::Panic(_)
        ) {
            let reg = result_reg.unwrap_or_else(|| {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstUnit(r));
                r
            });
            let ret_ty = self.current.return_type.clone();
            let reg = self.coerce_reg_to_type_info(reg, &ret_ty);
            self.terminate(Terminator::Return(reg));
        }

        self.current.local_count = self.local_count;
        self.current.fn_index = self.functions.len();
        self.functions
            .push(std::mem::replace(&mut self.current, saved));

        // Restore state
        self.locals = saved_locals;
        self.local_types = saved_local_types;
        self.mut_slots = saved_mut_slots;
        self.celled_slots = saved_celled_slots;
        self.type_subst = saved_subst;
        self.current_module_prefix = saved_module_prefix;
        self.local_count = saved_local_count;
        self.break_target = saved_break;
        self.continue_target = saved_continue;
    }

    // ── Block / Stmt ───────────────────────────────────────────────────
}
