//! BinaryHeap method implementations (max-heap by default).
//!
//! Supports: push, pop, peek, len, is_empty, clone, to_vec.

use crate::ast::Expr;
use crate::env::Env;
use crate::errors::check_arg_count;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

use super::super::Interpreter;

impl Interpreter {
    /// Handle method calls on BinaryHeap values.
    pub(crate) fn call_binary_heap_method(
        &mut self,
        receiver: Value,
        method: &str,
        args: Vec<Value>,
        receiver_expr: &Expr,
        env: &Env,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let Value::BinaryHeap(rc) = &receiver else {
            unreachable!()
        };
        let h = rc.borrow();
        match method {
            "len" => Ok(Value::Integer(h.len() as i64)),
            "is_empty" => Ok(Value::Bool(h.is_empty())),
            "peek" => match h.peek() {
                Some(val) => Ok(Value::some(val.clone())),
                None => Ok(Value::none()),
            },
            "push" => {
                check_arg_count("BinaryHeap::push", 1, &args, span)?;
                let val = args.into_iter().next().unwrap();
                drop(h);
                rc.borrow_mut().push(val);
                Ok(Value::Unit)
            }
            "pop" => {
                drop(h);
                match rc.borrow_mut().pop() {
                    Some(val) => Ok(Value::some(val)),
                    None => Ok(Value::none()),
                }
            }
            "clone" => Ok(Value::BinaryHeap(std::rc::Rc::clone(rc))),
            "to_vec" => Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(
                h.clone().into_sorted_vec(),
            )))),
            _ => self.try_to_json_method(Value::BinaryHeap(std::rc::Rc::clone(rc)), method, span, "BinaryHeap"),
        }
    }
}
