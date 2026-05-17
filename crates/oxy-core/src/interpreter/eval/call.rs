use std::rc::Rc;

use crate::ast::*;
use crate::env::{Env, Environment};
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::{FutureData, Value, ERR_VARIANT, OK_VARIANT, SOME_VARIANT};

use super::super::Interpreter;

impl Interpreter {
    pub(crate) fn try_builtin_call(
        &mut self,
        name: &str,
        args: &[Expr],
        span: &Span,
        env: &Env,
    ) -> Result<Option<Value>, FerriError> {
        match name {
            SOME_VARIANT => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Some() takes exactly 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let val = self.eval_expr(&args[0], env)?;
                Ok(Some(Value::some(val)))
            }
            OK_VARIANT => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Ok() takes exactly 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let val = self.eval_expr(&args[0], env)?;
                Ok(Some(Value::ok(val)))
            }
            ERR_VARIANT => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Err() takes exactly 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let val = self.eval_expr(&args[0], env)?;
                Ok(Some(Value::err(val)))
            }
            "spawn" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("spawn() takes exactly 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let func = self.eval_expr(&args[0], env)?;
                let result = self.call_function(&func, &[], span.line, span.column)?;
                Ok(Some(Value::JoinHandle(Box::new(result))))
            }
            "sleep" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("sleep() takes exactly 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let val = self.eval_expr(&args[0], env)?;
                if let Value::Integer(ms) = val {
                    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
                    return Ok(Some(Value::Unit));
                }
                Err(FerriError::Runtime {
                    message: format!(
                        "sleep() expects integer milliseconds, got {}",
                        val.type_name()
                    ),
                    line: span.line,
                    column: span.column,
                })
            }
            _ => Ok(None),
        }
    }

    pub(crate) fn eval_call_expr(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        if let Expr::Ident(name, _) = callee {
            if let Some(result) = self.try_builtin_call(name, args, span, env)? {
                return Ok(result);
            }
        }
        let func = self.eval_expr(callee, env)?;
        let mut arg_values = Vec::with_capacity(args.len());
        for arg in args {
            arg_values.push(self.eval_expr(arg, env)?);
        }
        if let Value::Function(ref func_data) = func {
            if self.async_fns.contains(&func_data.name) {
                return Ok(Value::Future(Box::new(FutureData {
                    name: func_data.name.clone(),
                    params: func_data.params.clone(),
                    return_type: func_data.return_type.clone(),
                    body: func_data.body.clone(),
                    closure_env: Rc::clone(&func_data.closure_env),
                    args: arg_values,
                })));
            }
        }
        self.call_function(&func, &arg_values, span.line, span.column)
    }

    pub(crate) fn call_function(
        &mut self,
        func: &Value,
        args: &[Value],
        line: usize,
        col: usize,
    ) -> Result<Value, FerriError> {
        let Value::Function(func_data) = func else {
            return Err(FerriError::Runtime {
                message: format!("'{}' is not callable", func.type_name()),
                line,
                column: col,
            });
        };

        if args.len() != func_data.params.len() {
            return Err(FerriError::Runtime {
                message: format!(
                    "function '{}' expects {} argument(s), got {}",
                    func_data.name,
                    func_data.params.len(),
                    args.len()
                ),
                line,
                column: col,
            });
        }

        if self.call_stack.len() >= 1024 {
            return Err(FerriError::Runtime {
                message: "recursion limit exceeded (max depth 1024)".into(),
                line,
                column: col,
            });
        }

        self.call_stack.push(crate::errors::CallFrame {
            name: func_data.name.clone(),
            line,
            column: col,
        });

        let call_env = Environment::child(&func_data.closure_env);
        for (param, arg) in func_data.params.iter().zip(args.iter()) {
            call_env
                .borrow_mut()
                .define(param.name.clone(), arg.clone(), true);
        }

        match self.eval_block(&func_data.body, &call_env) {
            Ok(val) => {
                self.call_stack.pop();
                Ok(val)
            }
            Err(FerriError::Return(val)) => {
                self.call_stack.pop();
                Ok(*val)
            }
            Err(e) => Err(e),
        }
    }

    pub(crate) fn write_output(&mut self, s: &str) {
        if let Some(ref mut output) = self.output {
            if let Some(last) = output.last_mut() {
                if !last.ends_with('\n') {
                    last.push_str(s);
                    return;
                }
            }
            output.push(s.to_string());
        } else {
            print!("{s}");
        }
    }

    pub(crate) fn mutate_variable(
        &mut self,
        expr: &Expr,
        new_val: Value,
        env: &Env,
        span: &Span,
    ) -> Result<(), FerriError> {
        match expr {
            Expr::Ident(name, _) => env.borrow_mut().set(name, new_val).map_err(|e| match e {
                FerriError::Runtime { message, .. } => FerriError::Runtime {
                    message,
                    line: span.line,
                    column: span.column,
                },
                other => other,
            }),
            Expr::SelfRef(_) => env.borrow_mut().set("self", new_val).map_err(|e| match e {
                FerriError::Runtime { message, .. } => FerriError::Runtime {
                    message,
                    line: span.line,
                    column: span.column,
                },
                other => other,
            }),
            Expr::FieldAccess { object, field, .. } => {
                let obj = self.eval_expr(object, env)?;
                if let Value::Struct { name, mut fields } = obj {
                    fields.insert(field.clone(), new_val);
                    self.mutate_variable(object, Value::Struct { name, fields }, env, span)
                } else {
                    Err(FerriError::Runtime {
                        message: format!(
                            "cannot mutate field `{field}` on non-struct type {}",
                            obj.type_name()
                        ),
                        line: span.line,
                        column: span.column,
                    })
                }
            }
            Expr::Index { object, index, .. } => {
                let obj = self.eval_expr(object, env)?;
                let idx = self.eval_expr(index, env)?;
                match obj {
                    Value::Vec(mut v) => {
                        if let Value::Integer(i) = idx {
                            if i >= 0 && i < v.len() as i64 {
                                v[i as usize] = new_val;
                                self.mutate_variable(object, Value::Vec(v), env, span)
                            } else {
                                Err(FerriError::Runtime {
                                    message: format!("index {i} out of bounds"),
                                    line: span.line,
                                    column: span.column,
                                })
                            }
                        } else {
                            Err(FerriError::Runtime {
                                message: "index must be an integer".into(),
                                line: span.line,
                                column: span.column,
                            })
                        }
                    }
                    _ => Err(FerriError::Runtime {
                        message: "cannot index-assign non-array type".into(),
                        line: span.line,
                        column: span.column,
                    }),
                }
            }
            _ => Err(FerriError::Runtime {
                message: "cannot mutate non-variable receiver".into(),
                line: span.line,
                column: span.column,
            }),
        }
    }
}
