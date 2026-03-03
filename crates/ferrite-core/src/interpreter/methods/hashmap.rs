//! HashMap method implementations.
//!
//! Supports: len, is_empty, insert, get, remove, contains_key, keys, values.

use crate::ast::Expr;
use crate::env::Env;
use crate::errors::check_arg_count;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

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
        let Value::HashMap(m) = receiver else {
            unreachable!()
        };
        match method {
            "len" => Ok(Value::Integer(m.len() as i64)),
            "is_empty" => Ok(Value::Bool(m.is_empty())),
            "insert" => {
                check_arg_count("HashMap::insert", 2, &args, span)?;
                let key = format!("{}", args[0]);
                let value = args[1].clone();
                let mut new_m = m;
                new_m.insert(key, value);
                self.mutate_variable(receiver_expr, Value::HashMap(new_m), env, span)?;
                Ok(Value::Unit)
            }
            "get" => {
                check_arg_count("HashMap::get", 1, &args, span)?;
                let key = format!("{}", args[0]);
                match m.get(&key) {
                    Some(val) => Ok(Value::some(val.clone())),
                    None => Ok(Value::none()),
                }
            }
            "remove" => {
                check_arg_count("HashMap::remove", 1, &args, span)?;
                let key = format!("{}", args[0]);
                let mut new_m = m;
                let removed = new_m.remove(&key);
                self.mutate_variable(receiver_expr, Value::HashMap(new_m), env, span)?;
                match removed {
                    Some(val) => Ok(Value::some(val)),
                    None => Ok(Value::none()),
                }
            }
            "contains_key" => {
                check_arg_count("HashMap::contains_key", 1, &args, span)?;
                let key = format!("{}", args[0]);
                Ok(Value::Bool(m.contains_key(&key)))
            }
            "keys" => {
                let mut keys: Vec<String> = m.keys().cloned().collect();
                keys.sort();
                Ok(Value::Vec(keys.into_iter().map(Value::String).collect()))
            }
            "values" => {
                let mut pairs: Vec<(&String, &Value)> = m.iter().collect();
                pairs.sort_by_key(|(k, _)| (*k).clone());
                Ok(Value::Vec(
                    pairs.into_iter().map(|(_, v)| v.clone()).collect(),
                ))
            }
            "clone" => Ok(Value::HashMap(m)),
            _ => self.try_to_json_method(Value::HashMap(m), method, span, "HashMap"),
        }
    }
}
