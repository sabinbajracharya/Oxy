//! Iterator method implementations — shared by interpreter and VM.
//!
//! Adapters (map, filter, take, skip, etc.) and simple consumers (next, collect,
//! sum, count, nth) are native. Closure consumers (any, all, find, fold, for_each,
//! position) use the interpreter's call_function in a Rust loop.

use crate::types::{IteratorState, Value};

/// Dispatch a method call on an Iterator value.
/// Closure consumers need `call_fn` to invoke closures.
pub fn dispatch(
    receiver: Value,
    method: &str,
    args: &[Value],
    mut call_fn: impl FnMut(&Value, &[Value]) -> Result<Value, String>,
) -> Result<Value, String> {
    let Value::Iterator(mut iter) = receiver else {
        unreachable!()
    };

    match method {
        // --- Adapters ---
        // Map and Filter are eager (not lazy) to avoid closure-in-drive_next issue
        "map" => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            let mut result = Vec::new();
            while let Some(val) = drive_next(&mut iter) {
                result.push(call_fn(&closure, &[val])?);
            }
            Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(
                result,
            ))))
        }
        "filter" => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            let mut result = Vec::new();
            while let Some(val) = drive_next(&mut iter) {
                if call_fn(&closure, &[val.clone()])?.is_truthy() {
                    result.push(val);
                }
            }
            Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(
                result,
            ))))
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
            let mut result = Vec::new();
            while let Some(val) = drive_next(&mut iter) {
                let mapped = call_fn(&closure, &[val])?;
                match mapped {
                    Value::Vec(rc) => result.extend(rc.borrow().clone()),
                    Value::Iterator(mut inner) => {
                        while let Some(v) = drive_next(&mut inner) {
                            result.push(v);
                        }
                    }
                    other => result.push(other),
                }
            }
            Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(
                result,
            ))))
        }
        "flatten" => {
            let mut result = Vec::new();
            while let Some(val) = drive_next(&mut iter) {
                match val {
                    Value::Vec(rc) => result.extend(rc.borrow().clone()),
                    Value::Iterator(mut inner) => {
                        while let Some(v) = drive_next(&mut inner) {
                            result.push(v);
                        }
                    }
                    other => result.push(other),
                }
            }
            Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(
                result,
            ))))
        }

        // --- Simple consumers (drain iterator — no closures) ---
        "next" => Ok(drive_next(&mut iter)
            .map(Value::some)
            .unwrap_or_else(Value::none)),
        "collect" => {
            let mut result = Vec::new();
            while let Some(val) = drive_next(&mut iter) {
                result.push(val);
            }
            Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(
                result,
            ))))
        }
        "sum" => {
            let mut total: Value = Value::Integer(0);
            while let Some(val) = drive_next(&mut iter) {
                total = add_values(total, val);
            }
            Ok(total)
        }
        "count" => {
            let mut n = 0i64;
            while drive_next(&mut iter).is_some() {
                n += 1;
            }
            Ok(Value::Integer(n))
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
                if drive_next(&mut iter).is_none() {
                    return Ok(Value::none());
                }
            }
            Ok(drive_next(&mut iter)
                .map(Value::some)
                .unwrap_or_else(Value::none))
        }

        // --- Closure consumers ---
        "any" | "all" | "find" | "position" | "fold" | "for_each" => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            match method {
                "any" => {
                    while let Some(val) = drive_next(&mut iter) {
                        if call_fn(&closure, &[val])?.is_truthy() {
                            return Ok(Value::Bool(true));
                        }
                    }
                    Ok(Value::Bool(false))
                }
                "all" => {
                    while let Some(val) = drive_next(&mut iter) {
                        if !call_fn(&closure, &[val])?.is_truthy() {
                            return Ok(Value::Bool(false));
                        }
                    }
                    Ok(Value::Bool(true))
                }
                "find" => {
                    while let Some(val) = drive_next(&mut iter) {
                        if call_fn(&closure, &[val.clone()])?.is_truthy() {
                            return Ok(Value::some(val));
                        }
                    }
                    Ok(Value::none())
                }
                "position" => {
                    let mut idx = 0i64;
                    while let Some(val) = drive_next(&mut iter) {
                        if call_fn(&closure, &[val])?.is_truthy() {
                            return Ok(Value::some(Value::Integer(idx)));
                        }
                        idx += 1;
                    }
                    Ok(Value::none())
                }
                "fold" => {
                    let mut acc = args.first().cloned().unwrap_or(Value::Unit);
                    let f = args.get(1).cloned().unwrap_or(Value::Unit);
                    while let Some(val) = drive_next(&mut iter) {
                        acc = call_fn(&f, &[acc, val])?;
                    }
                    Ok(acc)
                }
                "for_each" => {
                    while let Some(val) = drive_next(&mut iter) {
                        call_fn(&closure, &[val])?;
                    }
                    Ok(Value::Unit)
                }
                _ => unreachable!(),
            }
        }

        _ => Err(format!("no method '{}' on type Iterator", method)),
    }
}

/// Drive an iterator forward one step. Returns Some(value) or None if exhausted.
/// Standalone version (not on Interpreter) for use from VM builtins.
fn drive_next(iter: &mut IteratorState) -> Option<Value> {
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
        IteratorState::Map { .. } | IteratorState::Filter { .. } => {
            // Map/Filter are now eager — should not appear in lazy state
            None
        }
        IteratorState::Take { source, remaining } => {
            if *remaining == 0 {
                None
            } else {
                *remaining -= 1;
                drive_next(source)
            }
        }
        IteratorState::Skip { source, remaining } => {
            while *remaining > 0 {
                *remaining -= 1;
                drive_next(source)?;
            }
            drive_next(source)
        }
        IteratorState::Chain { first, second } => {
            drive_next(first).or_else(|| drive_next(second))
        }
        IteratorState::Zip { left, right } => {
            let l = drive_next(left)?;
            let r = drive_next(right)?;
            Some(Value::Tuple(vec![l, r]))
        }
        IteratorState::Enumerate { source, index } => {
            let val = drive_next(source)?;
            let pair = Value::Tuple(vec![Value::Integer(*index as i64), val]);
            *index += 1;
            Some(pair)
        }
        IteratorState::FlatMap { .. } => {
            // FlatMap is now eager — should not appear in lazy state
            None
        }
        IteratorState::Flatten { source, current } => loop {
            if let Some(inner) = current {
                if let Some(val) = drive_next(inner) {
                    return Some(val);
                }
                *current = None;
            }
            let next_val = drive_next(source)?;
            match next_val {
                Value::Iterator(inner_iter) => {
                    *current = Some(inner_iter);
                }
                other => return Some(other),
            }
        },
    }
}

fn add_values(a: Value, b: Value) -> Value {
    match (&a, &b) {
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(a + b),
        (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
        (Value::Integer(a), Value::Float(b)) => Value::Float(*a as f64 + b),
        (Value::Float(a), Value::Integer(b)) => Value::Float(a + *b as f64),
        _ => Value::Integer(0),
    }
}
