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
            let mut result = Vec::new();
            loop {
                let val = step(&rc)?;
                match val {
                    None => break,
                    Some(v) => result.push(call_fn(&closure, &[v])?),
                }
            }
            Ok(Value::Vec(Rc::new(RefCell::new(result))))
        }
        symbols::iterator_m::FILTER => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            let mut result = Vec::new();
            loop {
                let val = step(&rc)?;
                match val {
                    None => break,
                    Some(v) => {
                        if call_fn(&closure, &[v.clone()])?.is_truthy() {
                            result.push(v);
                        }
                    }
                }
            }
            Ok(Value::Vec(Rc::new(RefCell::new(result))))
        }
        // --- Lazy adapters (share source state, return new Iterator) ---
        symbols::iterator_m::TAKE => {
            let n = args
                .first()
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(0);
            Ok(Value::Iterator(Rc::new(RefCell::new(
                IteratorState::Take {
                    source: Rc::clone(&rc),
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
            Ok(Value::Iterator(Rc::new(RefCell::new(
                IteratorState::Skip {
                    source: Rc::clone(&rc),
                    remaining: n,
                },
            ))))
        }
        symbols::iterator_m::CHAIN => {
            let other = args.first().cloned().unwrap_or(Value::Unit);
            let right = into_iter_rc(other);
            Ok(Value::Iterator(Rc::new(RefCell::new(
                IteratorState::Chain {
                    first: Rc::clone(&rc),
                    second: right,
                },
            ))))
        }
        symbols::iterator_m::ZIP => {
            let other = args.first().cloned().unwrap_or(Value::Unit);
            let right = into_iter_rc(other);
            Ok(Value::Iterator(Rc::new(RefCell::new(IteratorState::Zip {
                left: Rc::clone(&rc),
                right,
            }))))
        }
        symbols::iterator_m::ENUMERATE => Ok(Value::Iterator(Rc::new(RefCell::new(
            IteratorState::Enumerate {
                source: Rc::clone(&rc),
                index: 0,
            },
        )))),
        symbols::iterator_m::REV => {
            let mut v = Vec::new();
            loop {
                let val = step(&rc)?;
                match val {
                    None => break,
                    Some(x) => v.push(x),
                }
            }
            v.reverse();
            Ok(Value::Vec(Rc::new(RefCell::new(v))))
        }
        symbols::iterator_m::FLAT_MAP => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            let mut result = Vec::new();
            loop {
                let val = step(&rc)?;
                match val {
                    None => break,
                    Some(v) => {
                        let mapped = call_fn(&closure, &[v])?;
                        match mapped {
                            Value::Vec(vec_rc) => result.extend(vec_rc.borrow().clone()),
                            Value::Iterator(inner_rc) => {
                                while let Some(x) = step(&inner_rc)? {
                                    result.push(x);
                                }
                            }
                            other => result.push(other),
                        }
                    }
                }
            }
            Ok(Value::Vec(Rc::new(RefCell::new(result))))
        }
        symbols::iterator_m::FLATTEN => {
            let mut result = Vec::new();
            loop {
                let val = step(&rc)?;
                match val {
                    None => break,
                    Some(v) => match v {
                        Value::Vec(vec_rc) => result.extend(vec_rc.borrow().clone()),
                        Value::Iterator(inner_rc) => {
                            while let Some(x) = step(&inner_rc)? {
                                result.push(x);
                            }
                        }
                        other => result.push(other),
                    },
                }
            }
            Ok(Value::Vec(Rc::new(RefCell::new(result))))
        }

        // --- Simple consumers (drain iterator state in place — no closures) ---
        symbols::iterator_m::NEXT => Ok(step(&rc)?.map(Value::some).unwrap_or_else(Value::none)),
        symbols::iterator_m::COLLECT => {
            let mut result = Vec::new();
            while let Some(val) = step(&rc)? {
                result.push(val);
            }
            Ok(Value::Vec(Rc::new(RefCell::new(result))))
        }
        symbols::iterator_m::SUM => {
            let mut total: Value = Value::I64(0);
            while let Some(val) = step(&rc)? {
                total = add_values(total, val);
            }
            Ok(total)
        }
        symbols::iterator_m::PRODUCT => {
            let mut total: Value = Value::I64(1);
            while let Some(val) = step(&rc)? {
                total = mul_values(total, val);
            }
            Ok(total)
        }
        symbols::iterator_m::MAX => {
            let mut best: Option<Value> = None;
            while let Some(val) = step(&rc)? {
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
            while let Some(val) = step(&rc)? {
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
            let mut n = 0i64;
            while step(&rc)?.is_some() {
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
                if step(&rc)?.is_none() {
                    return Ok(Value::none());
                }
            }
            Ok(step(&rc)?.map(Value::some).unwrap_or_else(Value::none))
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
                    while let Some(v) = step(&rc)? {
                        if call_fn(&closure, &[v])?.is_truthy() {
                            return Ok(Value::Bool(true));
                        }
                    }
                    Ok(Value::Bool(false))
                }
                symbols::iterator_m::ALL => {
                    while let Some(v) = step(&rc)? {
                        if !call_fn(&closure, &[v])?.is_truthy() {
                            return Ok(Value::Bool(false));
                        }
                    }
                    Ok(Value::Bool(true))
                }
                symbols::iterator_m::FIND => {
                    while let Some(v) = step(&rc)? {
                        if call_fn(&closure, &[v.clone()])?.is_truthy() {
                            return Ok(Value::some(v));
                        }
                    }
                    Ok(Value::none())
                }
                symbols::iterator_m::POSITION => {
                    let mut idx = 0i64;
                    while let Some(v) = step(&rc)? {
                        if call_fn(&closure, &[v])?.is_truthy() {
                            return Ok(Value::some(Value::I64(idx)));
                        }
                        idx += 1;
                    }
                    Ok(Value::none())
                }
                symbols::iterator_m::FOLD => {
                    let mut acc = args.first().cloned().unwrap_or(Value::Unit);
                    let f = args.get(1).cloned().unwrap_or(Value::Unit);
                    while let Some(v) = step(&rc)? {
                        acc = call_fn(&f, &[acc, v])?;
                    }
                    Ok(acc)
                }
                symbols::iterator_m::FOR_EACH => {
                    while let Some(v) = step(&rc)? {
                        call_fn(&closure, &[v])?;
                    }
                    Ok(Value::Unit)
                }
                _ => unreachable!(),
            }
        }

        _ => Err(format!("no method '{}' on type Iterator", method)),
    }
}

/// Drive one step of an iterator, taking a fresh borrow each time. Returns
/// `Err` if the iterator is already borrowed (re-entrant use from a closure).
fn step(rc: &Rc<RefCell<IteratorState>>) -> Result<Option<Value>, String> {
    let mut state = rc
        .try_borrow_mut()
        .map_err(|_| "iterator is already in use".to_string())?;
    Ok(drive_next(&mut state))
}

/// Wrap an arbitrary value as a shared iterator state for use as a `chain` /
/// `zip` operand. Iterator arguments share state via `Rc::clone`; Vec/scalar
/// arguments become a fresh `VecSource`.
fn into_iter_rc(val: Value) -> Rc<RefCell<IteratorState>> {
    match val {
        Value::Iterator(other_rc) => other_rc,
        Value::Vec(vec_rc) => Rc::new(RefCell::new(IteratorState::VecSource {
            data: vec_rc.borrow().clone(),
            index: 0,
        })),
        other => Rc::new(RefCell::new(IteratorState::VecSource {
            data: vec![other],
            index: 0,
        })),
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
                drive_next(&mut source.borrow_mut())
            }
        }
        IteratorState::Skip { source, remaining } => {
            while *remaining > 0 {
                *remaining -= 1;
                drive_next(&mut source.borrow_mut())?;
            }
            drive_next(&mut source.borrow_mut())
        }
        IteratorState::Chain { first, second } => {
            let left = drive_next(&mut first.borrow_mut());
            left.or_else(|| drive_next(&mut second.borrow_mut()))
        }
        IteratorState::Zip { left, right } => {
            let l = drive_next(&mut left.borrow_mut())?;
            let r = drive_next(&mut right.borrow_mut())?;
            Some(Value::Tuple(vec![l, r]))
        }
        IteratorState::Enumerate { source, index } => {
            let val = drive_next(&mut source.borrow_mut())?;
            let pair = Value::Tuple(vec![Value::I64(*index as i64), val]);
            *index += 1;
            Some(pair)
        }
        IteratorState::FlatMap { .. } => {
            // FlatMap is eager — should not appear in lazy state
            None
        }
        IteratorState::Flatten { source, current } => loop {
            if let Some(inner) = current {
                let v = drive_next(&mut inner.borrow_mut());
                if let Some(val) = v {
                    return Some(val);
                }
                *current = None;
            }
            let next_val = drive_next(&mut source.borrow_mut())?;
            match next_val {
                Value::Iterator(inner_rc) => {
                    *current = Some(inner_rc);
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
