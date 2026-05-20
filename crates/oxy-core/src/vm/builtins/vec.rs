//! Vec method implementations — shared by interpreter and VM.

use std::cell::RefCell;
use std::rc::Rc;

use crate::symbols;
use crate::types::Value;

/// Dispatch a method call on a Vec value.
/// Returns Ok(value) or Err(message) if the method is unknown.
pub fn dispatch<F>(
    receiver: Value,
    method: &str,
    args: &[Value],
    mut call_closure: F,
) -> Result<Value, String>
where
    F: FnMut(Value, &[Value]) -> Result<Value, String>,
{
    let Value::Vec(rc) = &receiver else {
        unreachable!()
    };
    let rc = rc.clone();
    match method {
        symbols::vec_m::LEN => Ok(Value::I64(rc.borrow().len() as i64)),
        symbols::vec_m::IS_EMPTY => Ok(Value::Bool(rc.borrow().is_empty())),
        symbols::vec_m::CONTAINS => {
            let val = args.first().ok_or("Vec::contains takes 1 argument")?;
            Ok(Value::Bool(rc.borrow().contains(val)))
        }
        symbols::vec_m::PUSH => {
            let val = args.first().cloned().unwrap_or(Value::Unit);
            rc.borrow_mut().push(val);
            Ok(Value::Unit)
        }
        symbols::vec_m::POP => match rc.borrow_mut().pop() {
            Some(val) => Ok(Value::some(val)),
            None => Ok(Value::none()),
        },
        symbols::vec_m::FIRST => match rc.borrow().first() {
            Some(val) => Ok(Value::some(val.clone())),
            None => Ok(Value::none()),
        },
        symbols::vec_m::LAST => match rc.borrow().last() {
            Some(val) => Ok(Value::some(val.clone())),
            None => Ok(Value::none()),
        },
        symbols::vec_m::GET => {
            let idx = args
                .first()
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
                    _ => None,
                })
                .ok_or("Vec::get requires an integer index")?;
            match rc.borrow().get(idx) {
                Some(val) => Ok(Value::some(val.clone())),
                None => Ok(Value::none()),
            }
        }
        symbols::vec_m::INSERT => {
            let idx = args
                .first()
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
                    _ => None,
                })
                .ok_or("Vec::insert requires an integer index")?;
            let val = args.get(1).cloned().unwrap_or(Value::Unit);
            let len = rc.borrow().len();
            rc.borrow_mut().insert(idx.min(len), val);
            Ok(Value::Unit)
        }
        symbols::vec_m::REMOVE => {
            let idx = args
                .first()
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
                    _ => None,
                })
                .ok_or("Vec::remove requires an integer index")?;
            if idx < rc.borrow().len() {
                Ok(rc.borrow_mut().remove(idx))
            } else {
                Ok(Value::none())
            }
        }
        symbols::vec_m::CLEAR => {
            rc.borrow_mut().clear();
            Ok(Value::Unit)
        }
        symbols::vec_m::REVERSE => {
            rc.borrow_mut().reverse();
            Ok(Value::Unit)
        }
        symbols::vec_m::JOIN => {
            let sep = args.first().map(|v| v.to_string()).unwrap_or_default();
            let s: String = rc
                .borrow()
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(&sep);
            Ok(Value::String(s))
        }
        symbols::vec_m::ITER => {
            let data = rc.borrow().clone();
            Ok(Value::Iterator(Box::new(
                crate::types::IteratorState::VecSource { data, index: 0 },
            )))
        }
        symbols::vec_m::CLONE => {
            let cloned = rc.borrow().clone();
            Ok(Value::Vec(Rc::new(RefCell::new(cloned))))
        }
        symbols::vec_m::SORT => {
            rc.borrow_mut().sort();
            Ok(Value::Unit)
        }
        symbols::vec_m::DEDUP => {
            rc.borrow_mut().dedup();
            Ok(Value::Unit)
        }
        symbols::vec_m::EXTEND => {
            let other = args.first().ok_or("Vec::extend takes 1 argument")?;
            match other {
                Value::Vec(other_rc) => {
                    rc.borrow_mut().extend(other_rc.borrow().clone());
                }
                _ => {
                    rc.borrow_mut().push(other.clone());
                }
            }
            Ok(Value::Unit)
        }
        symbols::vec_m::REV => {
            rc.borrow_mut().reverse();
            Ok(Value::Unit)
        }
        symbols::vec_m::CHUNKS => {
            let n = args
                .first()
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
                    _ => None,
                })
                .ok_or("Vec::chunks requires an integer size")?;
            let chunks: Vec<Value> = rc
                .borrow()
                .chunks(n.max(1))
                .map(|chunk| Value::Vec(Rc::new(RefCell::new(chunk.to_vec()))))
                .collect();
            Ok(Value::Vec(Rc::new(RefCell::new(chunks))))
        }
        symbols::vec_m::WINDOWS => {
            let n = args
                .first()
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
                    _ => None,
                })
                .ok_or("Vec::windows requires an integer size")?;
            let windows: Vec<Value> = rc
                .borrow()
                .windows(n.max(1))
                .map(|w| Value::Vec(Rc::new(RefCell::new(w.to_vec()))))
                .collect();
            Ok(Value::Vec(Rc::new(RefCell::new(windows))))
        }
        symbols::vec_m::MIN => {
            let v = rc.borrow();
            if v.is_empty() {
                Ok(Value::none())
            } else {
                let min = v
                    .iter()
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                Ok(Value::some(min.cloned().unwrap_or(Value::Unit)))
            }
        }
        symbols::vec_m::MAX => {
            let v = rc.borrow();
            if v.is_empty() {
                Ok(Value::none())
            } else {
                let max = v
                    .iter()
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                Ok(Value::some(max.cloned().unwrap_or(Value::Unit)))
            }
        }
        symbols::vec_m::SORT_BY => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            let mut v = rc.borrow_mut();
            let len = v.len();
            // Simple bubble sort using the closure
            for i in 0..len {
                for j in 0..len - i - 1 {
                    let a = v[j].clone();
                    let b = v[j + 1].clone();
                    match call_closure(closure.clone(), &[a, b]) {
                        Ok(Value::I64(n)) if n > 0 => {
                            v.swap(j, j + 1);
                        }
                        _ => {}
                    }
                }
            }
            Ok(Value::Unit)
        }
        symbols::vec_m::SORT_BY_KEY => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            let mut v = rc.borrow_mut();
            let len = v.len();
            if len <= 1 {
                return Ok(Value::Unit);
            }
            for i in 0..len {
                for j in 0..len - i - 1 {
                    let a_key = match call_closure(closure.clone(), &[v[j].clone()]) {
                        Ok(v) => v,
                        Err(e) => return Err(format!("sort_by_key call_closure failed: {e}")),
                    };
                    let b_key = match call_closure(closure.clone(), &[v[j + 1].clone()]) {
                        Ok(v) => v,
                        Err(e) => return Err(format!("sort_by_key call_closure failed: {e}")),
                    };
                    if a_key > b_key {
                        v.swap(j, j + 1);
                    }
                }
            }
            Ok(Value::Unit)
        }
        _ => Err(format!("no method '{}' on type Vec", method)),
    }
}

pub fn method_names() -> &'static [&'static str] {
    &[
        symbols::vec_m::LEN,
        symbols::vec_m::IS_EMPTY,
        symbols::vec_m::CONTAINS,
        symbols::vec_m::PUSH,
        symbols::vec_m::POP,
        symbols::vec_m::FIRST,
        symbols::vec_m::LAST,
        symbols::vec_m::GET,
        symbols::vec_m::INSERT,
        symbols::vec_m::REMOVE,
        symbols::vec_m::CLEAR,
        symbols::vec_m::REVERSE,
        symbols::vec_m::JOIN,
        symbols::vec_m::ITER,
        symbols::vec_m::CLONE,
        symbols::vec_m::SORT,
        symbols::vec_m::DEDUP,
        symbols::vec_m::EXTEND,
        symbols::vec_m::REV,
        symbols::vec_m::CHUNKS,
        symbols::vec_m::WINDOWS,
        symbols::vec_m::MIN,
        symbols::vec_m::MAX,
        symbols::vec_m::SORT_BY,
        symbols::vec_m::SORT_BY_KEY,
    ]
}
