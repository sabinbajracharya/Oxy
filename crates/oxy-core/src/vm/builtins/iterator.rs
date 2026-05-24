//! Iterator method implementations — shared by interpreter and VM.
//!
//! Adapters (map, filter, take, skip, etc.) and simple consumers (next, collect,
//! sum, count, nth) are native. Closure consumers (any, all, find, fold, for_each,
//! position) use the interpreter's call_function in a Rust loop.

use crate::symbols;
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
        symbols::iterator_m::MAP => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            let mut result = Vec::new();
            while let Some(val) = drive_next(&mut iter) {
                result.push(call_fn(&closure, &[val])?);
            }
            Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(
                result,
            ))))
        }
        symbols::iterator_m::FILTER => {
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
        symbols::iterator_m::TAKE => {
            let n = args
                .first()
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(0);
            Ok(Value::Iterator(Box::new(IteratorState::Take {
                source: iter,
                remaining: n,
            })))
        }
        symbols::iterator_m::SKIP => {
            let n = args
                .first()
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(0);
            Ok(Value::Iterator(Box::new(IteratorState::Skip {
                source: iter,
                remaining: n,
            })))
        }
        symbols::iterator_m::CHAIN => {
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
        symbols::iterator_m::ZIP => {
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
        symbols::iterator_m::ENUMERATE => Ok(Value::Iterator(Box::new(IteratorState::Enumerate {
            source: iter,
            index: 0,
        }))),
        symbols::iterator_m::REV => {
            // Eager: collect all elements, reverse, return VecSource
            let mut v = Vec::new();
            while let Some(val) = drive_next(&mut iter) {
                v.push(val);
            }
            v.reverse();
            Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(v))))
        }
        symbols::iterator_m::FLAT_MAP => {
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
        symbols::iterator_m::FLATTEN => {
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
        symbols::iterator_m::NEXT => Ok(drive_next(&mut iter)
            .map(Value::some)
            .unwrap_or_else(Value::none)),
        symbols::iterator_m::COLLECT => {
            let mut result = Vec::new();
            while let Some(val) = drive_next(&mut iter) {
                result.push(val);
            }
            Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(
                result,
            ))))
        }
        symbols::iterator_m::SUM => {
            let mut total: Value = Value::I64(0);
            while let Some(val) = drive_next(&mut iter) {
                total = add_values(total, val);
            }
            Ok(total)
        }
        symbols::iterator_m::PRODUCT => {
            let mut total: Value = Value::I64(1);
            while let Some(val) = drive_next(&mut iter) {
                total = mul_values(total, val);
            }
            Ok(total)
        }
        symbols::iterator_m::MAX => {
            let mut best: Option<Value> = None;
            while let Some(val) = drive_next(&mut iter) {
                best = Some(match best {
                    None => val,
                    Some(cur) => {
                        if val.cmp(&cur) == std::cmp::Ordering::Greater {
                            val
                        } else {
                            cur
                        }
                    }
                });
            }
            Ok(match best {
                Some(v) => Value::some(v),
                None => Value::none(),
            })
        }
        symbols::iterator_m::MIN => {
            let mut best: Option<Value> = None;
            while let Some(val) = drive_next(&mut iter) {
                best = Some(match best {
                    None => val,
                    Some(cur) => {
                        if val.cmp(&cur) == std::cmp::Ordering::Less {
                            val
                        } else {
                            cur
                        }
                    }
                });
            }
            Ok(match best {
                Some(v) => Value::some(v),
                None => Value::none(),
            })
        }
        symbols::iterator_m::COUNT => {
            let mut n = 0;
            while drive_next(&mut iter).is_some() {
                n += 1;
            }
            Ok(Value::I64(n))
        }
        symbols::iterator_m::NTH => {
            let n = args
                .first()
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
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
        symbols::iterator_m::ANY
        | symbols::iterator_m::ALL
        | symbols::iterator_m::FIND
        | symbols::iterator_m::POSITION
        | symbols::iterator_m::FOLD
        | symbols::iterator_m::FOR_EACH => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            match method {
                symbols::iterator_m::ANY => {
                    while let Some(val) = drive_next(&mut iter) {
                        if call_fn(&closure, &[val])?.is_truthy() {
                            return Ok(Value::Bool(true));
                        }
                    }
                    Ok(Value::Bool(false))
                }
                symbols::iterator_m::ALL => {
                    while let Some(val) = drive_next(&mut iter) {
                        if !call_fn(&closure, &[val])?.is_truthy() {
                            return Ok(Value::Bool(false));
                        }
                    }
                    Ok(Value::Bool(true))
                }
                symbols::iterator_m::FIND => {
                    while let Some(val) = drive_next(&mut iter) {
                        if call_fn(&closure, &[val.clone()])?.is_truthy() {
                            return Ok(Value::some(val));
                        }
                    }
                    Ok(Value::none())
                }
                symbols::iterator_m::POSITION => {
                    let mut idx = 0;
                    while let Some(val) = drive_next(&mut iter) {
                        if call_fn(&closure, &[val])?.is_truthy() {
                            return Ok(Value::some(Value::I64(idx)));
                        }
                        idx += 1;
                    }
                    Ok(Value::none())
                }
                symbols::iterator_m::FOLD => {
                    let mut acc = args.first().cloned().unwrap_or(Value::Unit);
                    let f = args.get(1).cloned().unwrap_or(Value::Unit);
                    while let Some(val) = drive_next(&mut iter) {
                        acc = call_fn(&f, &[acc, val])?;
                    }
                    Ok(acc)
                }
                symbols::iterator_m::FOR_EACH => {
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

pub fn method_names() -> &'static [&'static str] {
    &[
        symbols::iterator_m::MAP,
        symbols::iterator_m::FILTER,
        symbols::iterator_m::TAKE,
        symbols::iterator_m::SKIP,
        symbols::iterator_m::CHAIN,
        symbols::iterator_m::ZIP,
        symbols::iterator_m::ENUMERATE,
        symbols::iterator_m::REV,
        symbols::iterator_m::FLAT_MAP,
        symbols::iterator_m::FLATTEN,
        symbols::iterator_m::NEXT,
        symbols::iterator_m::COLLECT,
        symbols::iterator_m::SUM,
        symbols::iterator_m::PRODUCT,
        symbols::iterator_m::MAX,
        symbols::iterator_m::MIN,
        symbols::iterator_m::COUNT,
        symbols::iterator_m::NTH,
        symbols::iterator_m::ANY,
        symbols::iterator_m::ALL,
        symbols::iterator_m::FIND,
        symbols::iterator_m::POSITION,
        symbols::iterator_m::FOLD,
        symbols::iterator_m::FOR_EACH,
    ]
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
                let val = Value::I64(*current);
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
        IteratorState::Chain { first, second } => drive_next(first).or_else(|| drive_next(second)),
        IteratorState::Zip { left, right } => {
            let l = drive_next(left)?;
            let r = drive_next(right)?;
            Some(Value::Tuple(vec![l, r]))
        }
        IteratorState::Enumerate { source, index } => {
            let val = drive_next(source)?;
            let pair = Value::Tuple(vec![Value::I64(*index as i64), val]);
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
        (Value::I64(a), Value::I64(b)) => Value::I64(a + b),
        (Value::F64(a), Value::F64(b)) => Value::F64(a + b),
        (Value::I64(a), Value::F64(b)) => Value::F64(*a as f64 + b),
        (Value::F64(a), Value::I64(b)) => Value::F64(a + *b as f64),
        _ => Value::I64(0),
    }
}

fn mul_values(a: Value, b: Value) -> Value {
    match (&a, &b) {
        (Value::I64(a), Value::I64(b)) => Value::I64(a.wrapping_mul(*b)),
        (Value::F64(a), Value::F64(b)) => Value::F64(a * b),
        (Value::I64(a), Value::F64(b)) => Value::F64(*a as f64 * b),
        (Value::F64(a), Value::I64(b)) => Value::F64(a * *b as f64),
        _ => Value::I64(1),
    }
}
