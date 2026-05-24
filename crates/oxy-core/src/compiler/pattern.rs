//! Pattern compilation: `compile_pattern`, `bind_pattern_data`, destructuring.
//!
//! ```text
//! pattern.rs  ── impl Compiler { compile_pattern, bind_pattern_data, ... }
//!   uses: mod.rs (Compiler struct), expr.rs (compile_expr via self)
//! ```

use crate::ast::*;
use crate::errors::FerriError;
use crate::types::IntegerWidth;
use crate::vm::OpCode;

use super::Compiler;

impl Compiler {
    /// Compile a pattern check.
    ///
    /// Stack contract (uniform across every variant):
    ///   Input:  [scrutinee]
    ///   Output: [bool]   — scrutinee is always consumed
    ///
    /// Binding sites are reserved here (via `sym.define`) but the actual
    /// bind happens in `bind_pattern_data` on the match path.
    pub(crate) fn compile_pattern(
        &mut self,
        pattern: &Pattern,
        _next_arm_labels: &mut Vec<usize>,
        _is_last: bool,
    ) -> Result<(), FerriError> {
        match pattern {
            Pattern::Wildcard(_) => {
                self.emit(OpCode::Pop);
                self.emit(OpCode::ConstBool(true));
                Ok(())
            }
            Pattern::Ident(name, _) => {
                // Always matches. Define the slot; the bind happens in
                // bind_pattern_data when the caller reloads the scrutinee.
                self.emit(OpCode::Pop);
                self.emit(OpCode::ConstBool(true));
                self.sym.define(name);
                Ok(())
            }
            Pattern::Literal(expr) => {
                self.compile_expr(expr)?; // [scrut, lit]
                self.emit(OpCode::Eq); // [bool]
                Ok(())
            }
            Pattern::EnumVariant {
                enum_name,
                variant,
                fields,
                ..
            } => {
                let resolved_enum = self
                    .type_aliases
                    .get(enum_name)
                    .cloned()
                    .or_else(|| self.use_aliases.get(enum_name).cloned())
                    .unwrap_or_else(|| {
                        let module_prefix = self.module_stack.join("::");
                        if !module_prefix.is_empty() {
                            let qualified = format!("{}::{}", module_prefix, enum_name);
                            if self.enum_defs.contains_key(&qualified) {
                                return qualified;
                            }
                        }
                        enum_name.clone()
                    });
                self.emit(OpCode::EnumVariantEqual {
                    enum_name: resolved_enum,
                    variant: variant.clone(),
                });
                self.define_pattern_slots(fields);
                Ok(())
            }
            Pattern::Range {
                start,
                end,
                inclusive,
                ..
            } => match (start, end) {
                (Some(s), None) => {
                    self.emit(OpCode::ConstInt(*s, IntegerWidth::I64));
                    self.emit(OpCode::Ge);
                    Ok(())
                }
                (None, Some(e)) => {
                    self.emit(OpCode::ConstInt(*e, IntegerWidth::I64));
                    if *inclusive {
                        self.emit(OpCode::Le);
                    } else {
                        self.emit(OpCode::Lt);
                    }
                    Ok(())
                }
                (Some(s), Some(e)) => {
                    let scrut_tmp = self.sym.define("__range_scrut");
                    self.emit(OpCode::StoreLocal(scrut_tmp));
                    self.emit(OpCode::LoadLocal(scrut_tmp));
                    self.emit(OpCode::ConstInt(*s, IntegerWidth::I64));
                    self.emit(OpCode::Ge); // [lower]
                    self.emit(OpCode::LoadLocal(scrut_tmp));
                    self.emit(OpCode::ConstInt(*e, IntegerWidth::I64));
                    if *inclusive {
                        self.emit(OpCode::Le);
                    } else {
                        self.emit(OpCode::Lt);
                    } // [lower, upper]
                    self.emit(OpCode::And); // [result]
                    Ok(())
                }
                (None, None) => {
                    self.emit(OpCode::Pop);
                    self.emit(OpCode::ConstBool(true));
                    Ok(())
                }
            },
            Pattern::Tuple(patterns, _) => {
                let scrut_tmp = self.sym.define("__tuple_scrut");
                self.emit(OpCode::StoreLocal(scrut_tmp));
                self.define_pattern_slots(patterns);
                let acc_tmp = self.sym.define("__tuple_acc");
                self.emit(OpCode::ConstBool(true));
                self.emit(OpCode::StoreLocal(acc_tmp));
                for (i, pat) in patterns.iter().enumerate() {
                    match pat {
                        Pattern::Wildcard(_) | Pattern::Ident(_, _) => {
                            // Always matches — accumulator unchanged.
                        }
                        Pattern::Literal(lit_expr) => {
                            self.emit(OpCode::LoadLocal(scrut_tmp));
                            self.emit(OpCode::ConstInt(i as i64, IntegerWidth::I64));
                            self.emit(OpCode::VecIndex);
                            self.compile_expr(lit_expr)?;
                            self.emit(OpCode::Eq);
                            self.emit(OpCode::LoadLocal(acc_tmp));
                            self.emit(OpCode::And);
                            self.emit(OpCode::StoreLocal(acc_tmp));
                        }
                        _ => {
                            self.emit(OpCode::ConstBool(false));
                            self.emit(OpCode::StoreLocal(acc_tmp));
                        }
                    }
                }
                self.emit(OpCode::LoadLocal(acc_tmp));
                Ok(())
            }
            Pattern::Or(pats, _) => {
                if pats.is_empty() {
                    self.emit(OpCode::Pop);
                    self.emit(OpCode::ConstBool(false));
                    return Ok(());
                }
                let scrut_tmp = self.sym.define("__or_scrut");
                self.emit(OpCode::StoreLocal(scrut_tmp));
                self.emit(OpCode::ConstBool(false));
                for pat in pats {
                    self.emit(OpCode::LoadLocal(scrut_tmp));
                    self.compile_pattern(pat, &mut vec![], false)?;
                    self.emit(OpCode::Or);
                }
                Ok(())
            }
            Pattern::Struct { fields, .. } => {
                // Type checker guarantees the scrutinee is the named struct.
                self.emit(OpCode::Pop);
                self.emit(OpCode::ConstBool(true));
                let sub_pats: Vec<Pattern> = fields.iter().map(|(_, p)| p.clone()).collect();
                self.define_pattern_slots(&sub_pats);
                Ok(())
            }
            _ => {
                // Slice, Rest — not yet supported. Drop scrutinee, fail match.
                self.emit(OpCode::Pop);
                self.emit(OpCode::ConstBool(false));
                Ok(())
            }
        }
    }

    /// Pre-define slots for pattern variables (called during pattern compilation).
    pub(crate) fn define_pattern_slots(&mut self, patterns: &[Pattern]) {
        for p in patterns {
            match p {
                Pattern::Ident(name, _) => {
                    self.sym.define(name);
                }
                Pattern::EnumVariant { fields, .. } | Pattern::Tuple(fields, _) => {
                    self.define_pattern_slots(fields);
                }
                Pattern::Struct { fields, .. } => {
                    let sub_pats: Vec<Pattern> = fields.iter().map(|(_, p)| p.clone()).collect();
                    self.define_pattern_slots(&sub_pats);
                }
                _ => {}
            }
        }
    }

    /// Bind pattern variables after a successful match.
    ///
    /// Stack contract (uniform across every variant):
    ///   Input:  [value]   — caller pushes the matched value
    ///   Output: []        — value is always consumed (Pop, BindIdent, or StoreLocal)
    pub(crate) fn bind_pattern_data(&mut self, pattern: &Pattern) -> Result<(), FerriError> {
        match pattern {
            Pattern::Wildcard(_) => {
                self.emit(OpCode::Pop);
                Ok(())
            }
            Pattern::Ident(name, _) => {
                if let Some(slot) = self.sym.get(name) {
                    self.emit(OpCode::BindIdent(slot));
                } else {
                    self.emit(OpCode::Pop);
                }
                Ok(())
            }
            Pattern::EnumVariant { fields, .. } => {
                let temp = self.sym.define("__variant_tmp");
                self.emit(OpCode::StoreLocal(temp));
                for (i, field_pat) in fields.iter().enumerate() {
                    self.emit(OpCode::LoadLocal(temp));
                    self.emit(OpCode::EnumDataGet(i));
                    self.bind_pattern_data(field_pat)?;
                }
                Ok(())
            }
            Pattern::Literal(_) => {
                self.emit(OpCode::Pop);
                Ok(())
            }
            Pattern::Struct { fields, .. } => {
                let temp = self.sym.define("__struct_pat_tmp");
                self.emit(OpCode::StoreLocal(temp));
                for (field_name, sub_pat) in fields {
                    self.emit(OpCode::LoadLocal(temp));
                    self.emit(OpCode::FieldAccess {
                        field_name: field_name.clone(),
                    });
                    self.bind_pattern_data(sub_pat)?;
                }
                Ok(())
            }
            Pattern::Tuple(patterns, _) => {
                let temp = self.sym.define("__tuple_tmp");
                self.emit(OpCode::StoreLocal(temp));
                for (i, pat) in patterns.iter().enumerate() {
                    self.emit(OpCode::LoadLocal(temp));
                    self.emit(OpCode::ConstInt(i as i64, IntegerWidth::I64));
                    self.emit(OpCode::VecIndex);
                    self.bind_pattern_data(pat)?;
                }
                Ok(())
            }
            _ => {
                // Range, Or, Slice, Rest — no bindings, drop the value.
                self.emit(OpCode::Pop);
                Ok(())
            }
        }
    }

    /// Native destructuring for tuple and slice patterns.
    pub(crate) fn compile_destructure(
        &mut self,
        value: &Expr,
        patterns: &[Pattern],
        span: crate::lexer::Span,
    ) -> Result<(), FerriError> {
        self.compile_expr(value)?;
        let temp_slot = self.sym.define("__destructure_tmp");
        self.emit(OpCode::StoreLocal(temp_slot));
        for (i, pat) in patterns.iter().enumerate() {
            match pat {
                Pattern::Ident(name, _) => {
                    self.emit(OpCode::LoadLocal(temp_slot));
                    self.emit(OpCode::ConstInt(i as i64, IntegerWidth::I64));
                    self.emit(OpCode::VecIndex);
                    let slot = self.sym.define(name);
                    self.emit(OpCode::BindIdent(slot));
                }
                Pattern::Wildcard(_) | Pattern::Rest(_) => {
                    // Skip — no binding needed
                }
                Pattern::Tuple(..) | Pattern::EnumVariant { .. } => {
                    // Nested pattern: pre-define binding slots, extract the
                    // element at index i, and recursively bind via the
                    // general bind_pattern_data path.
                    self.define_pattern_slots(std::slice::from_ref(pat));
                    self.emit(OpCode::LoadLocal(temp_slot));
                    self.emit(OpCode::ConstInt(i as i64, IntegerWidth::I64));
                    self.emit(OpCode::VecIndex);
                    self.bind_pattern_data(pat)?;
                }
                _ => {
                    return Err(FerriError::Runtime {
                        message: "complex destructure patterns not yet supported natively".into(),
                        line: span.line,
                        column: span.column,
                    });
                }
            }
        }
        Ok(())
    }

    /// Complex let-patterns not yet supported in native bytecode.
    pub(crate) fn compile_letpattern_unsupported(
        &mut self,
        _pattern: &Box<Pattern>,
        _value: &Expr,
        span: crate::lexer::Span,
        _mutable: bool,
    ) -> Result<(), FerriError> {
        Err(self.not_yet_supported("Complex destructure patterns", span))
    }
}
