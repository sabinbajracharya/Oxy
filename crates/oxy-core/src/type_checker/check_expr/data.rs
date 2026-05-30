//! Type inference for aggregate/data expressions: struct init, field and
//! index access, tuples, arrays, repeats, and ranges.
//!
//! Part of `check_expr` — see that module for the `infer_expr` dispatcher.

use super::*;

impl TypeChecker {
    pub(super) fn infer_struct_init(
        &mut self,
        name: &str,
        fields: &[(String, Expr)],
        base: &Option<Box<Expr>>,
        span: &Span,
    ) -> Result<TypeInfo, PipelineError> {
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
                    return Err(PipelineError::TypeError {
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

    pub(super) fn infer_field_access(
        &mut self,
        object: &Expr,
        field: &str,
        span: &Span,
    ) -> Result<TypeInfo, PipelineError> {
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
                        return Err(PipelineError::TypeError {
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
                            return Err(PipelineError::TypeError {
                                message: format!("no field `{field}` on tuple struct `{resolved}`"),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        return Ok(TypeInfo::Unknown);
                    }
                    StructKind::Unit => {
                        return Err(PipelineError::TypeError {
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
            return Err(PipelineError::TypeError {
                message: format!("no field `{field}` on type `{}`", obj_ty.name()),
                line: span.line,
                column: span.column,
            });
        }
        Ok(TypeInfo::Unknown)
    }

    pub(super) fn infer_index(
        &mut self,
        object: &Expr,
        index: &Expr,
    ) -> Result<TypeInfo, PipelineError> {
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
            return Err(PipelineError::TypeError {
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

    pub(super) fn infer_tuple(&mut self, elements: &[Expr]) -> Result<TypeInfo, PipelineError> {
        for e in elements {
            self.infer_expr(e)?;
        }
        Ok(TypeInfo::Unknown)
    }

    pub(super) fn infer_array(
        &mut self,
        elements: &[Expr],
        span: &Span,
    ) -> Result<TypeInfo, PipelineError> {
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
            return Err(PipelineError::TypeError {
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

    pub(super) fn infer_repeat(
        &mut self,
        value: &Expr,
        count: &Expr,
    ) -> Result<TypeInfo, PipelineError> {
        let val_ty = self.infer_expr(value)?;
        let _ = self.infer_expr(count)?;
        // The repeat count is the array's length, which must be known at
        // compile time. Only an integer literal qualifies; a variable or
        // other runtime expression is rejected (matching `[T; N]`).
        let n = match count {
            Expr::IntLiteral(n, _, _) => *n as usize,
            _ => {
                let span = count.span();
                return Err(PipelineError::TypeError {
                    message: "array repeat count must be a constant integer literal, e.g. `[0; 5]`"
                        .to_string(),
                    line: span.line,
                    column: span.column,
                });
            }
        };
        Ok(TypeInfo::Array(Box::new(val_ty), n))
    }

    pub(super) fn infer_range(
        &mut self,
        start: &Option<Box<Expr>>,
        end: &Option<Box<Expr>>,
        span: &Span,
    ) -> Result<TypeInfo, PipelineError> {
        if let Some(s) = start {
            let st = self.infer_expr(s)?;
            if st != TypeInfo::Unknown && !st.is_integer() {
                return Err(PipelineError::TypeError {
                    message: format!("range start must be an integer, got `{}`", st.name()),
                    line: span.line,
                    column: span.column,
                });
            }
        }
        if let Some(e) = end {
            let et = self.infer_expr(e)?;
            if et != TypeInfo::Unknown && !et.is_integer() {
                return Err(PipelineError::TypeError {
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
}
