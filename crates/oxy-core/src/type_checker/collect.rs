use super::*;

impl TypeChecker {
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
                    self.enum_defs.insert(qualified);
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
                    let param_tys = self.resolve_param_types(f, &[]);
                    self.fn_return_types.insert(qualified.clone(), ret_ty);
                    self.fn_param_types.insert(qualified, param_tys);
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
                    let type_prefix = if prefix.is_empty() {
                        i.type_name.clone()
                    } else {
                        format!("{}::{}", prefix, i.type_name)
                    };
                    let impl_generics = self.struct_generic_names(&type_prefix);
                    for method in &i.methods {
                        let qualified = format!("{}::{}", type_prefix, method.name);
                        let unqualified = format!("{}::{}", i.type_name, method.name);
                        let ret_ty = if let Some(ref ann) = method.return_type {
                            self.resolve_annotation(ann)
                        } else {
                            TypeInfo::Unit
                        };
                        let param_tys = self.resolve_param_types(method, &impl_generics);
                        // Also register under unqualified type name (for use-aliased lookups)
                        self.fn_return_types
                            .insert(unqualified.clone(), ret_ty.clone());
                        self.fn_return_types.insert(qualified.clone(), ret_ty);
                        self.fn_param_types.insert(unqualified, param_tys.clone());
                        self.fn_param_types.insert(qualified, param_tys);
                    }
                }
                Item::ImplTrait(i) => {
                    let type_prefix = if prefix.is_empty() {
                        i.type_name.clone()
                    } else {
                        format!("{}::{}", prefix, i.type_name)
                    };
                    let impl_generics = self.struct_generic_names(&type_prefix);
                    for method in &i.methods {
                        let qualified = format!("{}::{}", type_prefix, method.name);
                        let unqualified = format!("{}::{}", i.type_name, method.name);
                        let ret_ty = if let Some(ref ann) = method.return_type {
                            self.resolve_annotation(ann)
                        } else {
                            TypeInfo::Unit
                        };
                        let param_tys = self.resolve_param_types(method, &impl_generics);
                        self.fn_return_types
                            .insert(unqualified.clone(), ret_ty.clone());
                        self.fn_return_types.insert(qualified.clone(), ret_ty);
                        self.fn_param_types.insert(unqualified, param_tys.clone());
                        self.fn_param_types.insert(qualified, param_tys);
                    }
                }
                _ => {}
            }
        }
        self.module_stack = saved_stack;
    }
}
