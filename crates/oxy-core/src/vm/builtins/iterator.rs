//! Iterator method implementations — shared by interpreter and VM.
//!
//! Adapters (map, filter, take, skip, etc.) and simple consumers (next, collect,
//! sum, count, nth) are native. Closure consumers (any, all, find, fold, for_each,
//! position) use the interpreter's call_function in a Rust loop.

use std::cell::RefCell;
use std::rc::Rc;

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
    let Value::Iterator(rc) = receiver else {
        unreachable!()
    };

    match method {
        // --- Eager adapters (closure-based, drain iterator) ---
        symbols::iterator_m::MAP => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            let mut state = rc.borrow_mut();
            let mut result = Vec::new();
            while let Some(val) = drive_next(&mut state) {
                result.push(call_fn(&closure, &[val])?);
            }
            Ok(Value::Vec(Rc::new(RefCell::new(result))))
        }
        symbols::iterator_m::FILTER => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            let mut state = rc.borrow_mut();
            let mut result = Vec::new();
            while let Some(val) = drive_next(&mut state) {
                if call_fn(&closure, &[val.clone()])?.is_truthy() {
                    result.push(val);
                }
            }
            Ok(Value::Vec(Rc::new(RefCell::new(result))))
        }
        // --- Lazy adapters (wrap current state, return new Iterator) ---
        symbols::iterator_m::TAKE => {
            let n = args
                .first()
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(0);
            let inner = rc.borrow().clone();
            Ok(Value::Iterator(Rc::new(RefCell::new(
                IteratorState::Take {
                    source: Box::new(inner),
                    remaining: n,
                },
            ))))
        }
        symbols::iterator_m::SKIP => {
            let n = args
                .first()
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(0);
            let inner = rc.borrow().clone();
            Ok(Value::Iterator(Rc::new(RefCell::new(
                IteratorState::Skip {
                    source: Box::new(inner),
                    remaining: n,
                },
            ))))
        }
        symbols::iterator_m::CHAIN => {
            let other = args.first().cloned().unwrap_or(Value::Unit);
            let right = match other {
                Value::Iterator(other_rc) => Box::new(other_rc.borrow().clone()),
                Value::Vec(vec_rc) => Box::new(IteratorState::VecSource {
                    data: vec_rc.borrow().clone(),
                    index: 0,
                }),
                _ => Box::new(IteratorState::VecSource {
                    data: vec![other],
                    index: 0,
                }),
            };
            let inner = rc.borrow().clone();
            Ok(Value::Iterator(Rc::new(RefCell::new(
                IteratorState::Chain {
                    first: Box::new(inner),
                    second: right,
                },
            ))))
        }
        symbols::iterator_m::ZIP => {
            let other = args.first().cloned().unwrap_or(Value::Unit);
            let right = match other {
                Value::Iterator(other_rc) => Box::new(other_rc.borrow().clone()),
                Value::Vec(vec_rc) => Box::new(IteratorState::VecSource {
                    data: vec_rc.borrow().clone(),
                    index: 0,
                }),
                _ => Box::new(IteratorState::VecSource {
                    data: vec![other],
                    index: 0,
                }),
            };
            let inner = rc.borrow().clone();
            Ok(Value::Iterator(Rc::new(RefCell::new(IteratorState::Zip {
                left: Box::new(inner),
                right,
            }))))
        }
        symbols::iterator_m::ENUMERATE => {
            let inner = rc.borrow().clone();
            Ok(Value::Iterator(Rc::new(RefCell::new(
                IteratorState::Enumerate {
                    source: Box::new(inner),
                    index: 0,
                },
            ))))
        }
        symbols::iterator_m::REV => {
            let mut state = rc.borrow_mut();
            let mut v = Vec::new();
            while let Some(val) = drive_next(&mut state) {
                v.push(val);
            }
            v.reverse();
            Ok(Value::Vec(Rc::new(RefCell::new(v))))
        }
        symbols::iterator_m::FLAT_MAP => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            let mut state = rc.borrow_mut();
            let mut result = Vec::new();
            while let Some(val) = drive_next(&mut state) {
                let mapped = call_fn(&closure, &[val])?;
                match mapped {
                    Value::Vec(vec_rc) => result.extend(vec_rc.borrow().clone()),
                    Value::Iterator(inner_rc) => {
                        let mut inner = inner_rc.borrow_mut();
                        while let Some(v) = drive_next(&mut inner) {
                            result.push(v);
                        }
                    }
                    other => result.push(other),
                }
            }
            Ok(Value::Vec(Rc::new(RefCell::new(result))))
        }
        symbols::iterator_m::FLATTEN => {
            let mut state = rc.borrow_mut();
            let mut result = Vec::new();
            while let Some(val) = drive_next(&mut state) {
                match val {
                    Value::Vec(vec_rc) => result.extend(vec_rc.borrow().clone()),
                    Value::Iterator(inner_rc) => {
                        let mut inner = inner_rc.borrow_mut();
                        while let Some(v) = drive_next(&mut inner) {
                            result.push(v);
                        }
                    }
                    other => result.push(other),
                }
            }
            Ok(Value::Vec(Rc::new(RefCell::new(result))))
        }

        // --- Simple consumers (drain iterator state in place — no closures) ---
        symbols::iterator_m::NEXT => Ok(drive_next(&mut rc.borrow_mut())
            .map(Value::some)
            .unwrap_or_else(Value::none)),
        symbols::iterator_m::COLLECT => {
            let mut state = rc.borrow_mut();
            let mut result = Vec::new();
            while let Some(val) = drive_next(&mut state) {
                result.push(val);
            }
            Ok(Value::Vec(Rc::new(RefCell::new(result))))
        }
        symbols::iterator_m::SUM => {
            let mut state = rc.borrow_mut();
            let mut total: Value = Value::I64(0);
            while let Some(val) = drive_next(&mut state) {
                total = add_values(total, val);
            }
            Ok(total)
        }
        symbols::iterator_m::PRODUCT => {
            let mut state = rc.borrow_mut();
            let mut total: Value = Value::I64(1);
            while let Some(val) = drive_next(&mut state) {
                total = mul_values(total, val);
            }
            Ok(total)
        }
        symbols::iterator_m::MAX => {
            let mut state = rc.borrow_mut();
            let mut best: Option<Value> = None;
            while let Some(val) = drive_next(&mut state) {
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
            let mut state = rc.borrow_mut();
            let mut best: Option<Value> = None;
            while let Some(val) = drive_next(&mut state) {
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
            let mut state = rc.borrow_mut();
            let mut n = 0i64;
            while drive_next(&mut state).is_some() {
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
            let mut state = rc.borrow_mut();
            for _ in 0..n {
                if drive_next(&mut state).is_none() {
                    return Ok(Value::none());
                }
            }
            Ok(drive_next(&mut state)
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
                    loop {
                        let val = { drive_next(&mut rc.borrow_mut()) };
                        match val {
                            None => break,
                            Some(v) => {
                                if call_fn(&closure, &[v])?.is_truthy() {
                                    return Ok(Value::Bool(true));
                                }
                            }
                        }
                    }
                    Ok(Value::Bool(false))
                }
                symbols::iterator_m::ALL => {
                    loop {
                        let val = { drive_next(&mut rc.borrow_mut()) };
                        match val {
                            None => break,
                            Some(v) => {
                                if !call_fn(&closure, &[v])?.is_truthy() {
                                    return Ok(Value::Bool(false));
                                }
                            }
                        }
                    }
                    Ok(Value::Bool(true))
                }
                symbols::iterator_m::FIND => {
                    loop {
                        let val = { drive_next(&mut rc.borrow_mut()) };
                        match val {
                            None => break,
                            Some(v) => {
                                if call_fn(&closure, &[v.clone()])?.is_truthy() {
                                    return Ok(Value::some(v));
                                }
                            }
                        }
                    }
                    Ok(Value::none())
                }
                symbols::iterator_m::POSITION => {
                    let mut idx = 0i64;
                    loop {
                        let val = { drive_next(&mut rc.borrow_mut()) };
                        match val {
                            None => break,
                            Some(v) => {
                                if call_fn(&closure, &[v])?.is_truthy() {
                                    return Ok(Value::some(Value::I64(idx)));
                                }
                                idx += 1;
                            }
                        }
                    }
                    Ok(Value::none())
                }
                symbols::iterator_m::FOLD => {
                    let mut acc = args.first().cloned().unwrap_or(Value::Unit);
                    let f = args.get(1).cloned().unwrap_or(Value::Unit);
                    loop {
                        let val = { drive_next(&mut rc.borrow_mut()) };
                        match val {
                            None => break,
                            Some(v) => acc = call_fn(&f, &[acc, v])?,
                        }
                    }
                    Ok(acc)
                }
                symbols::iterator_m::FOR_EACH => {
                    loop {
                        let val = { drive_next(&mut rc.borrow_mut()) };
                        match val {
                            None => break,
                            Some(v) => {
                                call_fn(&closure, &[v])?;
                            }
                        }
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
                Value::Iterator(inner_rc) => {
                    *current = Some(Box::new(inner_rc.borrow().clone()));
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
