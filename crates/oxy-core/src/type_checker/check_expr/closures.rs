//! Type inference for closures, `async` blocks, `await`, and the `?`
//! operator.
//!
//! Part of `check_expr` — see that module for the `infer_expr` dispatcher.

use super::*;

impl TypeChecker {
    pub(super) fn infer_closure(
        &mut self,
        params: &[ClosureParam],
        return_type: &Option<TypeAnnotation>,
        body: &Expr,
        is_async: &bool,
        expected: Option<&TypeInfo>,
    ) -> Result<TypeInfo, PipelineError> {
        // If the expected type is a function signature, use it to fill in
        // unannotated closure parameter types (bidirectional inference).
        let expected_params: Option<&[TypeInfo]> = match expected {
            Some(TypeInfo::Function { params: eps, .. }) if !eps.is_empty() => Some(eps.as_slice()),
            _ => None,
        };

        let mut param_types = Vec::with_capacity(params.len());
        let closure_env = TypeEnv::child(&self.env);
        for (i, p) in params.iter().enumerate() {
            let p_ty = if let Some(ref ann) = p.type_ann {
                self.resolve_annotation(ann)
            } else if let Some(eps) = expected_params {
                // Use the expected param type from the function signature
                // when the closure param is unannotated.
                eps.get(i).cloned().unwrap_or(TypeInfo::Unknown)
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
                return Err(PipelineError::TypeError {
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

    pub(super) fn infer_await(&mut self, inner: &Expr) -> Result<TypeInfo, PipelineError> {
        let inner_ty = self.infer_expr(inner)?;
        match inner_ty {
            TypeInfo::Future(t) => Ok(*t),
            TypeInfo::JoinHandle(t) => Ok(*t),
            _ => Ok(inner_ty),
        }
    }

    pub(super) fn infer_async_block(&mut self, body: &Block) -> Result<TypeInfo, PipelineError> {
        let last_ty = self.block_tail_type(body)?;
        Ok(TypeInfo::Future(Box::new(last_ty)))
    }

    pub(super) fn infer_try(
        &mut self,
        inner: &Expr,
        span: &Span,
    ) -> Result<TypeInfo, PipelineError> {
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
            return Err(PipelineError::TypeError {
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
            other => Err(PipelineError::TypeError {
                message: format!(
                    "`?` requires a `Result` or `Option` operand; got `{}`",
                    other.display_name()
                ),
                line: span.line,
                column: span.column,
            }),
        }
    }
}
