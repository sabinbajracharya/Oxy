//! HashMap method implementations — shared by interpreter and VM.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::symbols;
use crate::types::Value;

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let Value::HashMap(rc) = &receiver else {
        unreachable!()
    };
    let rc = rc.clone();
    match method {
        symbols::hashmap_m::LEN => Ok(Value::I64(rc.borrow().len() as i64)),
        symbols::hashmap_m::IS_EMPTY => Ok(Value::Bool(rc.borrow().is_empty())),
        symbols::hashmap_m::GET => {
            let key = args.first().cloned().unwrap_or(Value::Unit);
            match rc.borrow().get(&key).cloned() {
                Some(val) => Ok(Value::some(val)),
                None => Ok(Value::none()),
            }
        }
        symbols::hashmap_m::GET_OR => {
            let key = args.first().cloned().unwrap_or(Value::Unit);
            let default = args.get(1).cloned().unwrap_or(Value::Unit);
            match rc.borrow().get(&key).cloned() {
                Some(val) => Ok(val),
                None => Ok(default),
            }
        }
        symbols::hashmap_m::CONTAINS_KEY => {
            let key = args.first().cloned().unwrap_or(Value::Unit);
            Ok(Value::Bool(rc.borrow().contains_key(&key)))
        }
        symbols::hashmap_m::INSERT => {
            let key = args.first().cloned().unwrap_or(Value::Unit);
            let val = args.get(1).cloned().unwrap_or(Value::Unit);
            let old = rc.borrow_mut().insert(key, val);
            match old {
                Some(v) => Ok(Value::some(v)),
                None => Ok(Value::none()),
            }
        }
        symbols::hashmap_m::REMOVE => {
            let key = args.first().cloned().unwrap_or(Value::Unit);
            match rc.borrow_mut().remove(&key) {
                Some(val) => Ok(Value::some(val)),
                None => Ok(Value::none()),
            }
        }
        symbols::hashmap_m::KEYS => {
            let mut keys: Vec<Value> = rc.borrow().keys().cloned().collect();
            keys.sort();
            Ok(Value::Vec(Rc::new(RefCell::new(keys))))
        }
        symbols::hashmap_m::VALUES => {
            let mut values: Vec<Value> = rc.borrow().values().cloned().collect();
            values.sort();
            Ok(Value::Vec(Rc::new(RefCell::new(values))))
        }
        symbols::hashmap_m::CLONE => Ok(Value::HashMap(Rc::new(RefCell::new(rc.borrow().clone())))),
        _ => Err(format!("no method '{}' on type HashMap", method)),
    }
}

pub fn method_names() -> &'static [&'static str] {
    &[
        symbols::hashmap_m::LEN,
        symbols::hashmap_m::IS_EMPTY,
        symbols::hashmap_m::GET,
        symbols::hashmap_m::GET_OR,
        symbols::hashmap_m::CONTAINS_KEY,
        symbols::hashmap_m::INSERT,
        symbols::hashmap_m::REMOVE,
        symbols::hashmap_m::KEYS,
        symbols::hashmap_m::VALUES,
        symbols::hashmap_m::CLONE,
    ]
}

/// Helper to build a HashMap value from Rust types.
pub fn from_iter(entries: impl IntoIterator<Item = (Value, Value)>) -> Value {
    let mut m = HashMap::new();
    for (k, v) in entries {
        m.insert(k, v);
    }
    Value::HashMap(Rc::new(RefCell::new(m)))
}
