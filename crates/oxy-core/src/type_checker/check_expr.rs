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
    if let TypeAnnotation::Named {
        name, generic_args, ..
    } = ann
    {
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
    Ok(())
}

mod calls;
mod closures;
mod control_flow;
mod data;
mod operators;
mod primary;

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
    ) -> Result<(), PipelineError> {
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
    ) -> Result<(), PipelineError> {
        let effective: &[TypeInfo] = if skip_self && !params.is_empty() {
            &params[1..]
        } else {
            params
        };
        if args.len() != effective.len() {
            return Err(PipelineError::TypeError {
                message: format!(
                    "wrong number of arguments to `{display_name}`: expected {}, got {}",
                    effective.len(),
                    args.len()
                ),
                line: span.line,
                column: span.column,
            });
        }
        // Infer arg types, passing expected param types so that
        // closures can infer their parameter types from context
        // (bidirectional type checking).
        let arg_types: Vec<TypeInfo> = args
            .iter()
            .zip(effective.iter())
            .map(|(a, expected_ty)| self.infer_expr_expected(a, Some(expected_ty)))
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
                        return Err(PipelineError::TypeError {
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
                return Err(PipelineError::TypeError {
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

    /// Infers the type of an expression (inference mode — no expected type).
    /// This is the public entry point used by statements, items, and callers
    /// that don't have context about what type is expected.
    pub(super) fn infer_expr(&mut self, expr: &Expr) -> Result<TypeInfo, PipelineError> {
        self.infer_expr_expected(expr, None)
    }

    /// Infers the type of an expression with an optional expected type
    /// (bidirectional type checking). When `expected` is `Some`, the
    /// checker can use it to:
    /// - Auto-cast literals (e.g. `let x: float = 42`)
    /// - Infer closure parameter types from the expected function signature
    /// - Propagate context downward through grouped expressions
    ///
    /// This is a thin dispatcher: each non-trivial variant is handled by a
    /// dedicated `infer_<variant>` method below, so this `match` stays a
    /// readable table of the expression grammar.
    #[allow(clippy::only_used_in_recursion)]
    pub(super) fn infer_expr_expected(
        &mut self,
        expr: &Expr,
        expected: Option<&TypeInfo>,
    ) -> Result<TypeInfo, PipelineError> {
        match expr {
            // Literals: auto-cast to expected type when declared (e.g.
            // `let x: float = 42` → infer 42 as float).
            Expr::IntLiteral(..) => {
                if let Some(expected) = expected {
                    if expected.is_float() {
                        return Ok(TypeInfo::F64);
                    }
                    if *expected == TypeInfo::U8 {
                        return Ok(TypeInfo::U8);
                    }
                }
                Ok(TypeInfo::I64)
            }
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

            // Grouped expressions propagate the expected type inward.
            Expr::Grouped(inner, _) => self.infer_expr_expected(inner, expected),

            Expr::Repeat { value, count, .. } => self.infer_repeat(value, count),

            Expr::Array { elements, span } => {
                // Empty array: use expected element type.
                // `[...]` always creates a `List<T>` (growable), not a
                // fixed-size array — matching Gleam semantics.
                if elements.is_empty() {
                    if let Some(expected) = expected {
                        match expected {
                            TypeInfo::Vec(elem) => return Ok(TypeInfo::Vec(elem.clone())),
                            TypeInfo::Array(elem, _) => return Ok(TypeInfo::Vec(elem.clone())),
                            _ => {}
                        }
                    }
                }
                self.infer_array(elements, span)
            }

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
            } => self.infer_closure(params, return_type, body, is_async, expected),
            Expr::AsyncBlock { body, .. } => self.infer_async_block(body),
            Expr::Await { expr: inner, .. } => self.infer_await(inner),
            Expr::FString { .. } => Ok(TypeInfo::String),
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
}
