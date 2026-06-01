//! Type inference for call-shaped expressions: free/qualified calls,
//! method calls, path calls, and macro calls.
//!
//! Part of `check_expr` — see that module for the `infer_expr` dispatcher.

use super::*;

impl TypeChecker {
    pub(super) fn infer_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        span: &Span,
    ) -> Result<TypeInfo, PipelineError> {
        if let Expr::Ident(name, _) = callee {
            // Resolve the callee's qualified name and look up its params.
            let resolved_key = if self.fn_param_types.contains_key(name) {
                Some(name.clone())
            } else if !name.contains("::") {
                // Try use_aliases first (handles `use foo::bar` + `bar()`).
                if let Some(aliased) = self.use_aliases.get(name) {
                    if self.fn_param_types.contains_key(aliased) {
                        Some(aliased.clone())
                    } else {
                        None
                    }
                } else {
                    let module_prefix = self.module_stack.join("::");
                    if !module_prefix.is_empty() {
                        let qualified = format!("{}::{}", module_prefix, name);
                        if self.fn_param_types.contains_key(&qualified) {
                            Some(qualified)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            } else {
                None
            };
            // Fall back to glob imports: a bare `name` may have come from
            // a `use module::*`. Resolve it to `module::name` so the
            // visibility check below rejects a private glob item.
            let resolved_key = resolved_key.or_else(|| {
                if name.contains("::") {
                    return None;
                }
                self.glob_imports.iter().rev().find_map(|m| {
                    let q = format!("{m}::{name}");
                    (self.fn_defs.contains_key(&q) || self.fn_return_types.contains_key(&q))
                        .then_some(q)
                })
            });
            if let Some(key) = resolved_key {
                self.check_path_visible(&key, *span)?;
                let params = self.fn_param_types.get(&key).cloned().unwrap_or_default();
                self.check_args_against_params(&params, args, false, name, Some(&key), *span)?;
                let arg_types: Vec<TypeInfo> = args
                    .iter()
                    .map(|a| self.infer_expr(a))
                    .collect::<Result<_, _>>()?;
                if let Some(ret) =
                    self.resolve_generic_return(&key, &arg_types, false, &HashMap::new())
                {
                    return Ok(ret);
                }
                if let Some(ret) = self.fn_return_types.get(&key) {
                    return Ok(ret.clone());
                }
            } else {
                // Unknown callee — fall back to inferring args without
                // checking against any signature.
                let arg_types: Vec<TypeInfo> = args
                    .iter()
                    .map(|a| self.infer_expr(a))
                    .collect::<Result<_, _>>()?;
                // Built-in constructors: parameterize the wrapper by
                // the inner argument's inferred type.
                match name.as_str() {
                    "Some" => {
                        let inner = arg_types.first().cloned().unwrap_or(TypeInfo::Unknown);
                        return Ok(TypeInfo::Option(Box::new(inner)));
                    }
                    "Ok" => {
                        let inner = arg_types.first().cloned().unwrap_or(TypeInfo::Unknown);
                        return Ok(TypeInfo::Result(
                            Box::new(inner),
                            Box::new(TypeInfo::Unknown),
                        ));
                    }
                    "Err" => {
                        let inner = arg_types.first().cloned().unwrap_or(TypeInfo::Unknown);
                        return Ok(TypeInfo::Result(
                            Box::new(TypeInfo::Unknown),
                            Box::new(inner),
                        ));
                    }
                    "spawn" => {
                        // spawn(|| expr) → JoinHandle<expr_type>. Exactly one
                        // argument, and it must be a closure.
                        if args.len() != 1 {
                            return Err(PipelineError::TypeError {
                                message: format!(
                                    "spawn expects exactly 1 argument (a closure), found {}",
                                    args.len()
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        let inner = if let Expr::Closure { body, .. } = &args[0] {
                            self.infer_expr(body)?
                        } else {
                            return Err(PipelineError::TypeError {
                                message: "spawn expects a closure argument, e.g. \
                                                  spawn(|| expr)"
                                    .to_string(),
                                line: span.line,
                                column: span.column,
                            });
                        };
                        return Ok(TypeInfo::JoinHandle(Box::new(inner)));
                    }
                    "sleep" => {
                        // sleep(duration) → Unit. Exactly one argument.
                        if args.len() != 1 {
                            return Err(PipelineError::TypeError {
                                message: format!(
                                    "sleep expects exactly 1 argument (a duration), \
                                             found {}",
                                    args.len()
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        self.infer_expr(&args[0])?;
                        return Ok(TypeInfo::Unit);
                    }
                    "select" => {
                        // select(h1, h2, ...) → common inner type of all
                        // JoinHandle args, or Unknown if they differ.
                        // Requires at least two handles to choose between.
                        if args.len() < 2 {
                            return Err(PipelineError::TypeError {
                                message: format!(
                                    "select expects at least 2 arguments (handles to \
                                             choose between), found {}",
                                    args.len()
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        let mut inner: Option<TypeInfo> = None;
                        for t in &arg_types {
                            let unwrapped = match t {
                                TypeInfo::JoinHandle(inner_ty) => *inner_ty.clone(),
                                _ => TypeInfo::Unknown,
                            };
                            inner = match inner {
                                None => Some(unwrapped),
                                Some(prev) if prev == unwrapped => Some(prev),
                                _ => Some(TypeInfo::Unknown),
                            };
                        }
                        return Ok(inner.unwrap_or(TypeInfo::Unknown));
                    }
                    // Free functions for pipeline-friendly stdlib.
                    "map" | "filter" => {
                        if args.len() != 2 {
                            return Err(PipelineError::TypeError {
                                message: format!(
                                    "`{}` expects exactly 2 arguments (data, closure), found {}",
                                    name,
                                    args.len()
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        self.infer_expr(&args[0])?;
                        self.infer_expr(&args[1])?;
                        // map returns Vec<U>, filter returns Vec<T>
                        return Ok(TypeInfo::Unknown);
                    }
                    "fold" => {
                        if args.len() != 3 {
                            return Err(PipelineError::TypeError {
                                message: format!(
                                    "`fold` expects exactly 3 arguments \
                                     (data, init, closure), found {}",
                                    args.len()
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        self.infer_expr(&args[0])?;
                        let init_ty = self.infer_expr(&args[1])?;
                        self.infer_expr(&args[2])?;
                        return Ok(init_ty);
                    }
                    "any" | "all" => {
                        if args.len() != 2 {
                            return Err(PipelineError::TypeError {
                                message: format!(
                                    "`{}` expects exactly 2 arguments (data, predicate), found {}",
                                    name,
                                    args.len()
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        self.infer_expr(&args[0])?;
                        self.infer_expr(&args[1])?;
                        return Ok(TypeInfo::Bool);
                    }
                    "find" => {
                        if args.len() != 2 {
                            return Err(PipelineError::TypeError {
                                message: format!(
                                    "`find` expects exactly 2 arguments \
                                     (data, predicate), found {}",
                                    args.len()
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        self.infer_expr(&args[0])?;
                        self.infer_expr(&args[1])?;
                        return Ok(TypeInfo::Option(Box::new(TypeInfo::Unknown)));
                    }
                    "collect" => {
                        if args.len() != 1 {
                            return Err(PipelineError::TypeError {
                                message: format!(
                                    "`collect` expects exactly 1 argument (data), found {}",
                                    args.len()
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        self.infer_expr(&args[0])?;
                        return Ok(TypeInfo::Vec(Box::new(TypeInfo::Unknown)));
                    }
                    "sort" => {
                        if args.len() != 1 {
                            return Err(PipelineError::TypeError {
                                message: format!(
                                    "`sort` expects exactly 1 argument (data), found {}",
                                    args.len()
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        let data_ty = self.infer_expr(&args[0])?;
                        return Ok(data_ty);
                    }
                    "sort_by" => {
                        if args.len() != 2 {
                            return Err(PipelineError::TypeError {
                                message: format!(
                                    "`sort_by` expects exactly 2 arguments \
                                     (data, comparator), found {}",
                                    args.len()
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        let data_ty = self.infer_expr(&args[0])?;
                        self.infer_expr(&args[1])?;
                        return Ok(data_ty);
                    }
                    // Former `!` macro calls — now regular built-in functions.
                    "println" | "print" | "eprintln" => {
                        return Ok(TypeInfo::Unit);
                    }
                    "format" => {
                        return Ok(TypeInfo::String);
                    }
                    "dbg" => {
                        return Ok(arg_types.first().cloned().unwrap_or(TypeInfo::Unknown));
                    }
                    "panic" | "todo" => {
                        return Ok(TypeInfo::Unknown);
                    }
                    "assert_eq" | "assert_ne" | "assert" | "unimplemented" => {
                        // varargs — type-checked at runtime via registry
                        return Ok(TypeInfo::Unit);
                    }
                    "http::fetch" | "http::fetch_post" => {
                        // async HTTP call → Future<HttpResponse>
                        let _ = arg_types; // validate args but don't constrain
                        return Ok(TypeInfo::Future(Box::new(TypeInfo::UserStruct {
                            name: "HttpResponse".to_string(),
                            generic_args: vec![],
                        })));
                    }
                    _ => {}
                }
            }
        } else {
            for arg in args {
                self.infer_expr(arg)?;
            }
        }
        // Fallback: check if callee is a function-typed value (closure, async closure).
        // A bare-ident callee that isn't a local binding may be a builtin
        // or global function resolved at codegen (e.g. `println(..)`); it
        // is not an undefined *variable*, so don't infer it as a value
        // (which would trip the undefined-variable check). Undefined value
        // references in argument position were already checked above.
        if let Expr::Ident(cname, _) = callee {
            if self.env.borrow().get(cname).is_none() {
                return Ok(TypeInfo::Unknown);
            }
        }
        let callee_ty = self.infer_expr(callee)?;
        if let TypeInfo::Function { params, ret } = &callee_ty {
            // Only check arg count when params are known (non-empty).
            // A bare `Fn` type has 0 params and accepts any arity.
            if !params.is_empty() && args.len() != params.len() {
                return Err(PipelineError::TypeError {
                    message: format!("expected {} arguments, found {}", params.len(), args.len()),
                    line: span.line,
                    column: span.column,
                });
            }
            return Ok(*ret.clone());
        }
        Ok(TypeInfo::Unknown)
    }

    pub(super) fn infer_method_call(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
        span: &Span,
    ) -> Result<TypeInfo, PipelineError> {
        let obj_ty = self.infer_expr(object)?;
        if matches!(
            method,
            symbols::generic_m::APPLY | symbols::generic_m::TRY_APPLY
        ) {
            return self.infer_apply_like(&obj_ty, method, args, *span);
        }
        if let TypeInfo::UserStruct {
            name: struct_name,
            generic_args,
        } = &obj_ty
        {
            let resolved = self.resolve_struct_name(struct_name);
            let qualified = format!("{}::{}", resolved, method);
            let module_qualified = if self.module_stack.is_empty() {
                None
            } else {
                Some(format!(
                    "{}::{}::{}",
                    self.module_stack.join("::"),
                    resolved,
                    method
                ))
            };
            let resolved_key = if self.fn_param_types.contains_key(&qualified) {
                Some(qualified.clone())
            } else if let Some(mq) = module_qualified.as_ref() {
                if self.fn_param_types.contains_key(mq) {
                    Some(mq.clone())
                } else {
                    None
                }
            } else {
                None
            };
            // Seed initial generic bindings from the receiver type
            // so that `Cell<int>.replace("wrong")` rejects the call.
            let initial_bindings: HashMap<String, TypeInfo> =
                if let Some(def) = self.struct_defs.get(&resolved) {
                    let param_names: Vec<String> =
                        def.generic_params.iter().map(|p| p.name.clone()).collect();
                    param_names
                        .iter()
                        .zip(generic_args.iter())
                        .map(|(n, t)| (n.clone(), t.clone()))
                        .collect()
                } else {
                    HashMap::new()
                };
            if let Some(key) = resolved_key {
                let params = self.fn_param_types.get(&key).cloned().unwrap_or_default();
                self.check_args_against_params_with_bindings(
                    &params,
                    args,
                    true,
                    method,
                    Some(&key),
                    &initial_bindings,
                    *span,
                )?;
                let arg_types: Vec<TypeInfo> = args
                    .iter()
                    .map(|a| self.infer_expr(a))
                    .collect::<Result<_, _>>()?;
                if let Some(ret) =
                    self.resolve_generic_return(&key, &arg_types, true, &initial_bindings)
                {
                    return Ok(ret);
                }
                if let Some(ret_ty) = self.fn_return_types.get(&key) {
                    return Ok(ret_ty.clone());
                }
            } else {
                // Unknown user-method — infer args for side effects,
                // then fall through to the builtin method table.
                for arg in args {
                    self.infer_expr(arg)?;
                }
            }
        } else {
            // Check for impl-on-primitive (e.g. `impl Doublable for i64`).
            let primitive_qualified = format!("{}::{}", obj_ty.name(), method);
            let prim_key = if self.fn_param_types.contains_key(&primitive_qualified) {
                Some(primitive_qualified)
            } else {
                None
            };
            if let Some(key) = prim_key {
                let params = self.fn_param_types.get(&key).cloned().unwrap_or_default();
                self.check_args_against_params(&params, args, true, method, Some(&key), *span)?;
                if let Some(ret_ty) = self.fn_return_types.get(&key) {
                    return Ok(ret_ty.clone());
                }
            } else {
                let arg_types: Vec<TypeInfo> = args
                    .iter()
                    .map(|a| self.infer_expr(a))
                    .collect::<Result<_, _>>()?;
                // Validate the method against the builtin method tables.
                // Skip when the receiver type is Unknown (we have no
                // signature to compare against) or a UserStruct (handled
                // above; impl methods may not be in symbols).
                if obj_ty != TypeInfo::Unknown
                    && !matches!(obj_ty, TypeInfo::UserStruct { .. })
                    && !self.method_exists_on(&obj_ty, method)
                {
                    return Err(PipelineError::TypeError {
                        message: format!("no method `{method}` on type `{}`", obj_ty.name()),
                        line: span.line,
                        column: span.column,
                    });
                }
                // Fixed-size arrays disallow Vec mutators.
                if matches!(obj_ty, TypeInfo::Array(..)) && self.is_array_mutator(method) {
                    return Err(PipelineError::TypeError {
                                message: format!(
                                    "method `{method}` is not available on fixed-size arrays; convert to `List` first"
                                ),
                                line: span.line,
                                column: span.column,
                            });
                }
                // Per-method element-type checks for parameterized
                // containers (`Vec.push(T)`, `HashMap.insert(K, V)`,
                // ...). Returns the method's parameterized return type
                // when known.
                if let Some(ret) =
                    self.check_builtin_method_args(&obj_ty, method, args, &arg_types, *span)?
                {
                    return Ok(ret);
                }
            }
        }
        // Common built-in method return types. Keeps downstream
        // type-checking honest when calls are chained through builtins
        // like `.to_string()`. Anything not listed stays Unknown.
        Ok(match method {
            "to_string" => TypeInfo::String,
            "len" => TypeInfo::I64,
            "is_empty" | "contains" | "starts_with" | "ends_with" => TypeInfo::Bool,
            "find" => TypeInfo::Option(Box::new(TypeInfo::I64)),
            "clone" => obj_ty.clone(),
            _ => TypeInfo::Unknown,
        })
    }

    pub(super) fn infer_path_call(
        &mut self,
        path: &[String],
        args: &[Expr],
        span: &Span,
    ) -> Result<TypeInfo, PipelineError> {
        let qualified = path.join("::");
        // Resolve key, mirroring the lookup order used for fn_return_types.
        let resolved_key = if self.fn_param_types.contains_key(&qualified) {
            Some(qualified.clone())
        } else if path.len() == 2 {
            self.use_aliases.get(&path[0]).and_then(|prefix| {
                let aliased = format!("{}::{}", prefix, &path[1]);
                if self.fn_param_types.contains_key(&aliased) {
                    Some(aliased)
                } else {
                    None
                }
            })
        } else {
            None
        }
        .or_else(|| {
            let module_prefix = self.module_stack.join("::");
            if module_prefix.is_empty() {
                None
            } else {
                let module_qualified = format!("{}::{}", module_prefix, qualified);
                if self.fn_param_types.contains_key(&module_qualified) {
                    Some(module_qualified)
                } else {
                    None
                }
            }
        });
        if let Some(key) = resolved_key {
            self.check_path_visible(&key, *span)?;
            let params = self.fn_param_types.get(&key).cloned().unwrap_or_default();
            self.check_args_against_params(&params, args, false, &qualified, Some(&key), *span)?;
            // Substitute generic return type with concrete arg types.
            let arg_types: Vec<TypeInfo> = args
                .iter()
                .map(|a| self.infer_expr(a))
                .collect::<Result<_, _>>()?;
            if let Some(ret) = self.resolve_generic_return(&key, &arg_types, false, &HashMap::new())
            {
                return Ok(ret);
            }
            if let Some(ret) = self.fn_return_types.get(&key) {
                return Ok(ret.clone());
            }
        } else {
            // Built-in path calls with known return types.
            let name = qualified.as_str();
            if name == "http::fetch" || name == "http::fetch_post" {
                for arg in args {
                    self.infer_expr(arg)?;
                }
                return Ok(TypeInfo::Future(Box::new(TypeInfo::UserStruct {
                    name: "HttpResponse".to_string(),
                    generic_args: vec![],
                })));
            }
            for arg in args {
                self.infer_expr(arg)?;
            }
        }
        Ok(TypeInfo::Unknown)
    }

    fn infer_apply_like(
        &mut self,
        obj_ty: &TypeInfo,
        method: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<TypeInfo, PipelineError> {
        if args.len() != 1 {
            return Err(PipelineError::TypeError {
                message: format!(
                    "`{method}` expects exactly 1 closure argument, found {}",
                    args.len()
                ),
                line: span.line,
                column: span.column,
            });
        }

        let closure_expr = &args[0];
        if !matches!(closure_expr, Expr::Closure { .. }) {
            return Err(PipelineError::TypeError {
                message: format!("`{method}` expects a closure argument"),
                line: closure_expr.span().line,
                column: closure_expr.span().column,
            });
        }

        if method == symbols::generic_m::APPLY && Self::apply_closure_contains_try(closure_expr) {
            return Err(PipelineError::TypeError {
                message: "mismatched types. Expected closure to return `()`, found `Result<_, _>`. Consider using `try_apply` instead.".to_string(),
                line: closure_expr.span().line,
                column: closure_expr.span().column,
            });
        }

        let expected_closure_ty = if method == symbols::generic_m::APPLY {
            TypeInfo::Function {
                params: vec![obj_ty.clone()],
                ret: Box::new(TypeInfo::Unit),
            }
        } else {
            TypeInfo::Function {
                params: vec![obj_ty.clone()],
                ret: Box::new(TypeInfo::Result(
                    Box::new(TypeInfo::Unit),
                    Box::new(TypeInfo::Unknown),
                )),
            }
        };

        let closure_ty = self.infer_expr_expected(closure_expr, Some(&expected_closure_ty))?;
        let TypeInfo::Function { params, ret } = closure_ty else {
            return Err(PipelineError::TypeError {
                message: format!("`{method}` expects a closure argument"),
                line: closure_expr.span().line,
                column: closure_expr.span().column,
            });
        };

        if params.len() != 1 {
            return Err(PipelineError::TypeError {
                message: format!(
                    "`{method}` closure must take exactly 1 parameter, found {}",
                    params.len()
                ),
                line: closure_expr.span().line,
                column: closure_expr.span().column,
            });
        }

        if method == symbols::generic_m::APPLY {
            if *ret != TypeInfo::Unit {
                return Err(PipelineError::TypeError {
                    message: format!(
                        "mismatched types. Expected closure to return `()`, found `{}`. Consider using `try_apply` instead.",
                        ret.display_name()
                    ),
                    line: closure_expr.span().line,
                    column: closure_expr.span().column,
                });
            }
            return Ok(obj_ty.clone());
        }

        let TypeInfo::Result(ok_ty, err_ty) = ret.as_ref() else {
            return Err(PipelineError::TypeError {
                message: format!(
                    "mismatched types. Expected closure to return `Result<(), E>`, found `{}`",
                    ret.display_name()
                ),
                line: closure_expr.span().line,
                column: closure_expr.span().column,
            });
        };

        if **ok_ty != TypeInfo::Unit && **ok_ty != TypeInfo::Unknown {
            return Err(PipelineError::TypeError {
                message: format!(
                    "mismatched types. Expected closure to return `Result<(), E>`, found `Result<{}, _>`",
                    ok_ty.display_name()
                ),
                line: closure_expr.span().line,
                column: closure_expr.span().column,
            });
        }

        Ok(TypeInfo::Result(Box::new(obj_ty.clone()), err_ty.clone()))
    }

    fn apply_closure_contains_try(expr: &Expr) -> bool {
        match expr {
            Expr::Try { .. } => true,
            Expr::UnaryOp { expr, .. }
            | Expr::Grouped(expr, ..)
            | Expr::Await { expr, .. }
            | Expr::As { expr, .. } => Self::apply_closure_contains_try(expr),
            Expr::BinaryOp { left, right, .. } => {
                Self::apply_closure_contains_try(left) || Self::apply_closure_contains_try(right)
            }
            Expr::Call { callee, args, .. } => {
                Self::apply_closure_contains_try(callee)
                    || args.iter().any(Self::apply_closure_contains_try)
            }
            Expr::MethodCall { object, args, .. } => {
                Self::apply_closure_contains_try(object)
                    || args.iter().any(Self::apply_closure_contains_try)
            }
            Expr::PathCall { args, .. } | Expr::Tuple { elements: args, .. } => {
                args.iter().any(Self::apply_closure_contains_try)
            }
            Expr::Array { elements, .. } => elements.iter().any(Self::apply_closure_contains_try),
            Expr::Repeat { value, count, .. } => {
                Self::apply_closure_contains_try(value) || Self::apply_closure_contains_try(count)
            }
            Expr::Index { object, index, .. } => {
                Self::apply_closure_contains_try(object) || Self::apply_closure_contains_try(index)
            }
            Expr::Assign { target, value, .. } | Expr::CompoundAssign { target, value, .. } => {
                Self::apply_closure_contains_try(target) || Self::apply_closure_contains_try(value)
            }
            Expr::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                Self::apply_closure_contains_try(condition)
                    || then_block
                        .stmts
                        .iter()
                        .any(Self::apply_closure_stmt_contains_try)
                    || else_block
                        .as_deref()
                        .is_some_and(Self::apply_closure_contains_try)
            }
            Expr::IfLet {
                expr,
                guard,
                then_block,
                else_block,
                ..
            } => {
                Self::apply_closure_contains_try(expr)
                    || guard
                        .as_deref()
                        .is_some_and(Self::apply_closure_contains_try)
                    || then_block
                        .stmts
                        .iter()
                        .any(Self::apply_closure_stmt_contains_try)
                    || else_block
                        .as_deref()
                        .is_some_and(Self::apply_closure_contains_try)
            }
            Expr::Match { expr, arms, .. } => {
                Self::apply_closure_contains_try(expr)
                    || arms.iter().any(|a| {
                        a.guard
                            .as_deref()
                            .is_some_and(Self::apply_closure_contains_try)
                            || Self::apply_closure_contains_try(&a.body)
                    })
            }
            Expr::StructInit { fields, base, .. } => {
                fields
                    .iter()
                    .any(|(_, e)| Self::apply_closure_contains_try(e))
                    || base
                        .as_deref()
                        .is_some_and(Self::apply_closure_contains_try)
            }
            Expr::Block(block) | Expr::AsyncBlock { body: block, .. } => block
                .stmts
                .iter()
                .any(Self::apply_closure_stmt_contains_try),
            Expr::Closure { body, .. } => Self::apply_closure_contains_try(body),
            Expr::Range { start, end, .. } => {
                start
                    .as_deref()
                    .is_some_and(Self::apply_closure_contains_try)
                    || end.as_deref().is_some_and(Self::apply_closure_contains_try)
            }
            Expr::FString { parts, .. } => parts
                .iter()
                .any(|p| matches!(p, FStringPart::Expr(e) if Self::apply_closure_contains_try(e))),
            Expr::Return { value, .. } => value
                .as_deref()
                .is_some_and(Self::apply_closure_contains_try),
            Expr::IntLiteral(..)
            | Expr::FloatLiteral(..)
            | Expr::BoolLiteral(..)
            | Expr::StringLiteral(..)
            | Expr::CharLiteral(..)
            | Expr::Ident(..)
            | Expr::FieldAccess { .. }
            | Expr::Path { .. }
            | Expr::SelfRef(..) => false,
        }
    }

    fn apply_closure_stmt_contains_try(stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Expr { expr, .. } => Self::apply_closure_contains_try(expr),
            Stmt::Let { value, .. } => value.as_ref().is_some_and(Self::apply_closure_contains_try),
            Stmt::LetPattern { value, .. } => Self::apply_closure_contains_try(value),
            Stmt::Return { value, .. } => {
                value.as_ref().is_some_and(Self::apply_closure_contains_try)
            }
            Stmt::While {
                condition, body, ..
            } => {
                Self::apply_closure_contains_try(condition)
                    || body.stmts.iter().any(Self::apply_closure_stmt_contains_try)
            }
            Stmt::WhileLet { expr, body, .. } => {
                Self::apply_closure_contains_try(expr)
                    || body.stmts.iter().any(Self::apply_closure_stmt_contains_try)
            }
            Stmt::For { iterable, body, .. } => {
                Self::apply_closure_contains_try(iterable)
                    || body.stmts.iter().any(Self::apply_closure_stmt_contains_try)
            }
            Stmt::ForDestructure { iterable, body, .. } => {
                Self::apply_closure_contains_try(iterable)
                    || body.stmts.iter().any(Self::apply_closure_stmt_contains_try)
            }
            Stmt::Loop { body, .. } => body.stmts.iter().any(Self::apply_closure_stmt_contains_try),
            Stmt::Item(_) | Stmt::Use(_) | Stmt::Break { .. } | Stmt::Continue { .. } => false,
        }
    }
}
