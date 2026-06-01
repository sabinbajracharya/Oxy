//! Type inference for operators and assignment: binary/unary ops,
//! `as` casts, and (compound-)assignment with its mutability check.
//!
//! Part of `check_expr` — see that module for the `infer_expr` dispatcher.

use super::*;

impl TypeChecker {
    pub(super) fn infer_binary_op(
        &mut self,
        op: &BinOp,
        left: &Expr,
        right: &Expr,
        span: &Span,
    ) -> Result<TypeInfo, PipelineError> {
        let lt = self.infer_expr(left)?;
        let rt = self.infer_expr(right)?;
        let is_num = |t: &TypeInfo| t.is_integer() || t.is_float();
        let known = |t: &TypeInfo| *t != TypeInfo::Unknown;
        // Helper to format a clean operand mismatch error.
        let mk_err = |msg: String| PipelineError::TypeError {
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

    pub(super) fn infer_unary_op(
        &mut self,
        op: &UnaryOp,
        inner: &Expr,
        span: &Span,
    ) -> Result<TypeInfo, PipelineError> {
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
                    return Err(PipelineError::TypeError {
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
                    return Err(PipelineError::TypeError {
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
                    return Err(PipelineError::TypeError {
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

    pub(super) fn infer_compound_assign(
        &mut self,
        target: &Expr,
        value: &Expr,
    ) -> Result<TypeInfo, PipelineError> {
        let vt = self.infer_expr(value)?;
        if let Expr::Ident(name, _) = target {
            if let Some(existing) = self.env.borrow().get(name) {
                if !existing.accepts(&vt) {
                    return Err(PipelineError::TypeError {
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

    pub(super) fn infer_as(
        &mut self,
        expr: &Expr,
        type_name: &str,
        span: &Span,
    ) -> Result<TypeInfo, PipelineError> {
        let _ = self.infer_expr(expr)?;
        let target = TypeInfo::from_name(type_name);
        // `as` is only meaningful for primitive scalar conversions.
        // Anything that came back as `UserStruct` is an unknown name.
        let is_scalar = target.is_integer()
            || target.is_float()
            || matches!(target, TypeInfo::Bool | TypeInfo::String | TypeInfo::Char);
        if !is_scalar {
            return Err(PipelineError::TypeError {
                        message: format!(
                            "`as` cast to unknown type `{type_name}`; only numeric, bool, String, and char are supported"
                        ),
                        line: span.line,
                        column: span.column,
                    });
        }
        Ok(target)
    }

    pub(super) fn infer_assign(
        &mut self,
        target: &Expr,
        value: &Expr,
    ) -> Result<TypeInfo, PipelineError> {
        let vt = self.infer_expr(value)?;
        match target {
            Expr::Ident(name, _) => {
                let existing_mut = self.env.borrow().get_mutable(name);
                // Reassigning a known binding requires it to be `let mut`.
                if existing_mut == Some(false) {
                    return Err(PipelineError::TypeError {
                                message: format!(
                                    "cannot assign to immutable variable `{name}`; declare it with `var {name}`"
                                ),
                                line: target.span().line,
                                column: target.span().column,
                            });
                }
                // Check compatibility with existing binding
                if let Some(existing) = self.env.borrow().get(name) {
                    if !existing.accepts(&vt) {
                        return Err(PipelineError::TypeError {
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
                                        return Err(PipelineError::TypeError {
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

    /// Walk to the root of an assignment target's object chain and ensure the
    /// owning binding is mutable. `x.f = …` needs `let mut x`; `self.f = …`
    /// needs `mut self`. Roots without a binding name (indexing a temporary,
    /// etc.) are left unchecked.
    fn check_assign_root_mutable(&self, object: &Expr, span: Span) -> Result<(), PipelineError> {
        if let Some(name) = self.immutable_root_binding_name(object) {
            return Err(PipelineError::TypeError {
                message: format!(
                    "cannot assign to a field of immutable variable `{name}`; declare it with `var {name}`"
                ),
                line: span.line,
                column: span.column,
            });
        }
        Ok(())
    }

    pub(super) fn immutable_root_binding_name(&self, object: &Expr) -> Option<String> {
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
                        return Some(name.clone());
                    }
                    return None;
                }
                Expr::SelfRef { .. } => {
                    // self is always mutable — field assignment is always allowed.
                    return None;
                }
                _ => return None,
            }
        }
    }
}
