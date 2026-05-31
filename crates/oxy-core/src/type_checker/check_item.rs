use super::*;

impl TypeChecker {
    pub(super) fn check_item(&mut self, item: &Item) -> Result<(), PipelineError> {
        match item {
            Item::Function(f) => self.check_function(f),
            Item::Const {
                name,
                value,
                type_ann,
                span,
                ..
            } => {
                let declared = if let Some(ann) = type_ann {
                    self.resolve_annotation(ann)
                } else {
                    TypeInfo::Unknown
                };
                let inferred = self.infer_expr(value)?;
                if !declared.accepts(&inferred) {
                    return Err(PipelineError::TypeError {
                        message: format!(
                            "type mismatch: const `{name}` declared as `{}`, but value has type `{}`",
                            declared.name(), inferred.name()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
                Ok(())
            }
            Item::Module(m) => {
                self.module_stack.push(m.name.clone());
                if let Some(body) = &m.body {
                    for item in body {
                        self.check_item(item)?;
                    }
                }
                self.module_stack.pop();
                Ok(())
            }
            Item::Use(use_def) => self.process_use_def(use_def),
            Item::Impl(i) => {
                let qualified_type = if self.module_stack.is_empty() {
                    i.type_name.clone()
                } else {
                    format!("{}::{}", self.module_stack.join("::"), i.type_name)
                };
                let resolved = self.resolve_struct_name(&qualified_type);
                let saved_impl = self.current_impl_type.clone();
                self.current_impl_type = Some(resolved);
                for method in &i.methods {
                    self.check_function(method)?;
                }
                self.current_impl_type = saved_impl;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub(super) fn check_function(&mut self, f: &FnDef) -> Result<(), PipelineError> {
        // Track the function's own generic params, plus any inherited from
        // an enclosing impl block, while we walk its body.
        let impl_generics = self
            .current_impl_type
            .as_deref()
            .map(|t| self.struct_generic_names(t))
            .unwrap_or_default();
        let saved_generics = self.current_generics.clone();
        for p in &f.generic_params {
            self.current_generics.push(p.name.clone());
        }
        for g in &impl_generics {
            self.current_generics.push(g.clone());
        }

        let ret_ty = if let Some(ref ann) = f.return_type {
            let is_generic = match ann {
                TypeAnnotation::Named { name, .. } => {
                    self.current_generics.iter().any(|g| g == name)
                }
                TypeAnnotation::Array { .. } => false,
            };
            if is_generic {
                TypeInfo::Unknown
            } else {
                let ty = self.resolve_annotation(ann);
                self.validate_type_known(&ty, ann.span())?;
                ty
            }
        } else {
            TypeInfo::Unit
        };
        // async fn returns Future<T> to callers — .await unwraps it.
        // The body itself still returns the declared type, so we store the
        // Future-wrapped type in fn_return_types but keep ret_ty as the raw
        // annotation for body checking and current_fn_return.
        let stored_ret_ty = if f.is_async {
            TypeInfo::Future(Box::new(ret_ty.clone()))
        } else {
            ret_ty.clone()
        };
        // Register the function's type under the same key `collect_fn_types`
        // uses. A free function inside a module must be keyed by its qualified
        // name (`secret::hidden`), NOT its bare name — otherwise a private
        // module function leaks into the global bare-name namespace and a call
        // site resolves it without a visibility check. Methods (current_impl_type
        // set) keep the bare name, matching collect's unqualified registration.
        let fn_key = if self.current_impl_type.is_none() && !self.module_stack.is_empty() {
            format!("{}::{}", self.module_stack.join("::"), f.name)
        } else {
            f.name.clone()
        };
        self.fn_return_types
            .insert(fn_key.clone(), stored_ret_ty.clone());
        let param_tys = self.resolve_param_types(f, &impl_generics);
        // Validate every declared param type for unknown names.
        for (param, p_ty) in f.params.iter().zip(param_tys.iter()) {
            self.validate_type_known(p_ty, param.span)?;
        }
        self.fn_param_types.insert(fn_key, param_tys.clone());

        let fn_env = TypeEnv::child(&self.env);
        for (param, p_ty) in f.params.iter().zip(param_tys.iter()) {
            fn_env
                .borrow_mut()
                .define_mut(&param.name, p_ty.clone(), true);
        }

        let saved_env = self.env.clone();
        self.env = fn_env;
        let saved_fn_return = std::mem::replace(&mut self.current_fn_return, ret_ty.clone());

        let body_result = self.check_stmt_seq(&f.body.stmts, &ret_ty);

        self.env = saved_env;
        self.current_generics = saved_generics;
        self.current_fn_return = saved_fn_return;
        body_result
    }
}
