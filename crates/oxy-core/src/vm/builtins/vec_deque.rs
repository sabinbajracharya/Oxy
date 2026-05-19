//! VecDeque method implementations — shared by interpreter and VM.
use crate::types::Value;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let Value::VecDeque(rc) = &receiver else {
        unreachable!()
    };
    let d = rc.borrow();
    match method {
        "len" => Ok(Value::I64(d.len() as i64)),
        "is_empty" => Ok(Value::Bool(d.is_empty())),
        "front" => d
            .front()
            .cloned()
            .ok_or_else(|| "VecDeque::front on empty deque".into()),
        "back" => d
            .back()
            .cloned()
            .ok_or_else(|| "VecDeque::back on empty deque".into()),
        "push_front" => {
            drop(d);
            rc.borrow_mut()
                .push_front(args.first().cloned().unwrap_or(Value::Unit));
            Ok(Value::Unit)
        }
        "push_back" => {
            drop(d);
            rc.borrow_mut()
                .push_back(args.first().cloned().unwrap_or(Value::Unit));
            Ok(Value::Unit)
        }
        "pop_front" => {
            drop(d);
            match rc.borrow_mut().pop_front() {
                Some(val) => Ok(Value::some(val)),
                None => Ok(Value::none()),
            }
        }
        "pop_back" => {
            drop(d);
            match rc.borrow_mut().pop_back() {
                Some(val) => Ok(Value::some(val)),
                None => Ok(Value::none()),
            }
        }
        "to_vec" => Ok(Value::Vec(Rc::new(RefCell::new(
            d.iter().cloned().collect(),
        )))),
        "clone" => Ok(Value::VecDeque(Rc::clone(rc))),
        _ => Err(format!("no method '{}' on type VecDeque", method)),
    }
}
