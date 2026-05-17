use crate::ast::*;
use crate::env::{Env, Environment};
use crate::errors::FerriError;
use crate::types::Value;

use super::super::Interpreter;

/// Returns true if a `Break` error with the given label should be caught by a loop
/// with `loop_label`. Unlabeled break always caught by innermost loop; labeled break
/// only caught by the loop whose label matches.
fn should_catch_break(err_label: &Option<String>, loop_label: &Option<String>) -> bool {
    match (err_label, loop_label) {
        (None, _) => true,
        (Some(bl), Some(ll)) => bl == ll,
        _ => false,
    }
}

/// Returns true if a `Continue` error with the given label should be caught by a loop
/// with `loop_label`. Same logic as `should_catch_break`.
fn should_catch_continue(err_label: &Option<String>, loop_label: &Option<String>) -> bool {
    match (err_label, loop_label) {
        (None, _) => true,
        (Some(cl), Some(ll)) => cl == ll,
        _ => false,
    }
}

impl Interpreter {
    pub(crate) fn eval_stmt(&mut self, stmt: &Stmt, env: &Env) -> Result<Value, FerriError> {
        match stmt {
            Stmt::Let {
                name,
                mutable,
                value,
                ..
            } => {
                let val = if let Some(expr) = value {
                    self.eval_expr(expr, env)?
                } else {
                    Value::Unit
                };
                env.borrow_mut().define(name.clone(), val, *mutable);
                Ok(Value::Unit)
            }
            Stmt::Expr { expr, .. } => self.eval_expr(expr, env),
            Stmt::Return { value, .. } => {
                let val = if let Some(expr) = value {
                    self.eval_expr(expr, env)?
                } else {
                    Value::Unit
                };
                Err(FerriError::Return(Box::new(val)))
            }
            Stmt::While {
                label,
                condition,
                body,
                ..
            } => {
                loop {
                    let cond = self.eval_expr(condition, env)?;
                    if !cond.is_truthy() {
                        break;
                    }
                    match self.eval_block(body, env) {
                        Ok(_) => {}
                        Err(FerriError::Break(l, val)) if should_catch_break(&l, label) => {
                            return Ok(val.map(|v| *v).unwrap_or(Value::Unit));
                        }
                        Err(FerriError::Continue(l)) if should_catch_continue(&l, label) => {
                            continue;
                        }
                        Err(e) => return Err(e),
                    }
                }
                Ok(Value::Unit)
            }
            Stmt::Loop { label, body, .. } => loop {
                match self.eval_block(body, env) {
                    Ok(_) => {}
                    Err(FerriError::Break(l, val)) if should_catch_break(&l, label) => {
                        return Ok(val.map(|v| *v).unwrap_or(Value::Unit));
                    }
                    Err(FerriError::Continue(l)) if should_catch_continue(&l, label) => {
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            },
            Stmt::For {
                label,
                name,
                iterable,
                body,
                ..
            } => {
                let iter_val = self.eval_expr(iterable, env)?;
                let values = self.value_to_iter(&iter_val, iterable.span())?;
                let for_env = Environment::child(env);
                for_env.borrow_mut().define(name.clone(), Value::Unit, true);
                for val in values {
                    for_env.borrow_mut().set(name, val).ok();
                    match self.eval_block(body, &for_env) {
                        Ok(_) => {}
                        Err(FerriError::Break(l, val)) if should_catch_break(&l, label) => {
                            return Ok(val.map(|v| *v).unwrap_or(Value::Unit));
                        }
                        Err(FerriError::Continue(l)) if should_catch_continue(&l, label) => {
                            continue;
                        }
                        Err(e) => return Err(e),
                    }
                }
                Ok(Value::Unit)
            }
            Stmt::Break { label, value, .. } => {
                let val = if let Some(expr) = value {
                    Some(Box::new(self.eval_expr(expr, env)?))
                } else {
                    None
                };
                Err(FerriError::Break(label.clone(), val))
            }
            Stmt::Continue { label, .. } => Err(FerriError::Continue(label.clone())),

            Stmt::WhileLet {
                label,
                pattern,
                expr,
                body,
                ..
            } => {
                let mut result = Value::Unit;
                loop {
                    let val = self.eval_expr(expr, env)?;
                    if !Self::pattern_matches(pattern, &val) {
                        break;
                    }
                    let iter_env = Environment::child(env);
                    Self::bind_pattern(pattern, &val, &iter_env, false);
                    match self.eval_block(body, &iter_env) {
                        Ok(v) => result = v,
                        Err(FerriError::Break(l, v)) if should_catch_break(&l, label) => {
                            result = v.map(|v| *v).unwrap_or(Value::Unit);
                            break;
                        }
                        Err(FerriError::Continue(l)) if should_catch_continue(&l, label) => {
                            continue;
                        }
                        Err(e) => return Err(e),
                    }
                }
                Ok(result)
            }
            Stmt::ForDestructure {
                label,
                names,
                iterable,
                body,
                ..
            } => {
                let iter_val = self.eval_expr(iterable, env)?;
                let values = self.value_to_iter(&iter_val, iterable.span())?;
                let for_env = Environment::child(env);
                for name in names {
                    if name != "_" {
                        for_env.borrow_mut().define(name.clone(), Value::Unit, true);
                    }
                }
                for val in values {
                    if let Value::Tuple(ref elems) = val {
                        for (i, name) in names.iter().enumerate() {
                            if name == "_" {
                                continue;
                            }
                            let v = elems.get(i).cloned().unwrap_or(Value::Unit);
                            for_env.borrow_mut().set(name, v).ok();
                        }
                    } else if names.len() == 1 {
                        for_env.borrow_mut().set(&names[0], val).ok();
                    }
                    match self.eval_block(body, &for_env) {
                        Ok(_) => {}
                        Err(FerriError::Break(l, val)) if should_catch_break(&l, label) => {
                            return Ok(val.map(|v| *v).unwrap_or(Value::Unit));
                        }
                        Err(FerriError::Continue(l)) if should_catch_continue(&l, label) => {
                            continue;
                        }
                        Err(e) => return Err(e),
                    }
                }
                Ok(Value::Unit)
            }
            Stmt::LetPattern {
                pattern,
                mutable,
                value,
                span,
            } => {
                let val = self.eval_expr(value, env)?;
                if !Self::pattern_matches(pattern, &val) {
                    return Err(FerriError::Runtime {
                        message: "destructuring pattern does not match value".into(),
                        line: span.line,
                        column: span.column,
                    });
                }
                Self::bind_pattern(pattern, &val, env, *mutable);
                Ok(Value::Unit)
            }
        }
    }
}
