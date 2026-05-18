//! BinaryHeap method implementations — shared by interpreter and VM.
use std::collections::BinaryHeap;
use crate::types::Value;

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let Value::BinaryHeap(h) = &receiver else { unreachable!() };
    match method {
        "len" => Ok(Value::Integer(h.len() as i64)),
        "is_empty" => Ok(Value::Bool(h.is_empty())),
        "peek" => match h.peek() {
            Some(val) => Ok(Value::some(val.clone())),
            None => Ok(Value::none()),
        },
        "push" => {
            let mut new = h.clone();
            new.push(args.first().cloned().unwrap_or(Value::Unit));
            Ok(Value::Tuple(vec![Value::BinaryHeap(new), Value::Unit]))
        }
        "pop" => {
            let mut new = h.clone();
            match new.pop() {
                Some(val) => Ok(Value::Tuple(vec![Value::BinaryHeap(new), Value::some(val)])),
                None => Ok(Value::Tuple(vec![Value::BinaryHeap(new), Value::none()])),
            }
        }
        "to_vec" => Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(h.clone().into_sorted_vec())))),
        "clone" => Ok(Value::BinaryHeap(h.clone())),
        _ => Err(format!("no method '{}' on type BinaryHeap", method)),
    }
}
