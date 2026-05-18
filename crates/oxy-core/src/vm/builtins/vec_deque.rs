//! VecDeque method implementations — shared by interpreter and VM.
use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::RefCell;
use crate::types::Value;

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let Value::VecDeque(d) = &receiver else { unreachable!() };
    match method {
        "len" => Ok(Value::Integer(d.len() as i64)),
        "is_empty" => Ok(Value::Bool(d.is_empty())),
        "front" => d.front().cloned().ok_or_else(|| "VecDeque::front on empty deque".into()),
        "back" => d.back().cloned().ok_or_else(|| "VecDeque::back on empty deque".into()),
        "push_front" => {
            let mut new = d.clone();
            new.push_front(args.first().cloned().unwrap_or(Value::Unit));
            Ok(Value::Tuple(vec![Value::VecDeque(new), Value::Unit]))
        }
        "push_back" => {
            let mut new = d.clone();
            new.push_back(args.first().cloned().unwrap_or(Value::Unit));
            Ok(Value::Tuple(vec![Value::VecDeque(new), Value::Unit]))
        }
        "pop_front" => {
            let mut new = d.clone();
            let popped = new.pop_front().unwrap_or(Value::Unit);
            Ok(Value::Tuple(vec![Value::VecDeque(new), popped]))
        }
        "pop_back" => {
            let mut new = d.clone();
            let popped = new.pop_back().unwrap_or(Value::Unit);
            Ok(Value::Tuple(vec![Value::VecDeque(new), popped]))
        }
        "to_vec" => Ok(Value::Vec(Rc::new(RefCell::new(d.iter().cloned().collect())))),
        "clone" => Ok(Value::VecDeque(d.clone())),
        _ => Err(format!("no method '{}' on type VecDeque", method)),
    }
}
