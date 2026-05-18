//! HashMap method implementations — shared by interpreter and VM.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::types::Value;

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let Value::HashMap(rc) = &receiver else {
        unreachable!()
    };
    let rc = rc.clone();
    match method {
        "len" => Ok(Value::Integer(rc.borrow().len() as i64)),
        "is_empty" => Ok(Value::Bool(rc.borrow().is_empty())),
        "get" => {
            let key = args.first().cloned().unwrap_or(Value::Unit);
            Ok(rc.borrow().get(&key).cloned().unwrap_or(Value::Unit))
        }
        "get_or" => {
            let key = args.first().cloned().unwrap_or(Value::Unit);
            let default = args.get(1).cloned().unwrap_or(Value::Unit);
            Ok(rc.borrow().get(&key).cloned().unwrap_or(default))
        }
        "contains_key" => {
            let key = args.first().cloned().unwrap_or(Value::Unit);
            Ok(Value::Bool(rc.borrow().contains_key(&key)))
        }
        "insert" => {
            let key = args.first().cloned().unwrap_or(Value::Unit);
            let val = args.get(1).cloned().unwrap_or(Value::Unit);
            rc.borrow_mut().insert(key, val);
            Ok(Value::Unit)
        }
        "remove" => {
            let key = args.first().cloned().unwrap_or(Value::Unit);
            rc.borrow_mut().remove(&key);
            Ok(Value::Unit)
        }
        "keys" => {
            let keys: Vec<Value> = rc.borrow().keys().cloned().collect();
            Ok(Value::Vec(Rc::new(RefCell::new(keys))))
        }
        "values" => {
            let values: Vec<Value> = rc.borrow().values().cloned().collect();
            Ok(Value::Vec(Rc::new(RefCell::new(values))))
        }
        "clone" => Ok(Value::HashMap(Rc::new(RefCell::new(
            rc.borrow().clone(),
        )))),
        _ => Err(format!("no method '{}' on type HashMap", method)),
    }
}

/// Helper to build a HashMap value from Rust types.
pub fn from_iter(entries: impl IntoIterator<Item = (Value, Value)>) -> Value {
    let mut m = HashMap::new();
    for (k, v) in entries {
        m.insert(k, v);
    }
    Value::HashMap(Rc::new(RefCell::new(m)))
}
