//! HashSet method implementations.
//!
//! Supports: len, is_empty, insert, remove, contains, union, intersection, difference, clone, to_vec.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

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
        let Value::HashSet(rc) = &receiver else {
            unreachable!()
        };
        let rc = rc.clone();
        match method {
            "len" => Ok(Value::Integer(rc.borrow().len() as i64)),
            "is_empty" => Ok(Value::Bool(rc.borrow().is_empty())),
            "insert" => {
                check_arg_count("HashSet::insert", 1, &args, span)?;
                let val = args.into_iter().next().unwrap();
                let was_new = rc.borrow_mut().insert(val);
                Ok(Value::Bool(was_new))
            }
            "remove" => {
                check_arg_count("HashSet::remove", 1, &args, span)?;
                let val = args.into_iter().next().unwrap();
                let existed = rc.borrow_mut().remove(&val);
                Ok(Value::Bool(existed))
            }
            "contains" => {
                check_arg_count("HashSet::contains", 1, &args, span)?;
                Ok(Value::Bool(rc.borrow().contains(&args[0])))
            }
            "union" => {
                check_arg_count("HashSet::union", 1, &args, span)?;
                let Value::HashSet(other_rc) = &args[0] else {
                    return Err(FerriError::Runtime {
                        message: format!(
                            "HashSet::union expects a HashSet, got {}",
                            args[0].type_name()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                };
                let result: HashSet<Value> =
                    rc.borrow().union(&other_rc.borrow()).cloned().collect();
                Ok(Value::HashSet(Rc::new(RefCell::new(result))))
            }
            "intersection" => {
                check_arg_count("HashSet::intersection", 1, &args, span)?;
                let Value::HashSet(other_rc) = &args[0] else {
                    return Err(FerriError::Runtime {
                        message: format!(
                            "HashSet::intersection expects a HashSet, got {}",
                            args[0].type_name()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                };
                let result: HashSet<Value> = rc
                    .borrow()
                    .intersection(&other_rc.borrow())
                    .cloned()
                    .collect();
                Ok(Value::HashSet(Rc::new(RefCell::new(result))))
            }
            "difference" => {
                check_arg_count("HashSet::difference", 1, &args, span)?;
                let Value::HashSet(other_rc) = &args[0] else {
                    return Err(FerriError::Runtime {
                        message: format!(
                            "HashSet::difference expects a HashSet, got {}",
                            args[0].type_name()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                };
                let result: HashSet<Value> = rc
                    .borrow()
                    .difference(&other_rc.borrow())
                    .cloned()
                    .collect();
                Ok(Value::HashSet(Rc::new(RefCell::new(result))))
            }
            "clone" => Ok(Value::HashSet(Rc::new(RefCell::new(rc.borrow().clone())))),
            "to_vec" => {
                let s = rc.borrow();
                let mut v: Vec<Value> = s.iter().cloned().collect();
                v.sort();
                Ok(Value::Vec(Rc::new(RefCell::new(v))))
            }
            _ => self.try_to_json_method(Value::HashSet(rc), method, span, "HashSet"),
        }
    }
}
