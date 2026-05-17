//! Lazy iterator method implementations.
//!
//! Adapters: map, filter, take, skip, chain, zip, enumerate, flat_map, flatten.
//! Consumers: next, collect, sum, count, nth, find, any, all, position, fold, for_each.

use std::cell::RefCell;
use std::rc::Rc;

use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::{IteratorState, Value};

use super::super::Interpreter;

impl Interpreter {
    /// Drive an iterator forward one step. Returns Some(value) or None if exhausted.
    fn drive_next(&mut self, iter: &mut IteratorState) -> Option<Value> {
        match iter {
            IteratorState::VecSource { data, index } => {
                if *index < data.len() {
                    let val = data[*index].clone();
                    *index += 1;
                    Some(val)
                } else {
                    None
                }
            }
            IteratorState::RangeSource { current, end } => {
                if *current < *end {
                    let val = Value::Integer(*current);
                    *current += 1;
                    Some(val)
                } else {
                    None
                }
            }
            IteratorState::Map { source, closure } => {
                let val = self.drive_next(source)?;
                let result = self
                    .call_function(closure, &[val], 0, 0)
                    .unwrap_or(Value::Unit);
                Some(result)
            }
            IteratorState::Filter { source, closure } => loop {
                let val = self.drive_next(source)?;
                let keep = self
                    .call_function(closure, std::slice::from_ref(&val), 0, 0)
                    .map(|v| v.is_truthy())
                    .unwrap_or(false);
                if keep {
                    return Some(val);
                }
            },
            IteratorState::Take { source, remaining } => {
                if *remaining == 0 {
                    None
                } else {
                    *remaining -= 1;
                    self.drive_next(source)
                }
            }
            IteratorState::Skip { source, remaining } => {
                while *remaining > 0 {
                    *remaining -= 1;
                    self.drive_next(source)?;
                }
                self.drive_next(source)
            }
            IteratorState::Chain { first, second } => {
                self.drive_next(first).or_else(|| self.drive_next(second))
            }
            IteratorState::Zip { left, right } => {
                let l = self.drive_next(left)?;
                let r = self.drive_next(right)?;
                Some(Value::Tuple(vec![l, r]))
            }
            IteratorState::Enumerate { source, index } => {
                let val = self.drive_next(source)?;
                let pair = Value::Tuple(vec![Value::Integer(*index as i64), val]);
                *index += 1;
                Some(pair)
            }
            IteratorState::FlatMap {
                source,
                closure,
                current,
            } => loop {
                if let Some(inner) = current {
                    if let Some(val) = self.drive_next(inner) {
                        return Some(val);
                    }
                    *current = None;
                }
                let next_val = self.drive_next(source)?;
                match self.call_function(closure, &[next_val], 0, 0) {
                    Ok(v) => match v {
                        Value::Iterator(inner_iter) => {
                            *current = Some(inner_iter);
                        }
                        Value::Vec(rc) => {
                            if !rc.borrow().is_empty() {
                                *current = Some(Box::new(IteratorState::VecSource {
                                    data: rc.borrow().clone(),
                                    index: 0,
                                }));
                            }
                        }
                        _ => return Some(v),
                    },
                    Err(_) => return Some(Value::Unit),
                }
            },
            IteratorState::Flatten { source, current } => loop {
                if let Some(inner) = current {
                    if let Some(val) = self.drive_next(inner) {
                        return Some(val);
                    }
                    *current = None;
                }
                let next_val = self.drive_next(source)?;
                match next_val {
                    Value::Iterator(inner_iter) => {
                        *current = Some(inner_iter);
                    }
                    Value::Vec(rc) => {
                        if !rc.borrow().is_empty() {
                            *current = Some(Box::new(IteratorState::VecSource {
                                data: rc.borrow().clone(),
                                index: 0,
                            }));
                        }
                    }
                    other => return Some(other),
                }
            },
        }
    }

    /// Collect all remaining elements from an iterator into a Vec.
    pub(crate) fn collect_remaining(&mut self, mut iter: IteratorState) -> Vec<Value> {
        let mut result = Vec::new();
        while let Some(val) = self.drive_next(&mut iter) {
            result.push(val);
        }
        result
    }

    /// Handle method calls on Iterator values.
    pub(crate) fn call_iter_method(
        &mut self,
        receiver: Value,
        method: &str,
        args: Vec<Value>,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let Value::Iterator(mut iter) = receiver else {
            unreachable!()
        };
        match method {
            // --- Adapters (return new Iterator) ---
            "map" => {
                let closure = args.first().cloned().unwrap_or(Value::Unit);
                Ok(Value::Iterator(Box::new(IteratorState::Map {
                    source: iter,
                    closure,
                })))
            }
            "filter" => {
                let closure = args.first().cloned().unwrap_or(Value::Unit);
                Ok(Value::Iterator(Box::new(IteratorState::Filter {
                    source: iter,
                    closure,
                })))
            }
            "take" => {
                let n = args
                    .first()
                    .and_then(|v| match v {
                        Value::Integer(n) => Some(*n as usize),
                        _ => None,
                    })
                    .unwrap_or(0);
                Ok(Value::Iterator(Box::new(IteratorState::Take {
                    source: iter,
                    remaining: n,
                })))
            }
            "skip" => {
                let n = args
                    .first()
                    .and_then(|v| match v {
                        Value::Integer(n) => Some(*n as usize),
                        _ => None,
                    })
                    .unwrap_or(0);
                Ok(Value::Iterator(Box::new(IteratorState::Skip {
                    source: iter,
                    remaining: n,
                })))
            }
            "chain" => {
                let other = args.first().cloned().unwrap_or(Value::Unit);
                let right = match other {
                    Value::Iterator(other_iter) => other_iter,
                    Value::Vec(rc) => Box::new(IteratorState::VecSource {
                        data: rc.borrow().clone(),
                        index: 0,
                    }),
                    _ => Box::new(IteratorState::VecSource {
                        data: vec![other],
                        index: 0,
                    }),
                };
                Ok(Value::Iterator(Box::new(IteratorState::Chain {
                    first: iter,
                    second: right,
                })))
            }
            "zip" => {
                let other = args.first().cloned().unwrap_or(Value::Unit);
                let right = match other {
                    Value::Iterator(other_iter) => other_iter,
                    Value::Vec(rc) => Box::new(IteratorState::VecSource {
                        data: rc.borrow().clone(),
                        index: 0,
                    }),
                    _ => Box::new(IteratorState::VecSource {
                        data: vec![other],
                        index: 0,
                    }),
                };
                Ok(Value::Iterator(Box::new(IteratorState::Zip {
                    left: iter,
                    right,
                })))
            }
            "enumerate" => Ok(Value::Iterator(Box::new(IteratorState::Enumerate {
                source: iter,
                index: 0,
            }))),
            "flat_map" => {
                let closure = args.first().cloned().unwrap_or(Value::Unit);
                Ok(Value::Iterator(Box::new(IteratorState::FlatMap {
                    source: iter,
                    closure,
                    current: None,
                })))
            }
            "flatten" => Ok(Value::Iterator(Box::new(IteratorState::Flatten {
                source: iter,
                current: None,
            }))),

            // --- Consumers (drain iterator) ---
            "collect" => {
                let _ = args; // ignore args
                Ok(Value::Vec(Rc::new(RefCell::new(self.collect_remaining(*iter)))))
            }
            "sum" => {
                let collected = self.collect_remaining(*iter);
                let total =
                    collected
                        .into_iter()
                        .fold(Value::Integer(0), |acc, x| match (acc, x) {
                            (Value::Integer(a), Value::Integer(b)) => Value::Integer(a + b),
                            (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
                            (Value::Integer(a), Value::Float(b)) => Value::Float(a as f64 + b),
                            (Value::Float(a), Value::Integer(b)) => Value::Float(a + b as f64),
                            _ => Value::Integer(0),
                        });
                Ok(total)
            }
            "count" => {
                let collected = self.collect_remaining(*iter);
                Ok(Value::Integer(collected.len() as i64))
            }
            "nth" => {
                let n = args
                    .first()
                    .and_then(|v| match v {
                        Value::Integer(n) => Some(*n as usize),
                        _ => None,
                    })
                    .unwrap_or(0);
                for _ in 0..n {
                    if self.drive_next(&mut iter).is_none() {
                        return Ok(Value::none());
                    }
                }
                Ok(self
                    .drive_next(&mut iter)
                    .map(Value::some)
                    .unwrap_or_else(Value::none))
            }
            "find" => {
                let closure = args.first().cloned().unwrap_or(Value::Unit);
                while let Some(val) = self.drive_next(&mut iter) {
                    let ok = self
                        .call_function(&closure, std::slice::from_ref(&val), span.line, span.column)
                        .map(|v| v.is_truthy())
                        .unwrap_or(false);
                    if ok {
                        return Ok(Value::some(val));
                    }
                }
                Ok(Value::none())
            }
            "any" => {
                let closure = args.first().cloned().unwrap_or(Value::Unit);
                while let Some(val) = self.drive_next(&mut iter) {
                    let ok = self
                        .call_function(&closure, &[val], span.line, span.column)
                        .map(|v| v.is_truthy())
                        .unwrap_or(false);
                    if ok {
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(false))
            }
            "all" => {
                let closure = args.first().cloned().unwrap_or(Value::Unit);
                while let Some(val) = self.drive_next(&mut iter) {
                    let ok = self
                        .call_function(&closure, &[val], span.line, span.column)
                        .map(|v| v.is_truthy())
                        .unwrap_or(true);
                    if !ok {
                        return Ok(Value::Bool(false));
                    }
                }
                Ok(Value::Bool(true))
            }
            "position" => {
                let closure = args.first().cloned().unwrap_or(Value::Unit);
                let mut idx = 0i64;
                while let Some(val) = self.drive_next(&mut iter) {
                    let ok = self
                        .call_function(&closure, &[val], span.line, span.column)
                        .map(|v| v.is_truthy())
                        .unwrap_or(false);
                    if ok {
                        return Ok(Value::some(Value::Integer(idx)));
                    }
                    idx += 1;
                }
                Ok(Value::none())
            }
            "fold" => {
                let mut acc = args.first().cloned().unwrap_or(Value::Unit);
                let closure = args.get(1).cloned().unwrap_or(Value::Unit);
                while let Some(val) = self.drive_next(&mut iter) {
                    acc = self
                        .call_function(&closure, &[acc, val], span.line, span.column)
                        .unwrap_or(Value::Unit);
                }
                Ok(acc)
            }
            "for_each" => {
                let closure = args.first().cloned().unwrap_or(Value::Unit);
                while let Some(val) = self.drive_next(&mut iter) {
                    self.call_function(&closure, &[val], span.line, span.column)?;
                }
                Ok(Value::Unit)
            }
            _ => Err(FerriError::Runtime {
                message: format!("no method `{method}` on Iterator"),
                line: span.line,
                column: span.column,
            }),
        }
    }
}
