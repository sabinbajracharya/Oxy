//! Vec method implementations — shared by interpreter and VM.

use std::cell::RefCell;
use std::rc::Rc;

use crate::types::Value;

/// Dispatch a method call on a Vec value.
/// Returns Ok(value) or Err(message) if the method is unknown.
pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let Value::Vec(rc) = &receiver else {
        unreachable!()
    };
    let rc = rc.clone();
    match method {
        "len" => Ok(Value::Integer(rc.borrow().len() as i64)),
        "is_empty" => Ok(Value::Bool(rc.borrow().is_empty())),
        "contains" => {
            let val = args.first().ok_or("Vec::contains takes 1 argument")?;
            Ok(Value::Bool(rc.borrow().contains(val)))
        }
        "push" => {
            let val = args.first().cloned().unwrap_or(Value::Unit);
            rc.borrow_mut().push(val);
            Ok(Value::Unit)
        }
        "pop" => match rc.borrow_mut().pop() {
            Some(val) => Ok(Value::some(val)),
            None => Ok(Value::none()),
        },
        "first" => match rc.borrow().first() {
            Some(val) => Ok(Value::some(val.clone())),
            None => Ok(Value::none()),
        },
        "last" => match rc.borrow().last() {
            Some(val) => Ok(Value::some(val.clone())),
            None => Ok(Value::none()),
        },
        "get" => {
            let idx = args
                .first()
                .and_then(|v| match v {
                    Value::Integer(n) => Some(*n as usize),
                    _ => None,
                })
                .ok_or("Vec::get requires an integer index")?;
            match rc.borrow().get(idx) {
                Some(val) => Ok(Value::some(val.clone())),
                None => Ok(Value::none()),
            }
        }
        "insert" => {
            let idx = args
                .first()
                .and_then(|v| match v {
                    Value::Integer(n) => Some(*n as usize),
                    _ => None,
                })
                .ok_or("Vec::insert requires an integer index")?;
            let val = args.get(1).cloned().unwrap_or(Value::Unit);
            let len = rc.borrow().len();
            rc.borrow_mut().insert(idx.min(len), val);
            Ok(Value::Unit)
        }
        "remove" => {
            let idx = args
                .first()
                .and_then(|v| match v {
                    Value::Integer(n) => Some(*n as usize),
                    _ => None,
                })
                .ok_or("Vec::remove requires an integer index")?;
            if idx < rc.borrow().len() {
                Ok(rc.borrow_mut().remove(idx))
            } else {
                Ok(Value::none())
            }
        }
        "clear" => {
            rc.borrow_mut().clear();
            Ok(Value::Unit)
        }
        "reverse" => {
            rc.borrow_mut().reverse();
            Ok(Value::Unit)
        }
        "join" => {
            let sep = args.first().map(|v| v.to_string()).unwrap_or_default();
            let s: String = rc
                .borrow()
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(&sep);
            Ok(Value::String(s))
        }
        "iter" => {
            let data = rc.borrow().clone();
            Ok(Value::Iterator(Box::new(
                crate::types::IteratorState::VecSource { data, index: 0 },
            )))
        }
        "clone" => {
            let cloned = rc.borrow().clone();
            Ok(Value::Vec(Rc::new(RefCell::new(cloned))))
        }
        "sort" => {
            rc.borrow_mut().sort();
            Ok(Value::Unit)
        }
        "dedup" => {
            rc.borrow_mut().dedup();
            Ok(Value::Unit)
        }
        "extend" => {
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
        "rev" => {
            rc.borrow_mut().reverse();
            Ok(Value::Unit)
        }
        "chunks" => {
            let n = args
                .first()
                .and_then(|v| match v {
                    Value::Integer(n) => Some(*n as usize),
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
        "windows" => {
            let n = args
                .first()
                .and_then(|v| match v {
                    Value::Integer(n) => Some(*n as usize),
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
        "sort_by" | "sort_by_key" => {
            Err(format!("{}: closure methods not supported in VM yet", method))
        }
        _ => Err(format!("no method '{}' on type Vec", method)),
    }
}
