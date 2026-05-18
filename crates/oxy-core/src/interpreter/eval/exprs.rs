use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::*;
use crate::env::{Env, Environment};
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

use super::super::Interpreter;

impl Interpreter {
    pub(crate) fn eval_match_expr(
        &mut self,
        expr: &Expr,
        arms: &[MatchArm],
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        let val = self.eval_expr(expr, env)?;
        for arm in arms {
            if Self::pattern_matches(&arm.pattern, &val) {
                let match_env = Environment::child(env);
                Self::bind_pattern(&arm.pattern, &val, &match_env, false);
                if let Some(guard) = &arm.guard {
                    let guard_val = self.eval_expr(guard, &match_env)?;
                    if guard_val != Value::Bool(true) {
                        continue;
                    }
                }
                return self.eval_expr(&arm.body, &match_env);
            }
        }
        Err(FerriError::Runtime {
            message: "non-exhaustive match: no arm matched".into(),
            line: span.line,
            column: span.column,
        })
    }

    pub(crate) fn eval_range_expr(
        &mut self,
        start: &Option<Box<Expr>>,
        end: &Option<Box<Expr>>,
        inclusive: bool,
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        let start_val = match start {
            Some(s) => Some(self.eval_expr(s, env)?),
            None => None,
        };
        let end_val = match end {
            Some(e) => Some(self.eval_expr(e, env)?),
            None => None,
        };
        match (&start_val, &end_val) {
            (Some(Value::Integer(s)), Some(Value::Integer(e))) => {
                let end_n = if inclusive { *e + 1 } else { *e };
                Ok(Value::Range(*s, end_n))
            }
            (None, _) | (_, None) => {
                let s = match &start_val {
                    Some(Value::Integer(i)) => *i,
                    None => i64::MIN,
                    _ => {
                        return Err(FerriError::Runtime {
                            message: format!(
                                "range bound must be integer, got {}",
                                start_val.unwrap().type_name()
                            ),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                let e = match &end_val {
                    Some(Value::Integer(i)) => {
                        if inclusive {
                            *i + 1
                        } else {
                            *i
                        }
                    }
                    None => i64::MAX,
                    _ => {
                        return Err(FerriError::Runtime {
                            message: format!(
                                "range bound must be integer, got {}",
                                end_val.unwrap().type_name()
                            ),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                Ok(Value::Range(s, e))
            }
            _ => Err(FerriError::Runtime {
                message: format!(
                    "range bounds must be integers, got {} and {}",
                    start_val
                        .map(|v| v.type_name())
                        .unwrap_or_else(|| "none".to_string()),
                    end_val
                        .map(|v| v.type_name())
                        .unwrap_or_else(|| "none".to_string())
                ),
                line: span.line,
                column: span.column,
            }),
        }
    }

    pub(crate) fn eval_index_expr(
        &mut self,
        object: &Expr,
        index: &Expr,
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        let obj = self.eval_expr(object, env)?;
        let idx = self.eval_expr(index, env)?;
        match (&obj, &idx) {
            (Value::Vec(rc), Value::Integer(i)) => {
                let i = *i as usize;
                let v = rc.borrow();
                v.get(i).cloned().ok_or_else(|| FerriError::Runtime {
                    message: format!("index out of bounds: len is {}, but index is {i}", v.len()),
                    line: span.line,
                    column: span.column,
                })
            }
            (Value::String(s), Value::Integer(i)) => {
                let i = *i as usize;
                s.chars()
                    .nth(i)
                    .map(Value::Char)
                    .ok_or_else(|| FerriError::Runtime {
                        message: format!(
                            "index out of bounds: len is {}, but index is {i}",
                            s.len()
                        ),
                        line: span.line,
                        column: span.column,
                    })
            }
            (Value::Tuple(t), Value::Integer(i)) => {
                let i = *i as usize;
                t.get(i).cloned().ok_or_else(|| FerriError::Runtime {
                    message: format!("index out of bounds: len is {}, but index is {i}", t.len()),
                    line: span.line,
                    column: span.column,
                })
            }
            (Value::Vec(rc), Value::Range(start, end)) => {
                let v = rc.borrow();
                let len = v.len() as i64;
                let s = if *start == i64::MIN { 0 } else { *start };
                let e = if *end == i64::MAX { len } else { *end };
                let s = s.max(0) as usize;
                let e = (e.min(len)) as usize;
                Ok(Value::Vec(Rc::new(RefCell::new(v[s..e].to_vec()))))
            }
            (Value::HashMap(rc), k) => rc.borrow().get(k).cloned().ok_or_else(|| FerriError::Runtime {
                message: format!("key not found: \"{k}\""),
                line: span.line,
                column: span.column,
            }),
            (Value::String(s), Value::Range(start, end)) => {
                let chars: Vec<char> = s.chars().collect();
                let len = chars.len() as i64;
                let st = if *start == i64::MIN { 0 } else { *start };
                let en = if *end == i64::MAX { len } else { *end };
                let st = st.max(0) as usize;
                let en = (en.min(len)) as usize;
                Ok(Value::String(chars[st..en].iter().collect()))
            }
            _ => Err(FerriError::Runtime {
                message: format!("cannot index {} with {}", obj.type_name(), idx.type_name()),
                line: span.line,
                column: span.column,
            }),
        }
    }

    pub(crate) fn eval_field_access_expr(
        &mut self,
        object: &Expr,
        field: &str,
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        let obj = self.eval_expr(object, env)?;
        if let Ok(idx) = field.parse::<usize>() {
            match &obj {
                Value::Tuple(t) => t.get(idx).cloned().ok_or_else(|| FerriError::Runtime {
                    message: format!(
                        "index out of bounds: len is {}, but index is {idx}",
                        t.len()
                    ),
                    line: span.line,
                    column: span.column,
                }),
                _ => Err(FerriError::Runtime {
                    message: format!("cannot access field `.{field}` on {}", obj.type_name()),
                    line: span.line,
                    column: span.column,
                }),
            }
        } else if let Value::Struct { fields, .. } = &obj {
            fields
                .get(field)
                .cloned()
                .ok_or_else(|| FerriError::Runtime {
                    message: format!("no field `{field}` on struct {}", obj.type_name()),
                    line: span.line,
                    column: span.column,
                })
        } else {
            Err(FerriError::Runtime {
                message: format!("cannot access field `.{field}` on {}", obj.type_name()),
                line: span.line,
                column: span.column,
            })
        }
    }

    pub(crate) fn eval_struct_init_expr(
        &mut self,
        name: &str,
        fields: &[(String, Expr)],
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        let resolved_name = if name == "Self" {
            self.current_self_type
                .clone()
                .unwrap_or_else(|| name.to_string())
        } else {
            self.resolve_type_alias(name)
        };
        let mut field_map = HashMap::new();
        for (fname, fexpr) in fields {
            let val = self.eval_expr(fexpr, env)?;
            field_map.insert(fname.clone(), val);
        }
        if let Some(sdef) = self.struct_defs.get(&resolved_name) {
            if let StructKind::Named(def_fields) = &sdef.kind {
                for df in def_fields {
                    if !field_map.contains_key(&df.name) {
                        return Err(FerriError::Runtime {
                            message: format!(
                                "missing field `{}` in initializer of `{resolved_name}`",
                                df.name
                            ),
                            line: span.line,
                            column: span.column,
                        });
                    }
                }
            }
        }
        Ok(Value::Struct {
            name: resolved_name,
            fields: field_map,
        })
    }

    pub(crate) fn eval_if_let_expr(
        &mut self,
        pattern: &Pattern,
        expr: &Expr,
        then_block: &Block,
        else_block: &Option<Box<Expr>>,
        env: &Env,
    ) -> Result<Value, FerriError> {
        let val = self.eval_expr(expr, env)?;
        if Self::pattern_matches(pattern, &val) {
            let child_env = Environment::child(env);
            Self::bind_pattern(pattern, &val, &child_env, false);
            self.eval_block(then_block, &child_env)
        } else if let Some(else_expr) = else_block {
            self.eval_expr(else_expr, env)
        } else {
            Ok(Value::Unit)
        }
    }

    /// Evaluate a type cast expression: `expr as Type`.
    pub(crate) fn eval_as_expr(
        &mut self,
        expr: &Expr,
        type_name: &str,
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        let val = self.eval_expr(expr, env)?;
        match type_name {
            "i64" | "usize" => match &val {
                Value::Integer(_) => Ok(val),
                Value::Float(f) => Ok(Value::Integer(*f as i64)),
                Value::Char(c) => Ok(Value::Integer(*c as i64)),
                Value::Bool(b) => Ok(Value::Integer(*b as i64)),
                other => Err(FerriError::Runtime {
                    message: format!("cannot cast {} to i64", other.type_name()),
                    line: span.line,
                    column: span.column,
                }),
            },
            "f64" => match &val {
                Value::Integer(n) => Ok(Value::Float(*n as f64)),
                Value::Float(_) => Ok(val),
                other => Err(FerriError::Runtime {
                    message: format!("cannot cast {} to f64", other.type_name()),
                    line: span.line,
                    column: span.column,
                }),
            },
            "char" => match &val {
                Value::Integer(n) => {
                    if let Some(c) = char::from_u32(*n as u32) {
                        Ok(Value::Char(c))
                    } else {
                        Err(FerriError::Runtime {
                            message: format!("invalid char code {n}"),
                            line: span.line,
                            column: span.column,
                        })
                    }
                }
                Value::Char(_) => Ok(val),
                other => Err(FerriError::Runtime {
                    message: format!("cannot cast {} to char", other.type_name()),
                    line: span.line,
                    column: span.column,
                }),
            },
            "bool" => match &val {
                Value::Bool(_) => Ok(val),
                Value::Integer(n) => Ok(Value::Bool(*n != 0)),
                other => Err(FerriError::Runtime {
                    message: format!("cannot cast {} to bool", other.type_name()),
                    line: span.line,
                    column: span.column,
                }),
            },
            _ => Err(FerriError::Runtime {
                message: format!("unknown type for cast: `{type_name}`"),
                line: span.line,
                column: span.column,
            }),
        }
    }

    pub(crate) fn eval_try_expr(
        &mut self,
        expr: &Expr,
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        use crate::types::{
            ERR_VARIANT, NONE_VARIANT, OK_VARIANT, OPTION_TYPE, RESULT_TYPE, SOME_VARIANT,
        };

        let val = self.eval_expr(expr, env)?;
        match &val {
            Value::EnumVariant {
                enum_name,
                variant,
                data,
                ..
            } if enum_name == OPTION_TYPE && variant == SOME_VARIANT => {
                Ok(data.first().cloned().unwrap_or(Value::Unit))
            }
            Value::EnumVariant {
                enum_name, variant, ..
            } if enum_name == OPTION_TYPE && variant == NONE_VARIANT => {
                Err(FerriError::Return(Box::new(val)))
            }
            Value::EnumVariant {
                enum_name,
                variant,
                data,
                ..
            } if enum_name == RESULT_TYPE && variant == OK_VARIANT => {
                Ok(data.first().cloned().unwrap_or(Value::Unit))
            }
            Value::EnumVariant {
                enum_name, variant, ..
            } if enum_name == RESULT_TYPE && variant == ERR_VARIANT => {
                Err(FerriError::Return(Box::new(val)))
            }
            _ => Err(FerriError::Runtime {
                message: format!(
                    "`?` operator can only be used on Option or Result, got {}",
                    val.type_name()
                ),
                line: span.line,
                column: span.column,
            }),
        }
    }

    pub(crate) fn eval_closure_expr(
        &mut self,
        params: &[ClosureParam],
        return_type: &Option<TypeAnnotation>,
        body: &Expr,
        env: &Env,
    ) -> Result<Value, FerriError> {
        let fn_params: Vec<Param> = params
            .iter()
            .map(|cp| Param {
                name: cp.name.clone(),
                type_ann: cp.type_ann.clone().unwrap_or(TypeAnnotation {
                    name: "_".to_string(),
                    span: cp.span,
                }),
                span: cp.span,
            })
            .collect();

        let closure_body = match body {
            Expr::Block(block) => block.clone(),
            expr => Block {
                stmts: vec![Stmt::Expr {
                    expr: expr.clone(),
                    has_semicolon: false,
                }],
                span: expr.span(),
            },
        };

        Ok(Value::Function(Box::new(crate::types::FunctionData {
            name: "<closure>".to_string(),
            params: fn_params,
            return_type: return_type.clone(),
            body: closure_body,
            closure_env: env.clone(),
            target_ip: None,
                            captured_slots: vec![],
        })))
    }

    pub(crate) fn eval_await_expr(&mut self, expr: &Expr, env: &Env) -> Result<Value, FerriError> {
        let val = self.eval_expr(expr, env)?;
        match val {
            Value::Future(future) => {
                let call_env = Environment::child(&future.closure_env);
                for (param, arg) in future.params.iter().zip(future.args.iter()) {
                    call_env
                        .borrow_mut()
                        .define(param.name.clone(), arg.clone(), true);
                }
                match self.eval_block(&future.body, &call_env) {
                    Ok(val) => Ok(val),
                    Err(FerriError::Return(val)) => Ok(*val),
                    Err(e) => Err(e),
                }
            }
            Value::JoinHandle(val) => Ok(*val),
            other => Ok(other),
        }
    }

    pub(crate) fn eval_fstring_expr(
        &mut self,
        parts: &[FStringPart],
        env: &Env,
    ) -> Result<Value, FerriError> {
        let mut result = String::new();
        for part in parts {
            match part {
                FStringPart::Literal(s) => result.push_str(s),
                FStringPart::Expr(expr) => {
                    let val = self.eval_expr(expr, env)?;
                    result.push_str(&val.to_string());
                }
            }
        }
        Ok(Value::String(result))
    }
}
