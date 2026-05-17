use crate::ast::*;
use crate::env::Env;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

use super::super::Interpreter;

impl Interpreter {
    pub(crate) fn eval_assign_expr(
        &mut self,
        target: &Expr,
        value: &Expr,
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        let val = self.eval_expr(value, env)?;
        if let Expr::Ident(name, _) = target {
            env.borrow_mut()
                .set(name, val)
                .map_err(|e| FerriError::Runtime {
                    message: e.to_string(),
                    line: span.line,
                    column: span.column,
                })?;
            Ok(Value::Unit)
        } else if let Expr::FieldAccess { object, field, .. } = target {
            if let Expr::Ident(name, _) = object.as_ref() {
                let mut current = env.borrow().get(name).map_err(|_| FerriError::Runtime {
                    message: format!("undefined variable '{name}'"),
                    line: span.line,
                    column: span.column,
                })?;
                if let Value::Struct { fields, .. } = &mut current {
                    if fields.contains_key(field) {
                        fields.insert(field.clone(), val);
                    } else {
                        return Err(FerriError::Runtime {
                            message: format!("no field '{field}' on struct"),
                            line: span.line,
                            column: span.column,
                        });
                    }
                } else {
                    return Err(FerriError::Runtime {
                        message: format!("cannot set field on {}", current.type_name()),
                        line: span.line,
                        column: span.column,
                    });
                }
                env.borrow_mut()
                    .set(name, current)
                    .map_err(|e| FerriError::Runtime {
                        message: e.to_string(),
                        line: span.line,
                        column: span.column,
                    })?;
                Ok(Value::Unit)
            } else if let Expr::SelfRef(_) = object.as_ref() {
                let mut current = env.borrow().get("self").map_err(|_| FerriError::Runtime {
                    message: "'self' not available in this context".into(),
                    line: span.line,
                    column: span.column,
                })?;
                if let Value::Struct { fields, .. } = &mut current {
                    if fields.contains_key(field) {
                        fields.insert(field.clone(), val);
                    } else {
                        return Err(FerriError::Runtime {
                            message: format!("no field '{field}' on struct"),
                            line: span.line,
                            column: span.column,
                        });
                    }
                } else {
                    return Err(FerriError::Runtime {
                        message: format!("cannot set field on {}", current.type_name()),
                        line: span.line,
                        column: span.column,
                    });
                }
                env.borrow_mut()
                    .set("self", current)
                    .map_err(|e| FerriError::Runtime {
                        message: e.to_string(),
                        line: span.line,
                        column: span.column,
                    })?;
                Ok(Value::Unit)
            } else {
                Err(FerriError::Runtime {
                    message: "invalid field assignment target".into(),
                    line: span.line,
                    column: span.column,
                })
            }
        } else if let Expr::Index { object, index, .. } = target {
            let idx = self.eval_expr(index, env)?;
            let Value::Integer(i) = idx else {
                return Err(FerriError::Runtime {
                    message: format!("index must be integer, got {}", idx.type_name()),
                    line: span.line,
                    column: span.column,
                });
            };
            let i = i as usize;
            // Evaluate the indexed collection (handles both `v[i]` and `v[i][j]`)
            let collection = self.eval_expr(object, env)?;
            match collection {
                Value::Vec(rc) => {
                    let v = rc.borrow();
                    if i >= v.len() {
                        return Err(FerriError::Runtime {
                            message: format!(
                                "index out of bounds: len is {}, but index is {i}",
                                v.len()
                            ),
                            line: span.line,
                            column: span.column,
                        });
                    }
                    drop(v);
                    rc.borrow_mut()[i] = val;
                }
                _ => {
                    return Err(FerriError::Runtime {
                        message: format!("cannot index-assign into {}", collection.type_name()),
                        line: span.line,
                        column: span.column,
                    });
                }
            }
            Ok(Value::Unit)
        } else {
            Err(FerriError::Runtime {
                message: "invalid assignment target".into(),
                line: span.line,
                column: span.column,
            })
        }
    }

    pub(crate) fn eval_compound_assign_expr(
        &mut self,
        target: &Expr,
        op: BinOp,
        value: &Expr,
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        if let Expr::Ident(name, _) = target {
            let current = env.borrow().get(name).map_err(|_| FerriError::Runtime {
                message: format!("undefined variable '{name}'"),
                line: span.line,
                column: span.column,
            })?;
            let rval = self.eval_expr(value, env)?;
            let new_val = self.eval_binary_op(&current, op, &rval, span.line, span.column)?;
            env.borrow_mut()
                .set(name, new_val)
                .map_err(|e| FerriError::Runtime {
                    message: e.to_string(),
                    line: span.line,
                    column: span.column,
                })?;
            Ok(Value::Unit)
        } else {
            Err(FerriError::Runtime {
                message: "invalid compound assignment target".into(),
                line: span.line,
                column: span.column,
            })
        }
    }
}
