//! Vec method implementations.
//!
//! Supports: len, is_empty, contains, push, pop, first, last, reverse,
//! join, iter/into_iter, map, filter, for_each, fold, any, all, find,
//! enumerate, collect, flat_map, position, zip, take, skip, chain,
//! flatten, sum, count, rev, sort, dedup, windows, chunks, min, max.

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
                    let mapped = self.call_function(
                        func,
                        std::slice::from_ref(item),
                        span.line,
                        span.column,
                    )?;
                    result.push(mapped);
                }
                Ok(Value::Vec(result))
            }
            "filter" => {
                check_arg_count("Vec::filter", 1, &args, span)?;
                let func = &args[0];
                let mut result = Vec::new();
                for item in &v {
                    let keep = self.call_function(
                        func,
                        std::slice::from_ref(item),
                        span.line,
                        span.column,
                    )?;
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
                    self.call_function(func, std::slice::from_ref(item), span.line, span.column)?;
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
                    let result = self.call_function(
                        func,
                        std::slice::from_ref(item),
                        span.line,
                        span.column,
                    )?;
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
                    let result = self.call_function(
                        func,
                        std::slice::from_ref(item),
                        span.line,
                        span.column,
                    )?;
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
                    let result = self.call_function(
                        func,
                        std::slice::from_ref(item),
                        span.line,
                        span.column,
                    )?;
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
                    let mapped = self.call_function(
                        func,
                        std::slice::from_ref(item),
                        span.line,
                        span.column,
                    )?;
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
                    let result = self.call_function(
                        func,
                        std::slice::from_ref(item),
                        span.line,
                        span.column,
                    )?;
                    if result.is_truthy() {
                        return Ok(Value::some(Value::Integer(i as i64)));
                    }
                }
                Ok(Value::none())
            }
            "zip" => {
                check_arg_count("zip", 1, &args, span)?;
                if let Value::Vec(other) = &args[0] {
                    let zipped: Vec<Value> = v
                        .iter()
                        .zip(other.iter())
                        .map(|(a, b)| Value::Tuple(vec![a.clone(), b.clone()]))
                        .collect();
                    Ok(Value::Vec(zipped))
                } else {
                    Err(crate::errors::runtime_error(
                        "zip() argument must be a Vec",
                        span,
                    ))
                }
            }
            "take" => {
                check_arg_count("take", 1, &args, span)?;
                let n = crate::errors::expect_integer(&args[0], "take()", span)? as usize;
                Ok(Value::Vec(v.into_iter().take(n).collect()))
            }
            "skip" => {
                check_arg_count("skip", 1, &args, span)?;
                let n = crate::errors::expect_integer(&args[0], "skip()", span)? as usize;
                Ok(Value::Vec(v.into_iter().skip(n).collect()))
            }
            "chain" => {
                check_arg_count("chain", 1, &args, span)?;
                if let Value::Vec(other) = &args[0] {
                    let mut result = v;
                    result.extend(other.iter().cloned());
                    Ok(Value::Vec(result))
                } else {
                    Err(crate::errors::runtime_error(
                        "chain() argument must be a Vec",
                        span,
                    ))
                }
            }
            "flatten" => {
                check_arg_count("flatten", 0, &args, span)?;
                let mut result = Vec::new();
                for item in v {
                    match item {
                        Value::Vec(inner) => result.extend(inner),
                        other => result.push(other),
                    }
                }
                Ok(Value::Vec(result))
            }
            "sum" => {
                check_arg_count("sum", 0, &args, span)?;
                let mut int_sum: i64 = 0;
                let mut float_sum: f64 = 0.0;
                let mut is_float = false;
                for item in &v {
                    match item {
                        Value::Integer(n) => int_sum += n,
                        Value::Float(f) => {
                            is_float = true;
                            float_sum += f;
                        }
                        _ => {
                            return Err(crate::errors::runtime_error(
                                "sum() requires numeric elements",
                                span,
                            ));
                        }
                    }
                }
                if is_float {
                    Ok(Value::Float(float_sum + int_sum as f64))
                } else {
                    Ok(Value::Integer(int_sum))
                }
            }
            "count" => {
                check_arg_count("count", 0, &args, span)?;
                Ok(Value::Integer(v.len() as i64))
            }
            "rev" => {
                check_arg_count("rev", 0, &args, span)?;
                let mut reversed = v;
                reversed.reverse();
                Ok(Value::Vec(reversed))
            }
            "sort" => {
                check_arg_count("sort", 0, &args, span)?;
                let mut sorted = v;
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                Ok(Value::Vec(sorted))
            }
            "dedup" => {
                check_arg_count("dedup", 0, &args, span)?;
                let mut deduped = v;
                deduped.dedup();
                Ok(Value::Vec(deduped))
            }
            "windows" => {
                check_arg_count("windows", 1, &args, span)?;
                let n = crate::errors::expect_integer(&args[0], "windows()", span)? as usize;
                let result: Vec<Value> = v.windows(n).map(|w| Value::Vec(w.to_vec())).collect();
                Ok(Value::Vec(result))
            }
            "chunks" => {
                check_arg_count("chunks", 1, &args, span)?;
                let n = crate::errors::expect_integer(&args[0], "chunks()", span)? as usize;
                let result: Vec<Value> = v.chunks(n).map(|c| Value::Vec(c.to_vec())).collect();
                Ok(Value::Vec(result))
            }
            "min" => {
                check_arg_count("min", 0, &args, span)?;
                if v.is_empty() {
                    return Ok(Value::none());
                }
                let min = v
                    .iter()
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .cloned()
                    .unwrap();
                Ok(Value::some(min))
            }
            "max" => {
                check_arg_count("max", 0, &args, span)?;
                if v.is_empty() {
                    return Ok(Value::none());
                }
                let max = v
                    .iter()
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .cloned()
                    .unwrap();
                Ok(Value::some(max))
            }
            "clone" => Ok(Value::Vec(v)),
            _ => self.try_to_json_method(Value::Vec(v), method, span, "Vec"),
        }
    }
}
