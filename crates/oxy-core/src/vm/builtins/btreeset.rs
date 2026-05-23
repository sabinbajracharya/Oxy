//! BTreeSet method implementations — shared by interpreter and VM.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use crate::symbols;
use crate::types::Value;

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let Value::BTreeSet(rc) = &receiver else {
        unreachable!()
    };
    let rc = rc.clone();
    match method {
        symbols::btreeset_m::LEN => Ok(Value::I64(rc.borrow().len() as i64)),
        symbols::btreeset_m::IS_EMPTY => Ok(Value::Bool(rc.borrow().is_empty())),
        symbols::btreeset_m::CONTAINS => {
            let val = args.first().cloned().unwrap_or(Value::Unit);
            Ok(Value::Bool(rc.borrow().contains(&val)))
        }
        symbols::btreeset_m::INSERT => {
            let val = args.first().cloned().unwrap_or(Value::Unit);
            let was_new = rc.borrow_mut().insert(val);
            Ok(Value::Bool(was_new))
        }
        symbols::btreeset_m::REMOVE => {
            let val = args.first().cloned().unwrap_or(Value::Unit);
            let existed = rc.borrow_mut().remove(&val);
            Ok(Value::Bool(existed))
        }
        symbols::btreeset_m::TO_VEC => {
            let v: Vec<Value> = rc.borrow().iter().cloned().collect();
            Ok(Value::Vec(Rc::new(RefCell::new(v))))
        }
        symbols::btreeset_m::UNION => {
            let other = args.first().cloned().unwrap_or(Value::Unit);
            if let Value::BTreeSet(other_rc) = other {
                let union: BTreeSet<Value> =
                    rc.borrow().union(&other_rc.borrow()).cloned().collect();
                Ok(Value::BTreeSet(Rc::new(RefCell::new(union))))
            } else {
                Err("BTreeSet::union requires a BTreeSet argument".into())
            }
        }
        symbols::btreeset_m::INTERSECTION => {
            let other = args.first().cloned().unwrap_or(Value::Unit);
            if let Value::BTreeSet(other_rc) = other {
                let intersection: BTreeSet<Value> = rc
                    .borrow()
                    .intersection(&other_rc.borrow())
                    .cloned()
                    .collect();
                Ok(Value::BTreeSet(Rc::new(RefCell::new(intersection))))
            } else {
                Err("BTreeSet::intersection requires a BTreeSet argument".into())
            }
        }
        symbols::btreeset_m::DIFFERENCE => {
            let other = args.first().cloned().unwrap_or(Value::Unit);
            if let Value::BTreeSet(other_rc) = other {
                let difference: BTreeSet<Value> = rc
                    .borrow()
                    .difference(&other_rc.borrow())
                    .cloned()
                    .collect();
                Ok(Value::BTreeSet(Rc::new(RefCell::new(difference))))
            } else {
                Err("BTreeSet::difference requires a BTreeSet argument".into())
            }
        }
        symbols::btreeset_m::CLONE => {
            Ok(Value::BTreeSet(Rc::new(RefCell::new(rc.borrow().clone()))))
        }
        _ => Err(format!("no method '{}' on type BTreeSet", method)),
    }
}

pub fn method_names() -> &'static [&'static str] {
    &[
        symbols::btreeset_m::LEN,
        symbols::btreeset_m::IS_EMPTY,
        symbols::btreeset_m::CONTAINS,
        symbols::btreeset_m::INSERT,
        symbols::btreeset_m::REMOVE,
        symbols::btreeset_m::TO_VEC,
        symbols::btreeset_m::UNION,
        symbols::btreeset_m::INTERSECTION,
        symbols::btreeset_m::DIFFERENCE,
        symbols::btreeset_m::CLONE,
    ]
}
