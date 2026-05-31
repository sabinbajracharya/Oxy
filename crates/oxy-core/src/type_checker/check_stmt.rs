use super::*;

impl TypeChecker {
    pub(super) fn check_stmt(
        &mut self,
        stmt: &Stmt,
        fn_ret: &TypeInfo,
    ) -> Result<(), PipelineError> {
        match stmt {
            Stmt::Let {
                name,
                mutable,
                type_ann,
                value,
                span,
            } => {
                let declared = if let Some(ann) = type_ann {
                    let ty = self.resolve_annotation(ann);
                    self.validate_type_known(&ty, ann.span())?;
                    ty
                } else {
                    TypeInfo::Unknown
                };
                let inferred = if let Some(expr) = value {
                    if declared != TypeInfo::Unknown {
                        self.infer_expr_expected(expr, Some(&declared))?
                    } else {
                        self.infer_expr(expr)?
                    }
                } else {
                    TypeInfo::Unknown
                };
                if !declared.accepts(&inferred) {
                    return Err(PipelineError::TypeError {
                        message: format!(
                            "type mismatch: variable `{name}` declared as `{}`, but value has type `{}`",
                            declared.display_name(), inferred.display_name()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
                let stored_ty = if declared != TypeInfo::Unknown {
                    declared
                } else {
                    inferred
                };
                self.env.borrow_mut().define_mut(name, stored_ty, *mutable);
                Ok(())
            }
            Stmt::Expr {
                expr,
                has_semicolon,
            } => {
                // Tail expression without semicolon is an implicit return — check type.
                // Skip check if inferred as Unit (control-flow expressions with explicit
                // returns, e.g. `if x > 0 { return x; }`).
                if !has_semicolon && *fn_ret != TypeInfo::Unknown {
                    let inferred = self.infer_expr_expected(expr, Some(fn_ret))?;
                    if inferred != TypeInfo::Unit && !fn_ret.accepts(&inferred) {
                        let span = expr.span();
                        return Err(PipelineError::TypeError {
                            message: format!(
                                "type mismatch: function returns `{}`, but tail expression has type `{}`",
                                fn_ret.name(), inferred.name()
                            ),
                            line: span.line,
                            column: span.column,
                        });
                    }
                }
                // Check if the inner expression is an if/if-let (they only exist as Expr)
                if let Expr::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } = expr
                {
                    self.infer_expr(condition)?;
                    let block_env = TypeEnv::child(&self.env);
                    let saved = self.env.clone();
                    self.env = block_env;
                    for s in &then_block.stmts {
                        self.check_stmt(s, fn_ret)?;
                    }
                    self.env = saved;
                    if let Some(else_expr) = else_block {
                        self.infer_expr(else_expr)?;
                    }
                } else if let Expr::IfLet {
                    pattern,
                    expr: inner,
                    guard,
                    then_block,
                    else_block,
                    ..
                } = expr
                {
                    let _ = self.infer_expr(inner)?;
                    let block_env = TypeEnv::child(&self.env);
                    let saved = self.env.clone();
                    self.env = block_env;
                    self.bind_pattern(pattern, false);
                    if let Some(g) = guard {
                        let _ = self.infer_expr(g)?;
                    }
                    for s in &then_block.stmts {
                        self.check_stmt(s, fn_ret)?;
                    }
                    self.env = saved;
                    if let Some(else_expr) = else_block {
                        self.infer_expr(else_expr)?;
                    }
                } else {
                    self.infer_expr(expr)?;
                }
                Ok(())
            }
            Stmt::Return { value, span } => {
                let inferred = if let Some(expr) = value {
                    self.infer_expr(expr)?
                } else {
                    TypeInfo::Unit
                };
                if !fn_ret.accepts(&inferred) {
                    return Err(PipelineError::TypeError {
                        message: format!(
                            "type mismatch: function returns `{}`, but return expression has type `{}`",
                            fn_ret.name(), inferred.name()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
                Ok(())
            }
            Stmt::While {
                condition, body, ..
            } => {
                self.infer_expr(condition)?;
                self.loop_depth += 1;
                self.check_block(body, fn_ret)?;
                self.loop_depth -= 1;
                Ok(())
            }
            Stmt::Loop { body, .. } => {
                self.loop_depth += 1;
                self.check_block(body, fn_ret)?;
                self.loop_depth -= 1;
                Ok(())
            }
            Stmt::For {
                name,
                iterable,
                body,
                ..
            } => {
                let _ = self.infer_expr(iterable)?;
                let body_env = TypeEnv::child(&self.env);
                body_env.borrow_mut().define(name, TypeInfo::Unknown);
                let saved = self.env.clone();
                self.env = body_env;
                self.loop_depth += 1;
                self.check_block(body, fn_ret)?;
                self.loop_depth -= 1;
                self.env = saved;
                Ok(())
            }
            Stmt::WhileLet {
                pattern,
                expr: inner,
                body,
                ..
            } => {
                let _ = self.infer_expr(inner)?;
                let body_env = TypeEnv::child(&self.env);
                let saved = self.env.clone();
                self.env = body_env;
                self.bind_pattern(pattern, false);
                self.loop_depth += 1;
                let result = (|| -> Result<(), PipelineError> {
                    for s in &body.stmts {
                        self.check_stmt(s, fn_ret)?;
                    }
                    Ok(())
                })();
                self.loop_depth -= 1;
                self.env = saved;
                result
            }
            Stmt::ForDestructure {
                names,
                iterable,
                body,
                ..
            } => {
                let _ = self.infer_expr(iterable)?;
                let body_env = TypeEnv::child(&self.env);
                for name in names {
                    body_env.borrow_mut().define(name, TypeInfo::Unknown);
                }
                let saved = self.env.clone();
                self.env = body_env;
                self.loop_depth += 1;
                self.check_block(body, fn_ret)?;
                self.loop_depth -= 1;
                self.env = saved;
                Ok(())
            }
            Stmt::LetPattern {
                pattern,
                mutable,
                value,
                ..
            } => {
                self.infer_expr(value)?;
                self.bind_pattern(pattern, *mutable);
                Ok(())
            }
            Stmt::Break { span, .. } => {
                if self.loop_depth == 0 {
                    return Err(PipelineError::TypeError {
                        message: "break outside of loop".into(),
                        line: span.line,
                        column: span.column,
                    });
                }
                Ok(())
            }
            Stmt::Continue { span, .. } => {
                if self.loop_depth == 0 {
                    return Err(PipelineError::TypeError {
                        message: "continue outside of loop".into(),
                        line: span.line,
                        column: span.column,
                    });
                }
                Ok(())
            }
            // Nested items are hoisted to top-level by the parser when they
            // appear inside a fn body; a Stmt::Item only survives in unusual
            // call paths (e.g. tests that invoke parse_stmt directly). Skip
            // it here — the hoisted copy is type-checked at the program level.
            Stmt::Item(_) => Ok(()),
            Stmt::Use(use_def) => self.process_use_def(use_def),
        }
    }

    /// Define every variable a pattern introduces into the current scope.
    /// Types are left `Unknown` (pattern-level type inference is not modelled);
    /// the point is that these names resolve rather than tripping the
    /// undefined-variable check. `mutable` propagates from `let mut (a, b)`.
    pub(super) fn bind_pattern(&self, pattern: &Pattern, mutable: bool) {
        match pattern {
            Pattern::Ident(name, _) => {
                self.env
                    .borrow_mut()
                    .define_mut(name, TypeInfo::Unknown, mutable);
            }
            Pattern::Tuple(pats, _) | Pattern::Slice(pats, _) | Pattern::Or(pats, _) => {
                for p in pats {
                    self.bind_pattern(p, mutable);
                }
            }
            Pattern::EnumVariant { fields, .. } => {
                for p in fields {
                    self.bind_pattern(p, mutable);
                }
            }
            Pattern::Struct { fields, .. } => {
                for (_, p) in fields {
                    self.bind_pattern(p, mutable);
                }
            }
            Pattern::Literal(_)
            | Pattern::Wildcard(_)
            | Pattern::Rest(_)
            | Pattern::Range { .. } => {}
        }
    }

    /// Check a sequence of statements, rejecting any statement that follows an
    /// unconditional exit (return/break/continue/panic).
    pub(super) fn check_stmt_seq(
        &mut self,
        stmts: &[Stmt],
        fn_ret: &TypeInfo,
    ) -> Result<(), PipelineError> {
        let mut iter = stmts.iter().peekable();
        while let Some(stmt) = iter.next() {
            self.check_stmt(stmt, fn_ret)?;
            if stmt_always_terminates(stmt) {
                if let Some(next) = iter.peek() {
                    let span = next.span();
                    return Err(PipelineError::TypeError {
                        message: "unreachable code".to_string(),
                        line: span.line,
                        column: span.column,
                    });
                }
            }
        }
        Ok(())
    }

    pub(super) fn check_block(
        &mut self,
        block: &Block,
        fn_ret: &TypeInfo,
    ) -> Result<(), PipelineError> {
        let block_env = TypeEnv::child(&self.env);
        let saved = self.env.clone();
        self.env = block_env;
        let result = self.check_stmt_seq(&block.stmts, fn_ret);
        self.env = saved;
        result
    }
}

/// Returns true if a statement unconditionally transfers control out of the
/// current block (return, break, continue, or a direct panic!() call).
fn stmt_always_terminates(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => true,
        Stmt::Expr { expr, .. } => expr_always_terminates(expr),
        _ => false,
    }
}

fn expr_always_terminates(expr: &Expr) -> bool {
    matches!(&expr, Expr::Call { callee, .. } if matches!(callee.as_ref(), Expr::Ident(name, _) if name == "panic"))
}
