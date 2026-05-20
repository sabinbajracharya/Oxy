//! HashSet method implementations — shared by interpreter and VM.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::symbols;
use crate::types::Value;

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let Value::HashSet(rc) = &receiver else {
        unreachable!()
    };
    let rc = rc.clone();
    match method {
        symbols::hashset_m::LEN => Ok(Value::I64(rc.borrow().len() as i64)),
        symbols::hashset_m::IS_EMPTY => Ok(Value::Bool(rc.borrow().is_empty())),
        symbols::hashset_m::CONTAINS => {
            let val = args.first().cloned().unwrap_or(Value::Unit);
            Ok(Value::Bool(rc.borrow().contains(&val)))
        }
        symbols::hashset_m::INSERT => {
            let val = args.first().cloned().unwrap_or(Value::Unit);
            let was_new = rc.borrow_mut().insert(val);
            Ok(Value::Bool(was_new))
        }
        symbols::hashset_m::REMOVE => {
            let val = args.first().cloned().unwrap_or(Value::Unit);
            let existed = rc.borrow_mut().remove(&val);
            Ok(Value::Bool(existed))
        }
        symbols::hashset_m::TO_VEC => {
            let s = rc.borrow();
            let mut v: Vec<Value> = s.iter().cloned().collect();
            v.sort();
            Ok(Value::Vec(Rc::new(RefCell::new(v))))
        }
        symbols::hashset_m::UNION => {
            let other = args.first().cloned().unwrap_or(Value::Unit);
            if let Value::HashSet(other_rc) = other {
                let union: HashSet<Value> =
                    rc.borrow().union(&other_rc.borrow()).cloned().collect();
                Ok(Value::HashSet(Rc::new(RefCell::new(union))))
            } else {
                Err("HashSet::union requires a HashSet argument".into())
            }
        }
        symbols::hashset_m::INTERSECTION => {
            let other = args.first().cloned().unwrap_or(Value::Unit);
            if let Value::HashSet(other_rc) = other {
                let intersection: HashSet<Value> = rc
                    .borrow()
                    .intersection(&other_rc.borrow())
                    .cloned()
                    .collect();
                Ok(Value::HashSet(Rc::new(RefCell::new(intersection))))
            } else {
                Err("HashSet::intersection requires a HashSet argument".into())
            }
        }
        symbols::hashset_m::DIFFERENCE => {
            let other = args.first().cloned().unwrap_or(Value::Unit);
            if let Value::HashSet(other_rc) = other {
                let difference: HashSet<Value> = rc
                    .borrow()
                    .difference(&other_rc.borrow())
                    .cloned()
                    .collect();
                Ok(Value::HashSet(Rc::new(RefCell::new(difference))))
            } else {
                Err("HashSet::difference requires a HashSet argument".into())
            }
        }
        symbols::hashset_m::CLONE => Ok(Value::HashSet(Rc::new(RefCell::new(rc.borrow().clone())))),
        _ => Err(format!("no method '{}' on type HashSet", method)),
    }
}

pub fn method_names() -> &'static [&'static str] {
    &[
        symbols::hashset_m::LEN,
        symbols::hashset_m::IS_EMPTY,
        symbols::hashset_m::CONTAINS,
        symbols::hashset_m::INSERT,
        symbols::hashset_m::REMOVE,
        symbols::hashset_m::TO_VEC,
        symbols::hashset_m::UNION,
        symbols::hashset_m::INTERSECTION,
        symbols::hashset_m::DIFFERENCE,
        symbols::hashset_m::CLONE,
    ]
}
