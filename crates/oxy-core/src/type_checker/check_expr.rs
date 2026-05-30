use super::*;

/// Extract the inner type arguments from a concrete `TypeInfo` for recursing
/// into nested generic containers (e.g. `Vec<int>` → `[int]`).
fn type_args_of(ty: &TypeInfo) -> Vec<TypeInfo> {
    match ty {
        TypeInfo::Vec(t) => vec![t.as_ref().clone()],
        TypeInfo::Option(t) => vec![t.as_ref().clone()],
        TypeInfo::Result(t, e) => vec![t.as_ref().clone(), e.as_ref().clone()],
        TypeInfo::HashMap(k, v) => vec![k.as_ref().clone(), v.as_ref().clone()],
        TypeInfo::UserStruct { generic_args, .. } => generic_args.clone(),
        _ => vec![],
    }
}

/// Return true when `ty` is an unresolved generic-param reference
/// (e.g. `UserStruct { name: "T" }` whose name matches a generic param).
fn is_generic_placeholder(ty: &TypeInfo, generic_names: &[String]) -> bool {
    match ty {
        TypeInfo::UserStruct { name, generic_args } => {
            generic_names.contains(name) && generic_args.is_empty()
        }
        _ => false,
    }
}

/// Recursively walk `ann` (the declared param type) and `arg_ty` (the
/// concrete argument type) and bind each generic-param name to its
/// concrete type.  A conflict (same generic name → different concrete
/// types) returns an error tuple with position and message.
fn bind_generic_params(
    ann: &TypeAnnotation,
    arg_ty: &TypeInfo,
    generic_names: &[String],
    bindings: &mut HashMap<String, TypeInfo>,
    pos: usize,
) -> Result<(), (usize, String)> {
    match ann {
        TypeAnnotation::Named {
            name, generic_args, ..
        } => {
            let arg_inner = type_args_of(arg_ty);
            for (i, ga) in generic_args.iter().enumerate() {
                if i < arg_inner.len() {
                    bind_generic_params(ga, &arg_inner[i], generic_names, bindings, pos)?;
                }
            }
            if generic_names.contains(name) && generic_args.is_empty() {
                if *arg_ty == TypeInfo::Unknown {
                    return Ok(());
                }
                if let Some(existing) = bindings.get(name) {
                    // If the existing binding is an unresolved generic param
                    // (e.g. `Cell<T>` returned T → UserStruct { name: "T" }),
                    // replace it with the concrete type.
                    if is_generic_placeholder(existing, generic_names) {
                        bindings.insert(name.clone(), arg_ty.clone());
                    } else if !existing.accepts(arg_ty) && !arg_ty.accepts(existing) {
                        return Err((
                            pos,
                            format!(
                                "generic parameter `{name}` bound to `{}` is incompatible with `{}`",
                                existing.name(),
                                arg_ty.name()
                            ),
                        ));
                    }
                } else {
                    bindings.insert(name.clone(), arg_ty.clone());
                }
            }
        }
        _ => {}
    }
    Ok(())
}

impl TypeChecker {
    /// Build generic bindings from argument types and substitute into the
    /// return-type annotation. Returns the concrete return type for a call.
    fn resolve_generic_return(
        &self,
        fn_key: &str,
        arg_types: &[TypeInfo],
        skip_self: bool,
        initial_bindings: &HashMap<String, TypeInfo>,
    ) -> Option<TypeInfo> {
        let (generic_names, param_anns, ret_ann) = self.fn_generic_info.get(fn_key)?;
        let effective_anns: &[TypeAnnotation] = if skip_self && !param_anns.is_empty() {
            &param_anns[1..]
        } else {
            param_anns
        };
        let mut bindings = initial_bindings.clone();
        for (ann, arg_ty) in effective_anns.iter().zip(arg_types.iter()) {
            // Ignore errors — this is a best-effort substitution.
            let _ = bind_generic_params(ann, arg_ty, generic_names, &mut bindings, 0);
        }
        // Remove unresolved placeholder bindings before building substitution arrays.
        bindings.retain(|_k, v| !is_generic_placeholder(v, generic_names));
        if bindings.is_empty() {
            return None;
        }
        let param_names: Vec<String> = bindings.keys().cloned().collect();
        let concrete_types: Vec<TypeInfo> = bindings.values().cloned().collect();
        ret_ann
            .as_ref()
            .map(|ann| self.substitute_generics(ann, &param_names, &concrete_types))
    }

    #[allow(dead_code)]
    pub(super) fn check_expr_type(
        &mut self,
        expr: &Expr,
        expected: &TypeInfo,
    ) -> Result<(), FerriError> {
        let inferred = self.infer_expr(expr)?;
        if !expected.accepts(&inferred) {
            let span = expr.span();
            return Err(FerriError::TypeError {
                message: format!(
                    "type mismatch: expected `{}`, got `{}`",
                    expected.name(),
                    inferred.name()
                ),
                line: span.line,
                column: span.column,
            });
        }
        Ok(())
    }

    /// Check arity + per-arg type compatibility against the declared
    /// `params`. `display_name` and `span` are used for error messages.
    /// `skip_self` drops the first param (for method-call syntax where
    /// the receiver is implicit). Returns the first mismatch as a
    /// `TypeError`, or `Ok(())` if all args fit.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn check_args_against_params(
        &mut self,
        params: &[TypeInfo],
        args: &[Expr],
        skip_self: bool,
        display_name: &str,
        fn_key: Option<&str>,
        span: Span,
    ) -> Result<(), FerriError> {
        self.check_args_against_params_with_bindings(
            params,
            args,
            skip_self,
            display_name,
            fn_key,
            &HashMap::new(),
            span,
        )
    }

    /// Like [`check_args_against_params`] but seeds `initial_bindings` from
    /// the receiver's concrete generic arguments (for method calls).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn check_args_against_params_with_bindings(
        &mut self,
        params: &[TypeInfo],
        args: &[Expr],
        skip_self: bool,
        display_name: &str,
        fn_key: Option<&str>,
        initial_bindings: &HashMap<String, TypeInfo>,
        span: Span,
    ) -> Result<(), FerriError> {
        let effective: &[TypeInfo] = if skip_self && !params.is_empty() {
            &params[1..]
        } else {
            params
        };
        if args.len() != effective.len() {
            return Err(FerriError::TypeError {
                message: format!(
                    "wrong number of arguments to `{display_name}`: expected {}, got {}",
                    effective.len(),
                    args.len()
                ),
                line: span.line,
                column: span.column,
            });
        }
        // Infer arg types first so we can check cross-param generic consistency.
        let arg_types: Vec<TypeInfo> = args
            .iter()
            .map(|a| self.infer_expr(a))
            .collect::<Result<_, _>>()?;

        // Check generic-param consistency across arguments.
        if let Some(key) = fn_key {
            if let Some((generic_names, param_anns, _)) = self.fn_generic_info.get(key) {
                let effective_anns: &[TypeAnnotation] = if skip_self && !param_anns.is_empty() {
                    &param_anns[1..]
                } else {
                    param_anns
                };
                let mut bindings: HashMap<String, TypeInfo> = initial_bindings.clone();
                for (i, (ann, arg_ty)) in effective_anns.iter().zip(arg_types.iter()).enumerate() {
                    if let Err((_, msg)) =
                        bind_generic_params(ann, arg_ty, generic_names, &mut bindings, i)
                    {
                        let arg_span = args[i].span();
                        return Err(FerriError::TypeError {
                            message: format!(
                                "type mismatch in call to `{display_name}`: argument {} - {msg}",
                                i + 1,
                            ),
                            line: arg_span.line,
                            column: arg_span.column,
                        });
                    }
                }
            }
        }

        // Per-argument type check.
        for (i, (param_ty, arg_ty)) in effective.iter().zip(arg_types.iter()).enumerate() {
            if !param_ty.accepts(arg_ty) {
                let arg_span = args[i].span();
                return Err(FerriError::TypeError {
                    message: format!(
                        "type mismatch in call to `{display_name}`: argument {} expected `{}`, got `{}`",
                        i + 1,
                        param_ty.name(),
                        arg_ty.name()
                    ),
                    line: arg_span.line,
                    column: arg_span.column,
                });
            }
        }
        Ok(())
    }

    /// Infers the type of an expression. This is a thin dispatcher: each
    /// non-trivial variant is handled by a dedicated `infer_<variant>` method
    /// below, so this `match` stays a readable table of the expression grammar.
    pub(super) fn infer_expr(&mut self, expr: &Expr) -> Result<TypeInfo, FerriError> {
        match expr {
            Expr::IntLiteral(..) => Ok(TypeInfo::I64),
            Expr::FloatLiteral(..) => Ok(TypeInfo::F64),
            Expr::BoolLiteral(..) => Ok(TypeInfo::Bool),
            Expr::StringLiteral(..) => Ok(TypeInfo::String),
            Expr::CharLiteral(..) => Ok(TypeInfo::Char),

            Expr::Ident(name, _span) => self.infer_ident(name, _span),

            Expr::BinaryOp {
                op,
                left,
                right,
                span,
            } => self.infer_binary_op(op, left, right, span),

            Expr::UnaryOp {
                op,
                expr: inner,
                span,
            } => self.infer_unary_op(op, inner, span),

            Expr::Call {
                callee, args, span, ..
            } => self.infer_call(callee, args, span),

            Expr::Block(block) => self.infer_block(block),

            Expr::If {
                condition,
                then_block,
                else_block,
                span,
            } => self.infer_if(condition, then_block, else_block, span),

            Expr::IfLet {
                pattern,
                expr: inner,
                guard,
                then_block,
                else_block,
                span,
            } => self.infer_if_let(pattern, inner, guard, then_block, else_block, span),

            Expr::Grouped(inner, _) => self.infer_expr(inner),

            Expr::Repeat { value, count, .. } => self.infer_repeat(value, count),

            Expr::Array { elements, span } => self.infer_array(elements, span),

            Expr::Tuple { elements, .. } => self.infer_tuple(elements),

            Expr::Assign { target, value, .. } => self.infer_assign(target, value),

            Expr::Match {
                expr: matched,
                arms,
                span,
            } => self.infer_match(matched, arms, span),

            Expr::PathCall {
                path, args, span, ..
            } => self.infer_path_call(path, args, span),

            Expr::MethodCall {
                object,
                method,
                args,
                span,
                ..
            } => self.infer_method_call(object, method, args, span),

            Expr::FieldAccess {
                object,
                field,
                span,
                ..
            } => self.infer_field_access(object, field, span),

            Expr::Index { object, index, .. } => self.infer_index(object, index),

            Expr::Range {
                start, end, span, ..
            } => self.infer_range(start, end, span),

            Expr::StructInit {
                name,
                fields,
                base,
                span,
                ..
            } => self.infer_struct_init(name, fields, base, span),

            Expr::Try { expr: inner, span } => self.infer_try(inner, span),

            Expr::Closure {
                params,
                return_type,
                body,
                is_async,
                ..
            } => self.infer_closure(params, return_type, body, is_async),
            Expr::AsyncBlock { body, .. } => self.infer_async_block(body),
            Expr::Await { expr: inner, .. } => self.infer_await(inner),
            Expr::FString { .. } => Ok(TypeInfo::String),
            Expr::MacroCall { name, args, .. } => self.infer_macro_call(name, args),
            Expr::Path { segments, .. } => self.infer_path(segments),
            Expr::SelfRef { .. } => self.infer_self_ref(),
            Expr::As {
                expr,
                type_name,
                span,
            } => self.infer_as(expr, type_name, span),
            Expr::Return { value, .. } => self.infer_return(value),
            Expr::CompoundAssign { target, value, .. } => self.infer_compound_assign(target, value),
        }
    }

    fn infer_compound_assign(
        &mut self,
        target: &Expr,
        value: &Expr,
    ) -> Result<TypeInfo, FerriError> {
        let vt = self.infer_expr(value)?;
        if let Expr::Ident(name, _) = target {
            if let Some(existing) = self.env.borrow().get(name) {
                if !existing.accepts(&vt) {
                    return Err(FerriError::TypeError {
                                message: format!(
                                    "type mismatch: cannot compound-assign `{}` to variable `{name}` of type `{}`",
                                    vt.name(),
                                    existing.name()
                                ),
                                line: target.span().line,
                                column: target.span().column,
                            });
                }
            }
        }
        Ok(TypeInfo::Unit)
    }

    fn infer_return(&mut self, value: &Option<Box<Expr>>) -> Result<TypeInfo, FerriError> {
        if let Some(expr) = value {
            let _ = self.infer_expr(expr)?;
        }
        Ok(TypeInfo::Unknown) // diverging expression
    }

    fn infer_as(
        &mut self,
        expr: &Expr,
        type_name: &str,
        span: &Span,
    ) -> Result<TypeInfo, FerriError> {
        let _ = self.infer_expr(expr)?;
        let target = TypeInfo::from_name(type_name);
        // `as` is only meaningful for primitive scalar conversions.
        // Anything that came back as `UserStruct` is an unknown name.
        let is_scalar = target.is_integer()
            || target.is_float()
            || matches!(target, TypeInfo::Bool | TypeInfo::String | TypeInfo::Char);
        if !is_scalar {
            return Err(FerriError::TypeError {
                        message: format!(
                            "`as` cast to unknown type `{type_name}`; only numeric, bool, String, and char are supported"
                        ),
                        line: span.line,
                        column: span.column,
                    });
        }
        Ok(target)
    }

    fn infer_self_ref(&mut self) -> Result<TypeInfo, FerriError> {
        if let Some(ref impl_type) = self.current_impl_type {
            Ok(TypeInfo::user_struct(impl_type.clone()))
        } else {
            Ok(TypeInfo::Unknown)
        }
    }

    fn infer_path(&mut self, segments: &[String]) -> Result<TypeInfo, FerriError> {
        let qualified = segments.join("::");
        if let Some(ret) = self.fn_return_types.get(&qualified) {
            return Ok(ret.clone());
        }
        if self.struct_defs.contains_key(&qualified) {
            return Ok(TypeInfo::user_struct(qualified));
        }
        // Try through use_aliases for the first segment
        if segments.len() == 2 {
            if let Some(resolved) = self.use_aliases.get(&segments[0]) {
                let full = format!("{}::{}", resolved, segments[1]);
                if self.struct_defs.contains_key(&full) {
                    return Ok(TypeInfo::user_struct(full));
                }
            }
        }
        Ok(TypeInfo::Unknown)
    }

    fn infer_macro_call(&mut self, name: &str, args: &[Expr]) -> Result<TypeInfo, FerriError> {
        // Infer all args so nested calls / field accesses still get
        // type-checked.
        let arg_types: Vec<TypeInfo> = args
            .iter()
            .map(|a| self.infer_expr(a))
            .collect::<Result<_, _>>()?;
        if name == "vec" {
            // vec![a, b, c] must be homogeneous (or contain Unknown).
            let mut leader = TypeInfo::Unknown;
            for (i, t) in arg_types.iter().enumerate() {
                if *t == TypeInfo::Unknown {
                    continue;
                }
                if leader == TypeInfo::Unknown {
                    leader = t.clone();
                    continue;
                }
                if leader.accepts(t) {
                    continue;
                }
                if t.accepts(&leader) {
                    leader = t.clone();
                    continue;
                }
                let espan = args[i].span();
                return Err(FerriError::TypeError {
                    message: format!(
                        "`vec!` has mixed element types: element {} is `{}`, expected `{}`",
                        i + 1,
                        t.name(),
                        leader.name()
                    ),
                    line: espan.line,
                    column: espan.column,
                });
            }
            return Ok(TypeInfo::Vec(Box::new(leader)));
        }
        if name == "dbg" && arg_types.len() == 1 {
            // dbg!(expr) returns its argument unchanged.
            return Ok(arg_types.into_iter().next().unwrap());
        }
        Ok(TypeInfo::Unknown)
    }

    fn infer_await(&mut self, inner: &Expr) -> Result<TypeInfo, FerriError> {
        let inner_ty = self.infer_expr(inner)?;
        match inner_ty {
            TypeInfo::Future(t) => Ok(*t),
            TypeInfo::JoinHandle(t) => Ok(*t),
            _ => Ok(inner_ty),
        }
    }

    fn infer_async_block(&mut self, body: &Block) -> Result<TypeInfo, FerriError> {
        let last_ty = self.block_tail_type(body)?;
        Ok(TypeInfo::Future(Box::new(last_ty)))
    }

    fn infer_closure(
        &mut self,
        params: &[ClosureParam],
        return_type: &Option<TypeAnnotation>,
        body: &Expr,
        is_async: &bool,
    ) -> Result<TypeInfo, FerriError> {
        let mut param_types = Vec::with_capacity(params.len());
        let closure_env = TypeEnv::child(&self.env);
        for p in params {
            let p_ty = if let Some(ref ann) = p.type_ann {
                self.resolve_annotation(ann)
            } else {
                TypeInfo::Unknown
            };
            closure_env.borrow_mut().define(&p.name, p_ty.clone());
            param_types.push(p_ty);
        }
        let saved_env = self.env.clone();
        self.env = closure_env;
        let inferred_ret = self.infer_expr(body)?;
        self.env = saved_env;
        if let Some(ref ann) = return_type {
            let declared_ret = self.resolve_annotation(ann);
            if !declared_ret.accepts(&inferred_ret) {
                return Err(FerriError::TypeError {
                    message: format!(
                        "type mismatch: closure returns `{}`, but body has type `{}`",
                        declared_ret.name(),
                        inferred_ret.name()
                    ),
                    line: ann.span().line,
                    column: ann.span().column,
                });
            }
        }
        let ret_ty = if *is_async {
            TypeInfo::Future(Box::new(inferred_ret))
        } else {
            inferred_ret
        };
        Ok(TypeInfo::Function {
            params: param_types,
            ret: Box::new(ret_ty),
        })
    }

    fn infer_try(&mut self, inner: &Expr, span: &Span) -> Result<TypeInfo, FerriError> {
        let inner_ty = self.infer_expr(inner)?;
        // The `?` operator only makes sense in a function whose
        // return type is `Result<_, _>` or `Option<_>`. Otherwise
        // an error/None propagated by `?` would silently vanish
        // off the end of the function — exit 0 with no output.
        let ok_here = matches!(
            &self.current_fn_return,
            TypeInfo::Result(..) | TypeInfo::Option(..) | TypeInfo::Unknown
        );
        if !ok_here {
            return Err(FerriError::TypeError {
                message: format!(
                    "`?` cannot be used in a function returning `{}`. \
                             The enclosing function must return `Result<_, _>` or \
                             `Option<_>` so `?` has something to propagate into. \
                             Use a `match` on the expression instead, or change \
                             the function signature.",
                    self.current_fn_return.display_name()
                ),
                line: span.line,
                column: span.column,
            });
        }
        // The expression being `?`'d must itself be a Result or Option.
        // (Unknown is allowed as a wildcard so we don't false-positive
        // on values we couldn't infer.)
        match &inner_ty {
            TypeInfo::Result(ok, _) => Ok((**ok).clone()),
            TypeInfo::Option(inner) => Ok((**inner).clone()),
            TypeInfo::Unknown => Ok(TypeInfo::Unknown),
            other => Err(FerriError::TypeError {
                message: format!(
                    "`?` requires a `Result` or `Option` operand; got `{}`",
                    other.display_name()
                ),
                line: span.line,
                column: span.column,
            }),
        }
    }

    fn infer_struct_init(
        &mut self,
        name: &str,
        fields: &[(String, Expr)],
        base: &Option<Box<Expr>>,
        span: &Span,
    ) -> Result<TypeInfo, FerriError> {
        let resolved = self.resolve_struct_name(name);
        self.check_path_visible(&resolved, *span)?;
        // Pre-collect declared field types AND each field's raw
        // annotation, so we can infer concrete generic-arg types
        // from the supplied values (`Box { value: 5 }` → T = i64).
        let generic_param_names: Vec<String> = self
            .struct_defs
            .get(&resolved)
            .map(|def| def.generic_params.iter().map(|p| p.name.clone()).collect())
            .unwrap_or_default();
        let decl_field_info: HashMap<String, (TypeAnnotation, TypeInfo)> = self
            .struct_defs
            .get(&resolved)
            .and_then(|def| match &def.kind {
                StructKind::Named(decl_fields) => Some(
                    decl_fields
                        .iter()
                        .map(|f| {
                            let ty = match &f.type_ann {
                                TypeAnnotation::Named { name, .. }
                                    if generic_param_names.contains(name) =>
                                {
                                    TypeInfo::Unknown
                                }
                                ann => self.resolve_annotation(ann),
                            };
                            (f.name.clone(), (f.type_ann.clone(), ty))
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default();
        // First pass: infer field values, capture generic-arg bindings.
        let mut inferred_generics: Vec<TypeInfo> =
            vec![TypeInfo::Unknown; generic_param_names.len()];
        let mut field_value_types: Vec<(String, TypeInfo, Span)> = Vec::with_capacity(fields.len());
        for (field_name, f_expr) in fields {
            self.check_field_visible(&resolved, field_name, *span)?;
            let val_ty = self.infer_expr(f_expr)?;
            if let Some((ann, _)) = decl_field_info.get(field_name) {
                if let TypeAnnotation::Named { name: tname, .. } = ann {
                    if let Some(idx) = generic_param_names.iter().position(|g| g == tname) {
                        if inferred_generics[idx] == TypeInfo::Unknown {
                            inferred_generics[idx] = val_ty.clone();
                        }
                    }
                }
            }
            field_value_types.push((field_name.clone(), val_ty, f_expr.span()));
        }
        // Second pass: validate each field against the substituted
        // declared type.
        for (field_name, val_ty, fspan) in &field_value_types {
            if let Some((raw_ann, _)) = decl_field_info.get(field_name) {
                let decl_ty =
                    self.substitute_generics(raw_ann, &generic_param_names, &inferred_generics);
                if !decl_ty.accepts(val_ty) {
                    return Err(FerriError::TypeError {
                        message: format!(
                            "type mismatch: field `{}.{field_name}` declared as `{}`, got `{}`",
                            resolved,
                            decl_ty.display_name(),
                            val_ty.display_name()
                        ),
                        line: fspan.line,
                        column: fspan.column,
                    });
                }
            }
        }
        // Type-check the `..base` expression if present.
        if let Some(base_expr) = base {
            let _ = self.infer_expr(base_expr)?;
        }
        // If `resolved` names a struct-style enum variant (e.g.
        // `Shape::Rectangle`), the produced value's type is the
        // enclosing enum (`Shape`), not the variant. Without this,
        // `area(Shape::Rectangle { ... })` is rejected because the
        // arg types `Shape` and `Shape::Rectangle` don't match.
        let final_name = match resolved.rsplit_once("::") {
            Some((parent, _)) if self.enum_defs.contains(parent) => parent.to_string(),
            _ => resolved,
        };
        Ok(TypeInfo::UserStruct {
            name: final_name,
            generic_args: inferred_generics,
        })
    }

    fn infer_range(
        &mut self,
        start: &Option<Box<Expr>>,
        end: &Option<Box<Expr>>,
        span: &Span,
    ) -> Result<TypeInfo, FerriError> {
        if let Some(s) = start {
            let st = self.infer_expr(s)?;
            if st != TypeInfo::Unknown && !st.is_integer() {
                return Err(FerriError::TypeError {
                    message: format!("range start must be an integer, got `{}`", st.name()),
                    line: span.line,
                    column: span.column,
                });
            }
        }
        if let Some(e) = end {
            let et = self.infer_expr(e)?;
            if et != TypeInfo::Unknown && !et.is_integer() {
                return Err(FerriError::TypeError {
                    message: format!("range end must be an integer, got `{}`", et.name()),
                    line: span.line,
                    column: span.column,
                });
            }
        }
        // Treat ranges as Vec<int> for type-checking purposes: at
        // runtime a Range produces an Iterator, but Iterator and
        // Vec share the same method surface in our checker, so
        // pretending the type is `Vec<int>` lets `(0..n).map(...)`,
        // `.collect()`, `.sum()`, etc. all type-check without
        // introducing a separate TypeInfo::Range variant.
        Ok(TypeInfo::Vec(Box::new(TypeInfo::I64)))
    }

    fn infer_index(&mut self, object: &Expr, index: &Expr) -> Result<TypeInfo, FerriError> {
        let obj_ty = self.infer_expr(object)?;
        let idx_ty = self.infer_expr(index)?;
        let is_range_index = matches!(index, Expr::Range { .. });
        // Sequence indexing requires an integer (or a range for slicing).
        let is_seq = matches!(
            obj_ty,
            TypeInfo::Vec(_) | TypeInfo::Array(..) | TypeInfo::String
        );
        if is_seq && !is_range_index && idx_ty != TypeInfo::Unknown && !idx_ty.is_integer() {
            let ispan = index.span();
            return Err(FerriError::TypeError {
                message: format!(
                    "cannot index `{}` with `{}`: expected integer",
                    obj_ty.name(),
                    idx_ty.name()
                ),
                line: ispan.line,
                column: ispan.column,
            });
        }
        if obj_ty == TypeInfo::String {
            // Range index → String slice; integer index → Char.
            return Ok(if is_range_index {
                TypeInfo::String
            } else {
                TypeInfo::Char
            });
        }
        if let TypeInfo::Array(elem, _) = &obj_ty {
            if is_range_index {
                return Ok(TypeInfo::Vec(elem.clone()));
            }
            return Ok((**elem).clone());
        }
        if let TypeInfo::Vec(elem) = &obj_ty {
            if is_range_index {
                return Ok(TypeInfo::Vec(elem.clone()));
            }
            return Ok((**elem).clone());
        }
        Ok(TypeInfo::Unknown)
    }

    fn infer_field_access(
        &mut self,
        object: &Expr,
        field: &str,
        span: &Span,
    ) -> Result<TypeInfo, FerriError> {
        let obj_ty = self.infer_expr(object)?;
        if let TypeInfo::UserStruct {
            name: struct_name,
            generic_args,
        } = &obj_ty
        {
            let resolved = self.resolve_struct_name(struct_name);
            self.check_field_visible(&resolved, field, *span)?;
            if let Some(def) = self.struct_defs.get(&resolved) {
                let generic_param_names: Vec<String> =
                    def.generic_params.iter().map(|p| p.name.clone()).collect();
                let generic_args_owned = generic_args.clone();
                let def = def.clone();
                match &def.kind {
                    StructKind::Named(fields) => {
                        for f in fields {
                            if f.name == *field {
                                return Ok(self.substitute_generics(
                                    &f.type_ann,
                                    &generic_param_names,
                                    &generic_args_owned,
                                ));
                            }
                        }
                        return Err(FerriError::TypeError {
                            message: format!("no field `{field}` on struct `{resolved}`"),
                            line: span.line,
                            column: span.column,
                        });
                    }
                    StructKind::Tuple(types) => {
                        if let Ok(idx) = field.parse::<usize>() {
                            if let Some(ann) = types.get(idx) {
                                return Ok(self.substitute_generics(
                                    ann,
                                    &generic_param_names,
                                    &generic_args_owned,
                                ));
                            }
                            return Err(FerriError::TypeError {
                                message: format!("no field `{field}` on tuple struct `{resolved}`"),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        return Ok(TypeInfo::Unknown);
                    }
                    StructKind::Unit => {
                        return Err(FerriError::TypeError {
                            message: format!("no field `{field}` on unit struct `{resolved}`"),
                            line: span.line,
                            column: span.column,
                        });
                    }
                }
            }
            return Ok(TypeInfo::Unknown);
        }
        // Tuple field access (`.0`, `.1`) is also Expr::FieldAccess
        // with a numeric-looking name. Leave those alone for now.
        if field.chars().all(|c| c.is_ascii_digit()) {
            return Ok(TypeInfo::Unknown);
        }
        // Builtin types (Vec, String, primitives, ...) have no
        // user-accessible fields. If the receiver type is known and
        // concrete, an unknown field is a compile error.
        if obj_ty != TypeInfo::Unknown && !matches!(obj_ty, TypeInfo::UserStruct { .. }) {
            return Err(FerriError::TypeError {
                message: format!("no field `{field}` on type `{}`", obj_ty.name()),
                line: span.line,
                column: span.column,
            });
        }
        Ok(TypeInfo::Unknown)
    }

    fn infer_method_call(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
        span: &Span,
    ) -> Result<TypeInfo, FerriError> {
        let obj_ty = self.infer_expr(object)?;
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
                    return Err(FerriError::TypeError {
                        message: format!("no method `{method}` on type `{}`", obj_ty.name()),
                        line: span.line,
                        column: span.column,
                    });
                }
                // Fixed-size arrays disallow Vec mutators.
                if matches!(obj_ty, TypeInfo::Array(..)) && self.is_array_mutator(method) {
                    return Err(FerriError::TypeError {
                                message: format!(
                                    "method `{method}` is not available on fixed-size arrays; convert to `Vec` first"
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

    fn infer_path_call(
        &mut self,
        path: &[String],
        args: &[Expr],
        span: &Span,
    ) -> Result<TypeInfo, FerriError> {
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

    fn infer_match(
        &mut self,
        matched: &Expr,
        arms: &[MatchArm],
        span: &Span,
    ) -> Result<TypeInfo, FerriError> {
        let matched_ty = self.infer_expr(matched)?;
        if !self.match_is_exhaustive(&matched_ty, arms) {
            return Err(FerriError::TypeError {
                message: "non-exhaustive match: add a `_ =>` arm or cover all cases".to_string(),
                line: span.line,
                column: span.column,
            });
        }
        let mut arm_types: Vec<TypeInfo> = Vec::with_capacity(arms.len());
        for arm in arms {
            let arm_env = TypeEnv::child(&self.env);
            let saved = self.env.clone();
            self.env = arm_env;
            self.bind_pattern(&arm.pattern, false);
            if let Some(g) = &arm.guard {
                let _ = self.infer_expr(g)?;
            }
            let arm_ty = self.infer_expr(&arm.body)?;
            self.env = saved;
            arm_types.push(arm_ty);
        }
        // Pick the first non-Unit/non-Unknown arm as the leader,
        // then require all other producing-arms to unify with it.
        let mut leader: TypeInfo = TypeInfo::Unit;
        for t in &arm_types {
            if *t == TypeInfo::Unknown || *t == TypeInfo::Unit {
                continue;
            }
            if leader == TypeInfo::Unit {
                leader = t.clone();
                continue;
            }
            leader = self.unify_branch_types(&leader, t, "match", *span)?;
        }
        Ok(leader)
    }

    fn infer_assign(&mut self, target: &Expr, value: &Expr) -> Result<TypeInfo, FerriError> {
        let vt = self.infer_expr(value)?;
        match target {
            Expr::Ident(name, _) => {
                let existing_mut = self.env.borrow().get_mutable(name);
                // Reassigning a known binding requires it to be `let mut`.
                if existing_mut == Some(false) {
                    return Err(FerriError::TypeError {
                                message: format!(
                                    "cannot assign to immutable variable `{name}`; declare it with `let mut {name}`"
                                ),
                                line: target.span().line,
                                column: target.span().column,
                            });
                }
                // Check compatibility with existing binding
                if let Some(existing) = self.env.borrow().get(name) {
                    if !existing.accepts(&vt) {
                        return Err(FerriError::TypeError {
                                    message: format!(
                                        "type mismatch: cannot assign `{}` to variable `{name}` of type `{}`",
                                        vt.name(),
                                        existing.name()
                                    ),
                                    line: target.span().line,
                                    column: target.span().column,
                                });
                    }
                }
                // Refine the binding's type but preserve its mutability —
                // an assignment must not silently turn a `let mut` into an
                // immutable binding (the tail expression is inferred twice).
                self.env
                    .borrow_mut()
                    .define_mut(name, vt, existing_mut.unwrap_or(true));
            }
            Expr::FieldAccess {
                object,
                field,
                span: fspan,
            } => {
                // Mutating a field requires the owning binding to be
                // mutable: `let mut x` for a variable, `mut self` for a
                // method receiver.
                self.check_assign_root_mutable(object, *fspan)?;
                let obj_ty = self.infer_expr(object)?;
                if let TypeInfo::UserStruct {
                    name: struct_name, ..
                } = &obj_ty
                {
                    let resolved = self.resolve_struct_name(struct_name);
                    if let Some(def) = self.struct_defs.get(&resolved) {
                        let generic_names: Vec<String> =
                            def.generic_params.iter().map(|p| p.name.clone()).collect();
                        if let StructKind::Named(decl_fields) = &def.kind {
                            for f in decl_fields {
                                if f.name == *field {
                                    let decl_ty = match &f.type_ann {
                                        TypeAnnotation::Named { name, .. }
                                            if generic_names.contains(name) =>
                                        {
                                            TypeInfo::Unknown
                                        }
                                        ann => self.resolve_annotation(ann),
                                    };
                                    if !decl_ty.accepts(&vt) {
                                        return Err(FerriError::TypeError {
                                                    message: format!(
                                                        "type mismatch: cannot assign `{}` to field `{}.{field}` of type `{}`",
                                                        vt.name(),
                                                        resolved,
                                                        decl_ty.name()
                                                    ),
                                                    line: fspan.line,
                                                    column: fspan.column,
                                                });
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(TypeInfo::Unit)
    }

    fn infer_tuple(&mut self, elements: &[Expr]) -> Result<TypeInfo, FerriError> {
        for e in elements {
            self.infer_expr(e)?;
        }
        Ok(TypeInfo::Unknown)
    }

    fn infer_array(&mut self, elements: &[Expr], span: &Span) -> Result<TypeInfo, FerriError> {
        let mut elem_types = Vec::with_capacity(elements.len());
        for e in elements {
            elem_types.push(self.infer_expr(e)?);
        }
        // Determine the array's element type. Pick the first non-Unknown
        // type as the "leader" and require every other element to be
        // compatible with it via the standard accepts rules. A mismatch
        // here means the literal is heterogeneous and we error out so
        // it can't be silently widened to Unknown.
        let mut leader: TypeInfo = TypeInfo::Unknown;
        for (i, t) in elem_types.iter().enumerate() {
            if leader == TypeInfo::Unknown {
                leader = t.clone();
                continue;
            }
            if leader.accepts(t) {
                continue;
            }
            if t.accepts(&leader) {
                leader = t.clone();
                continue;
            }
            let espan = elements[i].span();
            return Err(FerriError::TypeError {
                message: format!(
                    "array literal has mixed element types: element {} is `{}`, expected `{}`",
                    i + 1,
                    t.name(),
                    leader.name()
                ),
                line: espan.line,
                column: espan.column,
            });
        }
        let _ = span;
        Ok(TypeInfo::Array(Box::new(leader), elements.len()))
    }

    fn infer_repeat(&mut self, value: &Expr, count: &Expr) -> Result<TypeInfo, FerriError> {
        let val_ty = self.infer_expr(value)?;
        let _ = self.infer_expr(count)?;
        // The repeat count is the array's length, which must be known at
        // compile time. Only an integer literal qualifies; a variable or
        // other runtime expression is rejected (matching `[T; N]`).
        let n = match count {
            Expr::IntLiteral(n, _, _) => *n as usize,
            _ => {
                let span = count.span();
                return Err(FerriError::TypeError {
                    message: "array repeat count must be a constant integer literal, e.g. `[0; 5]`"
                        .to_string(),
                    line: span.line,
                    column: span.column,
                });
            }
        };
        Ok(TypeInfo::Array(Box::new(val_ty), n))
    }

    fn infer_if_let(
        &mut self,
        pattern: &Pattern,
        inner: &Expr,
        guard: &Option<Box<Expr>>,
        then_block: &Block,
        else_block: &Option<Box<Expr>>,
        span: &Span,
    ) -> Result<TypeInfo, FerriError> {
        let _ = self.infer_expr(inner)?;
        let saved = self.env.clone();
        self.env = TypeEnv::child(&saved);
        self.bind_pattern(pattern, false);
        if let Some(g) = guard {
            let _ = self.infer_expr(g)?;
        }
        let then_ty = self.block_tail_type(then_block)?;
        self.env = saved;
        let result = if let Some(else_expr) = else_block {
            let else_ty = self.infer_expr(else_expr)?;
            self.unify_branch_types(&then_ty, &else_ty, "if let", *span)?
        } else {
            then_ty
        };
        Ok(result)
    }

    fn infer_if(
        &mut self,
        condition: &Expr,
        then_block: &Block,
        else_block: &Option<Box<Expr>>,
        span: &Span,
    ) -> Result<TypeInfo, FerriError> {
        self.infer_expr(condition)?;
        let then_ty = self.block_tail_type(then_block)?;
        let result = if let Some(else_expr) = else_block {
            let else_ty = self.infer_expr(else_expr)?;
            self.unify_branch_types(&then_ty, &else_ty, "if", *span)?
        } else {
            then_ty
        };
        Ok(result)
    }

    fn infer_block(&mut self, block: &Block) -> Result<TypeInfo, FerriError> {
        let mut last_ty = TypeInfo::Unit;
        for (i, stmt) in block.stmts.iter().enumerate() {
            let is_last = i == block.stmts.len() - 1;
            self.check_stmt(stmt, &TypeInfo::Unknown)?;
            if is_last {
                if let Stmt::Expr {
                    expr,
                    has_semicolon,
                } = stmt
                {
                    if !has_semicolon {
                        last_ty = self.infer_expr(expr)?;
                    }
                }
            }
        }
        Ok(last_ty)
    }

    fn infer_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        span: &Span,
    ) -> Result<TypeInfo, FerriError> {
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
                            return Err(FerriError::TypeError {
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
                            return Err(FerriError::TypeError {
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
                            return Err(FerriError::TypeError {
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
                            return Err(FerriError::TypeError {
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
                return Err(FerriError::TypeError {
                    message: format!("expected {} arguments, found {}", params.len(), args.len()),
                    line: span.line,
                    column: span.column,
                });
            }
            return Ok(*ret.clone());
        }
        Ok(TypeInfo::Unknown)
    }

    fn infer_unary_op(
        &mut self,
        op: &UnaryOp,
        inner: &Expr,
        span: &Span,
    ) -> Result<TypeInfo, FerriError> {
        let inner_ty = self.infer_expr(inner)?;
        match op {
            UnaryOp::Neg => {
                // Allow UserStruct in case the type implements
                // operator overloading via a Neg trait impl.
                let ok = inner_ty == TypeInfo::Unknown
                    || inner_ty.is_integer()
                    || inner_ty.is_float()
                    || matches!(inner_ty, TypeInfo::UserStruct { .. });
                if !ok {
                    return Err(FerriError::TypeError {
                        message: format!(
                            "unary `-` requires a numeric operand, got `{}`",
                            inner_ty.name()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
                Ok(inner_ty)
            }
            UnaryOp::Not => {
                if inner_ty != TypeInfo::Unknown && inner_ty != TypeInfo::Bool {
                    return Err(FerriError::TypeError {
                        message: format!(
                            "unary `!` requires a `bool` operand, got `{}`",
                            inner_ty.name()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
                Ok(TypeInfo::Bool)
            }
            UnaryOp::BitNot => {
                if inner_ty != TypeInfo::Unknown && !inner_ty.is_integer() {
                    return Err(FerriError::TypeError {
                        message: format!(
                            "unary `~` requires an integer operand, got `{}`",
                            inner_ty.name()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
                Ok(inner_ty)
            }
        }
    }

    fn infer_binary_op(
        &mut self,
        op: &BinOp,
        left: &Expr,
        right: &Expr,
        span: &Span,
    ) -> Result<TypeInfo, FerriError> {
        let lt = self.infer_expr(left)?;
        let rt = self.infer_expr(right)?;
        let is_num = |t: &TypeInfo| t.is_integer() || t.is_float();
        let known = |t: &TypeInfo| *t != TypeInfo::Unknown;
        // Helper to format a clean operand mismatch error.
        let mk_err = |msg: String| FerriError::TypeError {
            message: msg,
            line: span.line,
            column: span.column,
        };
        match op {
            BinOp::Eq | BinOp::NotEq => {
                // Either side may be Unknown (e.g. closure args). Once
                // both are known we require one to accept the other.
                if known(&lt) && known(&rt) && !lt.accepts(&rt) && !rt.accepts(&lt) {
                    return Err(mk_err(format!(
                        "cannot compare `{}` and `{}` with `{op}`",
                        lt.name(),
                        rt.name()
                    )));
                }
                return Ok(TypeInfo::Bool);
            }
            BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                if known(&lt) && known(&rt) {
                    let both_num = is_num(&lt) && is_num(&rt);
                    let same_scalar = lt == rt
                        && matches!(lt, TypeInfo::String | TypeInfo::Char | TypeInfo::Bool);
                    if !both_num && !same_scalar {
                        return Err(mk_err(format!(
                            "cannot order `{}` and `{}` with `{op}`",
                            lt.name(),
                            rt.name()
                        )));
                    }
                }
                return Ok(TypeInfo::Bool);
            }
            BinOp::And | BinOp::Or => {
                if known(&lt) && lt != TypeInfo::Bool {
                    return Err(mk_err(format!(
                        "logical `{op}` requires `bool` operands, left is `{}`",
                        lt.name()
                    )));
                }
                if known(&rt) && rt != TypeInfo::Bool {
                    return Err(mk_err(format!(
                        "logical `{op}` requires `bool` operands, right is `{}`",
                        rt.name()
                    )));
                }
                return Ok(TypeInfo::Bool);
            }
            BinOp::Add => {
                // String/Char concatenation paths.
                if lt == TypeInfo::String || rt == TypeInfo::String {
                    return Ok(TypeInfo::String);
                }
                if lt == TypeInfo::Char || rt == TypeInfo::Char {
                    return Ok(TypeInfo::String);
                }
            }
            BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                // Pure arithmetic — String/Char operands are illegal.
            }
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                if known(&lt) && !lt.is_integer() {
                    return Err(mk_err(format!(
                        "bitwise `{op}` requires integer operands, left is `{}`",
                        lt.name()
                    )));
                }
                if known(&rt) && !rt.is_integer() {
                    return Err(mk_err(format!(
                        "bitwise `{op}` requires integer operands, right is `{}`",
                        rt.name()
                    )));
                }
                return Ok(if lt.is_integer() { lt } else { rt });
            }
        }
        // Arithmetic Add/Sub/Mul/Div/Mod — operands must be numeric,
        // or user-defined structs (which may implement operator
        // overloading via traits).
        let arithmetic_ok = |t: &TypeInfo| {
            *t == TypeInfo::Unknown || is_num(t) || matches!(t, TypeInfo::UserStruct { .. })
        };
        if !arithmetic_ok(&lt) {
            return Err(mk_err(format!(
                "arithmetic `{op}` requires numeric operands, left is `{}`",
                lt.name()
            )));
        }
        if !arithmetic_ok(&rt) {
            return Err(mk_err(format!(
                "arithmetic `{op}` requires numeric operands, right is `{}`",
                rt.name()
            )));
        }
        // User-struct operator overloading: result type is the struct
        // (Add/Sub on Vec2 -> Vec2, etc).
        if let TypeInfo::UserStruct { .. } = &lt {
            return Ok(lt);
        }
        if let TypeInfo::UserStruct { .. } = &rt {
            return Ok(rt);
        }
        if matches!(lt, TypeInfo::F64) || matches!(rt, TypeInfo::F64) {
            Ok(TypeInfo::F64)
        } else {
            Ok(TypeInfo::I64)
        }
    }

    fn infer_ident(&mut self, name: &str, _span: &Span) -> Result<TypeInfo, FerriError> {
        if let Some(ty) = self.env.borrow().get(name) {
            return Ok(ty);
        }
        // An ident that names a function used as a value (e.g.
        // `apply(square, 5)`) must infer to a fn-pointer type, NOT
        // the function's return type. Previously this branch
        // returned the return type alone, so `square` was typed as
        // `int` and the apply(fn(int) -> int, …) signature
        // rejected it.
        if let Some(ret) = self.fn_return_types.get(name) {
            let params = self.fn_param_types.get(name).cloned().unwrap_or_default();
            return Ok(TypeInfo::Function {
                params,
                ret: Box::new(ret.clone()),
            });
        }
        // Try module-qualified function name
        {
            let module_prefix = self.module_stack.join("::");
            if !module_prefix.is_empty() {
                let qualified = format!("{}::{}", module_prefix, name);
                if let Some(ret) = self.fn_return_types.get(&qualified) {
                    let params = self
                        .fn_param_types
                        .get(&qualified)
                        .cloned()
                        .unwrap_or_default();
                    return Ok(TypeInfo::Function {
                        params,
                        ret: Box::new(ret.clone()),
                    });
                }
            }
        }
        // Try use_aliases -> struct_defs
        if let Some(resolved) = self.use_aliases.get(name) {
            if self.struct_defs.contains_key(resolved) {
                return Ok(TypeInfo::user_struct(resolved.clone()));
            }
        }
        // Try module-qualified struct name
        let resolved = self.resolve_struct_name(name);
        if self.struct_defs.contains_key(&resolved) {
            return Ok(TypeInfo::user_struct(resolved));
        }
        // A name that matches any known symbol — a function (possibly
        // reached via `use`/glob/`super`/re-export, so matched by short
        // name), struct, enum, enum variant, or `const` — is a value
        // reference, not an undefined variable. Function/path resolution
        // for callees happens in `Expr::Call`; this guard only prevents
        // the value-position fallback from flagging a legitimate name.
        if self.name_matches_known_symbol(name) {
            return Ok(TypeInfo::Unknown);
        }
        // Nothing resolved — this is a genuine undefined variable.
        // Offer a "did you mean" hint from names in scope plus
        // top-level functions, mirroring the old interpreter's DX.
        let mut candidates: Vec<String> = Vec::new();
        self.env.borrow().collect_names(&mut candidates);
        candidates.extend(self.fn_return_types.keys().cloned());
        let suggestion = crate::errors::suggest_name(name, candidates.iter().map(|s| s.as_str()));
        let message = match suggestion {
            Some(s) => format!("undefined variable '{name}'; did you mean '{s}'?"),
            None => format!("undefined variable '{name}'"),
        };
        Err(FerriError::TypeError {
            message,
            line: _span.line,
            column: _span.column,
        })
    }

    /// Whether `name` (a bare identifier) corresponds to any program-level
    /// symbol: a function, struct, enum, enum variant, `const`, or re-export.
    /// Functions/types may be reached through `use`/glob/`super`/re-export, so
    /// they are matched by short name (the last `::` segment) — being permissive
    /// here only risks *not* flagging a typo that happens to collide with a real
    /// symbol name, never a false "undefined" on legitimate code.
    fn name_matches_known_symbol(&self, name: &str) -> bool {
        if self.enum_variant_names.contains(name) || self.const_names.contains(name) {
            return true;
        }
        let suffix = format!("::{name}");
        let short_match = |k: &String| k == name || k.ends_with(&suffix);
        self.fn_return_types.keys().any(short_match)
            || self.struct_defs.keys().any(short_match)
            || self.enum_defs.iter().any(short_match)
            || self.reexports.keys().any(short_match)
    }

    /// Walk to the root of an assignment target's object chain and ensure the
    /// owning binding is mutable. `x.f = …` needs `let mut x`; `self.f = …`
    /// needs `mut self`. Roots without a binding name (indexing a temporary,
    /// etc.) are left unchecked.
    fn check_assign_root_mutable(&self, object: &Expr, span: Span) -> Result<(), FerriError> {
        let mut cur = object;
        loop {
            match cur {
                Expr::FieldAccess { object, .. } | Expr::Index { object, .. } => {
                    cur = object;
                }
                Expr::Grouped(inner, _) => {
                    cur = inner;
                }
                Expr::Ident(name, _) => {
                    if let Some(false) = self.env.borrow().get_mutable(name) {
                        return Err(FerriError::TypeError {
                            message: format!(
                                "cannot assign to a field of immutable variable `{name}`; declare it with `let mut {name}`"
                            ),
                            line: span.line,
                            column: span.column,
                        });
                    }
                    return Ok(());
                }
                Expr::SelfRef { .. } => {
                    if let Some(false) = self.env.borrow().get_mutable("self") {
                        return Err(FerriError::TypeError {
                            message:
                                "cannot assign to a field through immutable `self`; declare the method with `mut self`"
                                    .to_string(),
                            line: span.line,
                            column: span.column,
                        });
                    }
                    return Ok(());
                }
                _ => return Ok(()),
            }
        }
    }

    /// Decide whether a `match` covers every possible value of `matched_ty`.
    /// Conservative: only reports non-exhaustive when confident (a closed
    /// domain — scalar, bool, or known enum — with no catch-all). Unknown or
    /// open types are assumed exhaustive to avoid false positives.
    fn match_is_exhaustive(&self, matched_ty: &TypeInfo, arms: &[MatchArm]) -> bool {
        // A guardless irrefutable arm covers everything that remains.
        if arms
            .iter()
            .any(|a| a.guard.is_none() && Self::pattern_is_irrefutable(&a.pattern))
        {
            return true;
        }
        // Bool: exhaustive iff both `true` and `false` are matched literally.
        if *matched_ty == TypeInfo::Bool {
            let (mut seen_true, mut seen_false) = (false, false);
            for arm in arms {
                if arm.guard.is_some() {
                    continue;
                }
                Self::collect_bool_literals(&arm.pattern, &mut seen_true, &mut seen_false);
            }
            return seen_true && seen_false;
        }
        // Known enum: exhaustive iff every variant appears in a guardless arm.
        if let TypeInfo::UserStruct { name, .. } = matched_ty {
            if let Some(variants) = self.enum_variants.get(name) {
                let mut covered = std::collections::HashSet::new();
                for arm in arms {
                    if arm.guard.is_some() {
                        continue;
                    }
                    Self::collect_covered_variants(&arm.pattern, &mut covered);
                }
                return variants.iter().all(|v| covered.contains(v.as_str()));
            }
        }
        // Scalar domains (int/byte/float/string/char) are effectively
        // unbounded — without a catch-all they cannot be exhaustive.
        if matched_ty.is_integer()
            || matched_ty.is_float()
            || *matched_ty == TypeInfo::String
            || *matched_ty == TypeInfo::Char
        {
            return false;
        }
        // Open / unknown types: assume exhaustive (conservative).
        true
    }

    /// A pattern that matches any value of its type (binds or discards), so an
    /// arm using it is a catch-all.
    fn pattern_is_irrefutable(pattern: &Pattern) -> bool {
        match pattern {
            Pattern::Wildcard(_) | Pattern::Ident(_, _) | Pattern::Rest(_) => true,
            Pattern::Or(pats, _) => pats.iter().any(Self::pattern_is_irrefutable),
            Pattern::Tuple(pats, _) => pats.iter().all(Self::pattern_is_irrefutable),
            _ => false,
        }
    }

    fn collect_bool_literals(pattern: &Pattern, seen_true: &mut bool, seen_false: &mut bool) {
        match pattern {
            Pattern::Literal(Expr::BoolLiteral(true, _)) => *seen_true = true,
            Pattern::Literal(Expr::BoolLiteral(false, _)) => *seen_false = true,
            Pattern::Or(pats, _) => {
                for p in pats {
                    Self::collect_bool_literals(p, seen_true, seen_false);
                }
            }
            _ => {}
        }
    }

    fn collect_covered_variants<'a>(
        pattern: &'a Pattern,
        covered: &mut std::collections::HashSet<&'a str>,
    ) {
        match pattern {
            Pattern::EnumVariant { variant, .. } => {
                covered.insert(variant.as_str());
            }
            Pattern::Or(pats, _) => {
                for p in pats {
                    Self::collect_covered_variants(p, covered);
                }
            }
            _ => {}
        }
    }
}
