use super::*;

impl TypeChecker {
    pub(super) fn check_stmt(&mut self, stmt: &Stmt, fn_ret: &TypeInfo) -> Result<(), FerriError> {
        match stmt {
            Stmt::Let {
                name,
                type_ann,
                value,
                span,
                ..
            } => {
                let declared = if let Some(ann) = type_ann {
                    let ty = self.resolve_annotation(ann);
                    self.validate_type_known(&ty, ann.span())?;
                    ty
                } else {
                    TypeInfo::Unknown
                };
                let inferred = if let Some(expr) = value {
                    self.infer_expr(expr)?
                } else {
                    TypeInfo::Unit
                };
                if !declared.accepts(&inferred) {
                    return Err(FerriError::TypeError {
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
                self.env.borrow_mut().define(name, stored_ty);
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
                    let inferred = self.infer_expr(expr)?;
                    if inferred != TypeInfo::Unit && !fn_ret.accepts(&inferred) {
                        let span = expr.span();
                        return Err(FerriError::TypeError {
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
                    expr: inner,
                    guard,
                    then_block,
                    else_block,
                    ..
                } = expr
                {
                    let _ = self.infer_expr(inner)?;
                    if let Some(g) = guard {
                        let _ = self.infer_expr(g)?;
                    }
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
                    return Err(FerriError::TypeError {
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
                expr: inner, body, ..
            } => {
                let _ = self.infer_expr(inner)?;
                self.loop_depth += 1;
                self.check_block(body, fn_ret)?;
                self.loop_depth -= 1;
                Ok(())
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
            Stmt::LetPattern { value, .. } => {
                self.infer_expr(value)?;
                Ok(())
            }
            Stmt::Break { span, .. } => {
                if self.loop_depth == 0 {
                    return Err(FerriError::TypeError {
                        message: "break outside of loop".into(),
                        line: span.line,
                        column: span.column,
                    });
                }
                Ok(())
            }
            Stmt::Continue { span, .. } => {
                if self.loop_depth == 0 {
                    return Err(FerriError::TypeError {
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
            Stmt::Use(use_def) => {
                let base_path = use_def.path.join("::");
                self.check_path_visible(&base_path, use_def.span)?;
                match &use_def.tree {
                    UseTree::Simple(alias) => {
                        let local_name = alias
                            .as_ref()
                            .cloned()
                            .unwrap_or_else(|| use_def.path.last().cloned().unwrap_or_default());
                        self.use_aliases.insert(local_name, base_path.clone());
                    }
                    UseTree::Group(items) => {
                        for (name, alias) in items {
                            let local_name = alias.as_ref().unwrap_or(name);
                            let qualified = format!("{}::{}", base_path, name);
                            self.check_path_visible(&qualified, use_def.span)?;
                            self.use_aliases.insert(local_name.clone(), qualified);
                        }
                    }
                    UseTree::Glob => {
                        // Glob entries are resolved by the compiler
                    }
                }
                Ok(())
            }
        }
    }

    pub(super) fn check_block(
        &mut self,
        block: &Block,
        fn_ret: &TypeInfo,
    ) -> Result<(), FerriError> {
        let block_env = TypeEnv::child(&self.env);
        let saved = self.env.clone();
        self.env = block_env;
        for stmt in &block.stmts {
            self.check_stmt(stmt, fn_ret)?;
        }
        self.env = saved;
        Ok(())
    }
}
