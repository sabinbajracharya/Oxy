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
use std::cell::RefCell;
use std::rc::Rc;

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
        let Value::Vec(rc) = &receiver else {
            unreachable!()
        };
        let rc = rc.clone();
        match method {
            "len" => Ok(Value::Integer(rc.borrow().len() as i64)),
            "is_empty" => Ok(Value::Bool(rc.borrow().is_empty())),
            "contains" => {
                check_arg_count("Vec::contains", 1, &args, span)?;
                Ok(Value::Bool(rc.borrow().contains(&args[0])))
            }
            "push" => {
                check_arg_count("Vec::push", 1, &args, span)?;
                rc.borrow_mut().push(args.into_iter().next().unwrap());
                Ok(Value::Unit)
            }
            "pop" => {
                let popped = rc.borrow_mut().pop();
                match popped {
                    Some(val) => Ok(Value::some(val)),
                    None => Ok(Value::none()),
                }
            }
            "first" => {
                let result = rc.borrow().first().cloned();
                match result {
                    Some(val) => Ok(Value::some(val)),
                    None => Ok(Value::none()),
                }
            }
            "last" => {
                let result = rc.borrow().last().cloned();
                match result {
                    Some(val) => Ok(Value::some(val)),
                    None => Ok(Value::none()),
                }
            }
            "reverse" => {
                rc.borrow_mut().reverse();
                Ok(Value::Unit)
            }
            "join" => {
                check_arg_count("Vec::join", 1, &args, span)?;
                let sep = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => format!("{other}"),
                };
                let s: Vec<String> = rc.borrow().iter().map(|e| format!("{e}")).collect();
                Ok(Value::String(s.join(&sep)))
            }
            "iter" | "into_iter" | "iter_mut" => {
                let data = rc.borrow().clone();
                Ok(Value::Iterator(Box::new(
                    crate::types::IteratorState::VecSource { data, index: 0 },
                )))
            }
            "map" => {
                check_arg_count("Vec::map", 1, &args, span)?;
                let closure = args[0].clone();
                let data = rc.borrow().clone();
                Ok(Value::Iterator(Box::new(
                    crate::types::IteratorState::Map {
                        source: Box::new(crate::types::IteratorState::VecSource {
                            data,
                            index: 0,
                        }),
                        closure,
                    },
                )))
            }
            "filter" => {
                check_arg_count("Vec::filter", 1, &args, span)?;
                let closure = args[0].clone();
                let data = rc.borrow().clone();
                Ok(Value::Iterator(Box::new(
                    crate::types::IteratorState::Filter {
                        source: Box::new(crate::types::IteratorState::VecSource {
                            data,
                            index: 0,
                        }),
                        closure,
                    },
                )))
            }
            "for_each" => {
                check_arg_count("Vec::for_each", 1, &args, span)?;
                let func = &args[0];
                let snapshot = rc.borrow().clone();
                for item in &snapshot {
                    self.call_function(func, std::slice::from_ref(item), span.line, span.column)?;
                }
                Ok(Value::Unit)
            }
            "fold" => {
                check_arg_count("Vec::fold", 2, &args, span)?;
                let mut acc = args[0].clone();
                let func = &args[1];
                let snapshot = rc.borrow().clone();
                for item in &snapshot {
                    acc = self.call_function(func, &[acc, item.clone()], span.line, span.column)?;
                }
                Ok(acc)
            }
            "any" => {
                check_arg_count("Vec::any", 1, &args, span)?;
                let func = &args[0];
                let snapshot = rc.borrow().clone();
                for item in &snapshot {
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
                let snapshot = rc.borrow().clone();
                for item in &snapshot {
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
                let snapshot = rc.borrow().clone();
                for item in &snapshot {
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
                let data = rc.borrow().clone();
                Ok(Value::Iterator(Box::new(
                    crate::types::IteratorState::Enumerate {
                        source: Box::new(crate::types::IteratorState::VecSource { data, index: 0 }),
                        index: 0,
                    },
                )))
            }
            "collect" => Ok(receiver.clone()),
            "flat_map" => {
                check_arg_count("Vec::flat_map", 1, &args, span)?;
                let closure = args[0].clone();
                let data = rc.borrow().clone();
                Ok(Value::Iterator(Box::new(
                    crate::types::IteratorState::FlatMap {
                        source: Box::new(crate::types::IteratorState::VecSource {
                            data,
                            index: 0,
                        }),
                        closure,
                        current: None,
                    },
                )))
            }
            "flatten" => {
                let data = rc.borrow().clone();
                Ok(Value::Iterator(Box::new(
                    crate::types::IteratorState::Flatten {
                        source: Box::new(crate::types::IteratorState::VecSource { data, index: 0 }),
                        current: None,
                    },
                )))
            }
            "position" => {
                check_arg_count("Vec::position", 1, &args, span)?;
                let func = &args[0];
                let snapshot = rc.borrow().clone();
                for (i, item) in snapshot.iter().enumerate() {
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
                let right = match &args[0] {
                    Value::Vec(other_rc) => Box::new(crate::types::IteratorState::VecSource {
                        data: other_rc.borrow().clone(),
                        index: 0,
                    }),
                    Value::Iterator(_) => match args[0].clone() {
                        Value::Iterator(iter) => iter,
                        _ => unreachable!(),
                    },
                    other => Box::new(crate::types::IteratorState::VecSource {
                        data: vec![other.clone()],
                        index: 0,
                    }),
                };
                let data = rc.borrow().clone();
                Ok(Value::Iterator(Box::new(
                    crate::types::IteratorState::Zip {
                        left: Box::new(crate::types::IteratorState::VecSource {
                            data,
                            index: 0,
                        }),
                        right,
                    },
                )))
            }
            "take" => {
                check_arg_count("take", 1, &args, span)?;
                let n = crate::errors::expect_integer(&args[0], "take()", span)? as usize;
                let data = rc.borrow().clone();
                Ok(Value::Iterator(Box::new(
                    crate::types::IteratorState::Take {
                        source: Box::new(crate::types::IteratorState::VecSource {
                            data,
                            index: 0,
                        }),
                        remaining: n,
                    },
                )))
            }
            "skip" => {
                check_arg_count("skip", 1, &args, span)?;
                let n = crate::errors::expect_integer(&args[0], "skip()", span)? as usize;
                let data = rc.borrow().clone();
                Ok(Value::Iterator(Box::new(
                    crate::types::IteratorState::Skip {
                        source: Box::new(crate::types::IteratorState::VecSource {
                            data,
                            index: 0,
                        }),
                        remaining: n,
                    },
                )))
            }
            "chain" => {
                check_arg_count("chain", 1, &args, span)?;
                let right = match &args[0] {
                    Value::Vec(other_rc) => Box::new(crate::types::IteratorState::VecSource {
                        data: other_rc.borrow().clone(),
                        index: 0,
                    }),
                    Value::Iterator(_) => match args[0].clone() {
                        Value::Iterator(iter) => iter,
                        _ => unreachable!(),
                    },
                    other => Box::new(crate::types::IteratorState::VecSource {
                        data: vec![other.clone()],
                        index: 0,
                    }),
                };
                let data = rc.borrow().clone();
                Ok(Value::Iterator(Box::new(
                    crate::types::IteratorState::Chain {
                        first: Box::new(crate::types::IteratorState::VecSource {
                            data,
                            index: 0,
                        }),
                        second: right,
                    },
                )))
            }
            "sum" => {
                check_arg_count("sum", 0, &args, span)?;
                let mut int_sum: i64 = 0;
                let mut float_sum: f64 = 0.0;
                let mut is_float = false;
                for item in rc.borrow().iter() {
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
                Ok(Value::Integer(rc.borrow().len() as i64))
            }
            "rev" => {
                check_arg_count("rev", 0, &args, span)?;
                rc.borrow_mut().reverse();
                Ok(Value::Unit)
            }
            "sort" => {
                check_arg_count("sort", 0, &args, span)?;
                rc.borrow_mut()
                    .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                Ok(Value::Unit)
            }
            "sort_by" => {
                check_arg_count("Vec::sort_by", 1, &args, span)?;
                let func = &args[0];
                let snapshot = rc.borrow().clone();
                let mut sorted = snapshot;
                sorted.sort_by(|a, b| {
                    match self.call_function(func, &[a.clone(), b.clone()], span.line, span.column)
                    {
                        Ok(Value::Integer(n)) => n.cmp(&0),
                        Ok(_) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
                        Err(_) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
                    }
                });
                *rc.borrow_mut() = sorted;
                Ok(Value::Unit)
            }
            "sort_by_key" => {
                check_arg_count("Vec::sort_by_key", 1, &args, span)?;
                let func = &args[0];
                let snapshot = rc.borrow().clone();
                let mut pairs: Vec<(Value, Value)> = snapshot
                    .into_iter()
                    .map(|elem| {
                        let key = self
                            .call_function(
                                func,
                                std::slice::from_ref(&elem),
                                span.line,
                                span.column,
                            )
                            .unwrap_or_else(|_| elem.clone());
                        (key, elem)
                    })
                    .collect();
                pairs.sort_by(|(ak, _), (bk, _)| ak.cmp(bk));
                let sorted: Vec<Value> = pairs.into_iter().map(|(_, elem)| elem).collect();
                *rc.borrow_mut() = sorted;
                Ok(Value::Unit)
            }
            "dedup" => {
                check_arg_count("dedup", 0, &args, span)?;
                rc.borrow_mut().dedup();
                Ok(Value::Unit)
            }
            "windows" => {
                check_arg_count("windows", 1, &args, span)?;
                let n = crate::errors::expect_integer(&args[0], "windows()", span)? as usize;
                let result: Vec<Value> = rc
                    .borrow()
                    .windows(n)
                    .map(|w| Value::Vec(Rc::new(RefCell::new(w.to_vec()))))
                    .collect();
                Ok(Value::Vec(Rc::new(RefCell::new(result))))
            }
            "chunks" => {
                check_arg_count("chunks", 1, &args, span)?;
                let n = crate::errors::expect_integer(&args[0], "chunks()", span)? as usize;
                let result: Vec<Value> = rc
                    .borrow()
                    .chunks(n)
                    .map(|c| Value::Vec(Rc::new(RefCell::new(c.to_vec()))))
                    .collect();
                Ok(Value::Vec(Rc::new(RefCell::new(result))))
            }
            "min" => {
                check_arg_count("min", 0, &args, span)?;
                let v = rc.borrow();
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
                let v = rc.borrow();
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
            "clone" => Ok(Value::Vec(Rc::new(RefCell::new(rc.borrow().clone())))),
            _ => self.try_to_json_method(Value::Vec(rc), method, span, "Vec"),
        }
    }
}
