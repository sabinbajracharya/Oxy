//! HashSet method implementations.
//!
//! Supports: len, is_empty, insert, remove, contains, union, intersection, difference, clone, to_vec.

use std::collections::HashSet;

use crate::ast::Expr;
use crate::env::Env;
use crate::errors::check_arg_count;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

use super::super::Interpreter;

impl Interpreter {
    /// Handle method calls on HashSet values.
    pub(crate) fn call_hashset_method(
        &mut self,
        receiver: Value,
        method: &str,
        args: Vec<Value>,
        receiver_expr: &Expr,
        env: &Env,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let Value::HashSet(s) = receiver else {
            unreachable!()
        };
        match method {
            "len" => Ok(Value::Integer(s.len() as i64)),
            "is_empty" => Ok(Value::Bool(s.is_empty())),
            "insert" => {
                check_arg_count("HashSet::insert", 1, &args, span)?;
                let val = args.into_iter().next().unwrap();
                let mut new_s = s;
                let was_new = new_s.insert(val);
                self.mutate_variable(receiver_expr, Value::HashSet(new_s), env, span)?;
                Ok(Value::Bool(was_new))
            }
            "remove" => {
                check_arg_count("HashSet::remove", 1, &args, span)?;
                let val = args.into_iter().next().unwrap();
                let mut new_s = s;
                let existed = new_s.remove(&val);
                self.mutate_variable(receiver_expr, Value::HashSet(new_s), env, span)?;
                Ok(Value::Bool(existed))
            }
            "contains" => {
                check_arg_count("HashSet::contains", 1, &args, span)?;
                Ok(Value::Bool(s.contains(&args[0])))
            }
            "union" => {
                check_arg_count("HashSet::union", 1, &args, span)?;
                let Value::HashSet(other) = &args[0] else {
                    return Err(FerriError::Runtime {
                        message: format!(
                            "HashSet::union expects a HashSet, got {}",
                            args[0].type_name()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                };
                let result: HashSet<Value> = s.union(other).cloned().collect();
                Ok(Value::HashSet(result))
            }
            "intersection" => {
                check_arg_count("HashSet::intersection", 1, &args, span)?;
                let Value::HashSet(other) = &args[0] else {
                    return Err(FerriError::Runtime {
                        message: format!(
                            "HashSet::intersection expects a HashSet, got {}",
                            args[0].type_name()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                };
                let result: HashSet<Value> = s.intersection(other).cloned().collect();
                Ok(Value::HashSet(result))
            }
            "difference" => {
                check_arg_count("HashSet::difference", 1, &args, span)?;
                let Value::HashSet(other) = &args[0] else {
                    return Err(FerriError::Runtime {
                        message: format!(
                            "HashSet::difference expects a HashSet, got {}",
                            args[0].type_name()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                };
                let result: HashSet<Value> = s.difference(other).cloned().collect();
                Ok(Value::HashSet(result))
            }
            "clone" => Ok(Value::HashSet(s)),
            "to_vec" => {
                let mut v: Vec<Value> = s.into_iter().collect();
                v.sort();
                Ok(Value::Vec(v))
            }
            _ => self.try_to_json_method(Value::HashSet(s), method, span, "HashSet"),
        }
    }
}
