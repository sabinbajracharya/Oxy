//! Vec method implementations.
//!
//! Supports: len, is_empty, contains, push, pop, first, last, reverse,
//! join, iter/into_iter, map, filter, for_each, fold, any, all, find,
//! enumerate, collect, flat_map, position.

use crate::ast::Expr;
use crate::env::Env;
use crate::errors::check_arg_count;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

use super::super::Interpreter;

impl Interpreter {
    /// Handle method calls on Vec values.
    pub(crate) fn call_vec_method(
        &mut self,
        receiver: Value,
        method: &str,
        args: Vec<Value>,
        receiver_expr: &Expr,
        env: &Env,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let Value::Vec(v) = receiver else {
            unreachable!()
        };
        match method {
            "len" => Ok(Value::Integer(v.len() as i64)),
            "is_empty" => Ok(Value::Bool(v.is_empty())),
            "contains" => {
                check_arg_count("Vec::contains", 1, &args, span)?;
                Ok(Value::Bool(v.contains(&args[0])))
            }
            "push" => {
                check_arg_count("Vec::push", 1, &args, span)?;
                let mut new_v = v;
                new_v.push(args.into_iter().next().unwrap());
                self.mutate_variable(receiver_expr, Value::Vec(new_v), env, span)?;
                Ok(Value::Unit)
            }
            "pop" => {
                let mut new_v = v;
                let popped = new_v.pop();
                self.mutate_variable(receiver_expr, Value::Vec(new_v), env, span)?;
                match popped {
                    Some(val) => Ok(Value::some(val)),
                    None => Ok(Value::none()),
                }
            }
            "first" => {
                let result = v.first().cloned();
                match result {
                    Some(val) => Ok(Value::some(val)),
                    None => Ok(Value::none()),
                }
            }
            "last" => {
                let result = v.last().cloned();
                match result {
                    Some(val) => Ok(Value::some(val)),
                    None => Ok(Value::none()),
                }
            }
            "reverse" => {
                let mut new_v = v;
                new_v.reverse();
                self.mutate_variable(receiver_expr, Value::Vec(new_v), env, span)?;
                Ok(Value::Unit)
            }
            "join" => {
                check_arg_count("Vec::join", 1, &args, span)?;
                let sep = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => format!("{other}"),
                };
                let s: Vec<String> = v.iter().map(|e| format!("{e}")).collect();
                Ok(Value::String(s.join(&sep)))
            }
            // iter() returns the Vec itself (we don't have a separate Iterator type)
            "iter" | "into_iter" | "iter_mut" => Ok(Value::Vec(v)),
            "map" => {
                check_arg_count("Vec::map", 1, &args, span)?;
                let func = &args[0];
                let mut result = Vec::new();
                for item in &v {
                    let mapped =
                        self.call_function(func, &[item.clone()], span.line, span.column)?;
                    result.push(mapped);
                }
                Ok(Value::Vec(result))
            }
            "filter" => {
                check_arg_count("Vec::filter", 1, &args, span)?;
                let func = &args[0];
                let mut result = Vec::new();
                for item in &v {
                    let keep = self.call_function(func, &[item.clone()], span.line, span.column)?;
                    if keep.is_truthy() {
                        result.push(item.clone());
                    }
                }
                Ok(Value::Vec(result))
            }
            "for_each" => {
                check_arg_count("Vec::for_each", 1, &args, span)?;
                let func = &args[0];
                for item in &v {
                    self.call_function(func, &[item.clone()], span.line, span.column)?;
                }
                Ok(Value::Unit)
            }
            "fold" => {
                check_arg_count("Vec::fold", 2, &args, span)?;
                let mut acc = args[0].clone();
                let func = &args[1];
                for item in &v {
                    acc = self.call_function(func, &[acc, item.clone()], span.line, span.column)?;
                }
                Ok(acc)
            }
            "any" => {
                check_arg_count("Vec::any", 1, &args, span)?;
                let func = &args[0];
                for item in &v {
                    let result =
                        self.call_function(func, &[item.clone()], span.line, span.column)?;
                    if result.is_truthy() {
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(false))
            }
            "all" => {
                check_arg_count("Vec::all", 1, &args, span)?;
                let func = &args[0];
                for item in &v {
                    let result =
                        self.call_function(func, &[item.clone()], span.line, span.column)?;
                    if !result.is_truthy() {
                        return Ok(Value::Bool(false));
                    }
                }
                Ok(Value::Bool(true))
            }
            "find" => {
                check_arg_count("Vec::find", 1, &args, span)?;
                let func = &args[0];
                for item in &v {
                    let result =
                        self.call_function(func, &[item.clone()], span.line, span.column)?;
                    if result.is_truthy() {
                        return Ok(Value::some(item.clone()));
                    }
                }
                Ok(Value::none())
            }
            "enumerate" => {
                let result: Vec<Value> = v
                    .iter()
                    .enumerate()
                    .map(|(i, item)| Value::Tuple(vec![Value::Integer(i as i64), item.clone()]))
                    .collect();
                Ok(Value::Vec(result))
            }
            // collect() — identity on Vec (already collected)
            "collect" => Ok(Value::Vec(v)),
            "flat_map" => {
                check_arg_count("Vec::flat_map", 1, &args, span)?;
                let func = &args[0];
                let mut result = Vec::new();
                for item in &v {
                    let mapped =
                        self.call_function(func, &[item.clone()], span.line, span.column)?;
                    match mapped {
                        Value::Vec(inner) => result.extend(inner),
                        other => result.push(other),
                    }
                }
                Ok(Value::Vec(result))
            }
            "position" => {
                check_arg_count("Vec::position", 1, &args, span)?;
                let func = &args[0];
                for (i, item) in v.iter().enumerate() {
                    let result =
                        self.call_function(func, &[item.clone()], span.line, span.column)?;
                    if result.is_truthy() {
                        return Ok(Value::some(Value::Integer(i as i64)));
                    }
                }
                Ok(Value::none())
            }
            _ => self.try_to_json_method(Value::Vec(v), method, span, "Vec"),
        }
    }
}
