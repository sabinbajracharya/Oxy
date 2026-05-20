//! BinaryHeap method implementations — shared by interpreter and VM.
use crate::symbols;
use crate::types::Value;

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let Value::BinaryHeap(rc) = &receiver else {
        unreachable!()
    };
    let h = rc.borrow();
    match method {
        symbols::binaryheap_m::LEN => Ok(Value::I64(h.len() as i64)),
        symbols::binaryheap_m::IS_EMPTY => Ok(Value::Bool(h.is_empty())),
        symbols::binaryheap_m::PEEK => match h.peek() {
            Some(val) => Ok(Value::some(val.clone())),
            None => Ok(Value::none()),
        },
        symbols::binaryheap_m::PUSH => {
            drop(h);
            rc.borrow_mut()
                .push(args.first().cloned().unwrap_or(Value::Unit));
            Ok(Value::Unit)
        }
        symbols::binaryheap_m::POP => {
            drop(h);
            match rc.borrow_mut().pop() {
                Some(val) => Ok(Value::some(val)),
                None => Ok(Value::none()),
            }
        }
        symbols::binaryheap_m::TO_VEC => Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(
            h.clone().into_sorted_vec(),
        )))),
        symbols::binaryheap_m::CLONE => Ok(Value::BinaryHeap(std::rc::Rc::clone(rc))),
        _ => Err(format!("no method '{}' on type BinaryHeap", method)),
    }
}

pub fn method_names() -> &'static [&'static str] {
    &[
        symbols::binaryheap_m::LEN,
        symbols::binaryheap_m::IS_EMPTY,
        symbols::binaryheap_m::PEEK,
        symbols::binaryheap_m::PUSH,
        symbols::binaryheap_m::POP,
        symbols::binaryheap_m::TO_VEC,
        symbols::binaryheap_m::CLONE,
    ]
}
