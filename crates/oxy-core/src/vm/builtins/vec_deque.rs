//! VecDeque method implementations — shared by interpreter and VM.
use crate::symbols;
use crate::types::Value;
use std::cell::RefCell;
use std::rc::Rc;

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let Value::VecDeque(rc) = &receiver else {
        unreachable!()
    };
    let d = rc.borrow();
    match method {
        symbols::vecdeque_m::LEN => Ok(Value::I64(d.len() as i64)),
        symbols::vecdeque_m::IS_EMPTY => Ok(Value::Bool(d.is_empty())),
        symbols::vecdeque_m::FRONT => d
            .front()
            .cloned()
            .ok_or_else(|| "VecDeque::front on empty deque".into()),
        symbols::vecdeque_m::BACK => d
            .back()
            .cloned()
            .ok_or_else(|| "VecDeque::back on empty deque".into()),
        symbols::vecdeque_m::PUSH_FRONT => {
            drop(d);
            rc.borrow_mut()
                .push_front(args.first().cloned().unwrap_or(Value::Unit));
            Ok(Value::Unit)
        }
        symbols::vecdeque_m::PUSH_BACK => {
            drop(d);
            rc.borrow_mut()
                .push_back(args.first().cloned().unwrap_or(Value::Unit));
            Ok(Value::Unit)
        }
        symbols::vecdeque_m::POP_FRONT => {
            drop(d);
            match rc.borrow_mut().pop_front() {
                Some(val) => Ok(Value::some(val)),
                None => Ok(Value::none()),
            }
        }
        symbols::vecdeque_m::POP_BACK => {
            drop(d);
            match rc.borrow_mut().pop_back() {
                Some(val) => Ok(Value::some(val)),
                None => Ok(Value::none()),
            }
        }
        symbols::vecdeque_m::TO_VEC => Ok(Value::Vec(Rc::new(RefCell::new(
            d.iter().cloned().collect(),
        )))),
        symbols::vecdeque_m::CLONE => Ok(Value::VecDeque(Rc::clone(rc))),
        _ => Err(format!("no method '{}' on type VecDeque", method)),
    }
}

pub fn method_names() -> &'static [&'static str] {
    &[
        symbols::vecdeque_m::LEN,
        symbols::vecdeque_m::IS_EMPTY,
        symbols::vecdeque_m::FRONT,
        symbols::vecdeque_m::BACK,
        symbols::vecdeque_m::PUSH_FRONT,
        symbols::vecdeque_m::PUSH_BACK,
        symbols::vecdeque_m::POP_FRONT,
        symbols::vecdeque_m::POP_BACK,
        symbols::vecdeque_m::TO_VEC,
        symbols::vecdeque_m::CLONE,
    ]
}
