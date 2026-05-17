//! HashMap method implementations.
//!
//! Supports: len, is_empty, insert, get, remove, contains_key, keys, values, get_or.

use crate::ast::Expr;
use crate::env::Env;
use crate::errors::check_arg_count;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;
use std::cell::RefCell;
use std::rc::Rc;

use super::super::Interpreter;

impl Interpreter {
    /// Handle method calls on HashMap values.
    pub(crate) fn call_hashmap_method(
        &mut self,
        receiver: Value,
        method: &str,
        args: Vec<Value>,
        receiver_expr: &Expr,
        env: &Env,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let Value::HashMap(rc) = &receiver else {
            unreachable!()
        };
        let rc = rc.clone();
        match method {
            "len" => Ok(Value::Integer(rc.borrow().len() as i64)),
            "is_empty" => Ok(Value::Bool(rc.borrow().is_empty())),
            "insert" => {
                check_arg_count("HashMap::insert", 2, &args, span)?;
                let key = args[0].clone();
                let value = args[1].clone();
                rc.borrow_mut().insert(key, value);
                Ok(Value::Unit)
            }
            "get" => {
                check_arg_count("HashMap::get", 1, &args, span)?;
                match rc.borrow().get(&args[0]) {
                    Some(val) => Ok(Value::some(val.clone())),
                    None => Ok(Value::none()),
                }
            }
            "remove" => {
                check_arg_count("HashMap::remove", 1, &args, span)?;
                let removed = rc.borrow_mut().remove(&args[0]);
                match removed {
                    Some(val) => Ok(Value::some(val)),
                    None => Ok(Value::none()),
                }
            }
            "contains_key" => {
                check_arg_count("HashMap::contains_key", 1, &args, span)?;
                Ok(Value::Bool(rc.borrow().contains_key(&args[0])))
            }
            "get_or" => {
                check_arg_count("HashMap::get_or", 2, &args, span)?;
                let key = args[0].clone();
                let default = args[1].clone();
                let m = rc.borrow();
                if let Some(val) = m.get(&key) {
                    Ok(val.clone())
                } else {
                    Ok(default)
                }
            }
            "keys" => {
                let m = rc.borrow();
                let mut keys: Vec<Value> = m.keys().cloned().collect();
                keys.sort();
                Ok(Value::Vec(Rc::new(RefCell::new(keys))))
            }
            "values" => {
                let m = rc.borrow();
                let mut pairs: Vec<(&Value, &Value)> = m.iter().collect();
                pairs.sort_by(|(a, _), (b, _)| a.cmp(b));
                Ok(Value::Vec(Rc::new(RefCell::new(
                    pairs.into_iter().map(|(_, v)| v.clone()).collect(),
                ))))
            }
            "clone" => Ok(Value::HashMap(Rc::new(RefCell::new(rc.borrow().clone())))),
            _ => self.try_to_json_method(Value::HashMap(rc), method, span, "HashMap"),
        }
    }
}
