//! BTreeMap method implementations — shared by interpreter and VM.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::symbols;
use crate::types::Value;

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let Value::BTreeMap(rc) = &receiver else {
        unreachable!()
    };
    let rc = rc.clone();
    match method {
        symbols::btreemap_m::LEN => Ok(Value::I64(rc.borrow().len() as i64)),
        symbols::btreemap_m::IS_EMPTY => Ok(Value::Bool(rc.borrow().is_empty())),
        symbols::btreemap_m::GET => {
            let key = args.first().cloned().unwrap_or(Value::Unit);
            match rc.borrow().get(&key).cloned() {
                Some(val) => Ok(Value::some(val)),
                None => Ok(Value::none()),
            }
        }
        symbols::btreemap_m::GET_OR => {
            let key = args.first().cloned().unwrap_or(Value::Unit);
            let default = args.get(1).cloned().unwrap_or(Value::Unit);
            match rc.borrow().get(&key).cloned() {
                Some(val) => Ok(val),
                None => Ok(default),
            }
        }
        symbols::btreemap_m::CONTAINS_KEY => {
            let key = args.first().cloned().unwrap_or(Value::Unit);
            Ok(Value::Bool(rc.borrow().contains_key(&key)))
        }
        symbols::btreemap_m::INSERT => {
            let key = args.first().cloned().unwrap_or(Value::Unit);
            let val = args.get(1).cloned().unwrap_or(Value::Unit);
            let old = rc.borrow_mut().insert(key, val);
            match old {
                Some(v) => Ok(Value::some(v)),
                None => Ok(Value::none()),
            }
        }
        symbols::btreemap_m::REMOVE => {
            let key = args.first().cloned().unwrap_or(Value::Unit);
            match rc.borrow_mut().remove(&key) {
                Some(val) => Ok(Value::some(val)),
                None => Ok(Value::none()),
            }
        }
        symbols::btreemap_m::KEYS => {
            let keys: Vec<Value> = rc.borrow().keys().cloned().collect();
            Ok(Value::Vec(Rc::new(RefCell::new(keys))))
        }
        symbols::btreemap_m::VALUES => {
            let values: Vec<Value> = rc.borrow().values().cloned().collect();
            Ok(Value::Vec(Rc::new(RefCell::new(values))))
        }
        symbols::btreemap_m::CLONE => {
            Ok(Value::BTreeMap(Rc::new(RefCell::new(rc.borrow().clone()))))
        }
        _ => Err(format!("no method '{}' on type BTreeMap", method)),
    }
}

pub fn method_names() -> &'static [&'static str] {
    &[
        symbols::btreemap_m::LEN,
        symbols::btreemap_m::IS_EMPTY,
        symbols::btreemap_m::GET,
        symbols::btreemap_m::GET_OR,
        symbols::btreemap_m::CONTAINS_KEY,
        symbols::btreemap_m::INSERT,
        symbols::btreemap_m::REMOVE,
        symbols::btreemap_m::KEYS,
        symbols::btreemap_m::VALUES,
        symbols::btreemap_m::CLONE,
    ]
}

/// Helper to build a BTreeMap value from Rust types.
pub fn from_iter(entries: impl IntoIterator<Item = (Value, Value)>) -> Value {
    let mut m = BTreeMap::new();
    for (k, v) in entries {
        m.insert(k, v);
    }
    Value::BTreeMap(Rc::new(RefCell::new(m)))
}
