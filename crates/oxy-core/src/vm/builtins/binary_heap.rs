//! BinaryHeap method implementations — shared by interpreter and VM.
use crate::types::Value;
use std::collections::BinaryHeap;

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let Value::BinaryHeap(rc) = &receiver else {
        unreachable!()
    };
    let h = rc.borrow();
    match method {
        "len" => Ok(Value::I64(h.len() as i64)),
        "is_empty" => Ok(Value::Bool(h.is_empty())),
        "peek" => match h.peek() {
            Some(val) => Ok(Value::some(val.clone())),
            None => Ok(Value::none()),
        },
        "push" => {
            drop(h);
            rc.borrow_mut()
                .push(args.first().cloned().unwrap_or(Value::Unit));
            Ok(Value::Unit)
        }
        "pop" => {
            drop(h);
            match rc.borrow_mut().pop() {
                Some(val) => Ok(Value::some(val)),
                None => Ok(Value::none()),
            }
        }
        "to_vec" => Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(
            h.clone().into_sorted_vec(),
        )))),
        "clone" => Ok(Value::BinaryHeap(std::rc::Rc::clone(rc))),
        _ => Err(format!("no method '{}' on type BinaryHeap", method)),
    }
}
