use crate::ast::*;
use crate::env::{Env, Environment};
use crate::errors::FerriError;
use crate::types::Value;

use super::super::Interpreter;

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
                condition, body, ..
            } => {
                loop {
                    let cond = self.eval_expr(condition, env)?;
                    if !cond.is_truthy() {
                        break;
                    }
                    match self.eval_block(body, env) {
                        Ok(_) => {}
                        Err(FerriError::Break(val)) => {
                            return Ok(val.map(|v| *v).unwrap_or(Value::Unit))
                        }
                        Err(FerriError::Continue) => continue,
                        Err(e) => return Err(e),
                    }
                }
                Ok(Value::Unit)
            }
            Stmt::Loop { body, .. } => loop {
                match self.eval_block(body, env) {
                    Ok(_) => {}
                    Err(FerriError::Break(val)) => {
                        return Ok(val.map(|v| *v).unwrap_or(Value::Unit))
                    }
                    Err(FerriError::Continue) => continue,
                    Err(e) => return Err(e),
                }
            },
            Stmt::For {
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
                        Err(FerriError::Break(val)) => {
                            return Ok(val.map(|v| *v).unwrap_or(Value::Unit))
                        }
                        Err(FerriError::Continue) => continue,
                        Err(e) => return Err(e),
                    }
                }
                Ok(Value::Unit)
            }
            Stmt::Break { value, .. } => {
                let val = if let Some(expr) = value {
                    Some(Box::new(self.eval_expr(expr, env)?))
                } else {
                    None
                };
                Err(FerriError::Break(val))
            }
            Stmt::Continue { .. } => Err(FerriError::Continue),

            Stmt::WhileLet {
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
                        Err(FerriError::Break(v)) => {
                            result = v.map(|v| *v).unwrap_or(Value::Unit);
                            break;
                        }
                        Err(FerriError::Continue) => continue,
                        Err(e) => return Err(e),
                    }
                }
                Ok(result)
            }
            Stmt::ForDestructure {
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
                        Err(FerriError::Break(val)) => {
                            return Ok(val.map(|v| *v).unwrap_or(Value::Unit))
                        }
                        Err(FerriError::Continue) => continue,
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
