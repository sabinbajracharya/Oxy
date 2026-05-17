//! VecDeque method implementations.
//!
//! Supports: push_front, push_back, pop_front, pop_back, front, back, len, is_empty, clone, to_vec.

use crate::ast::Expr;
use crate::env::Env;
use crate::errors::check_arg_count;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

use super::super::Interpreter;

impl Interpreter {
    /// Handle method calls on VecDeque values.
    pub(crate) fn call_vec_deque_method(
        &mut self,
        receiver: Value,
        method: &str,
        args: Vec<Value>,
        receiver_expr: &Expr,
        env: &Env,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let Value::VecDeque(d) = receiver else {
            unreachable!()
        };
        match method {
            "len" => Ok(Value::Integer(d.len() as i64)),
            "is_empty" => Ok(Value::Bool(d.is_empty())),
            "front" => match d.front() {
                Some(val) => Ok(val.clone()),
                None => Err(FerriError::Runtime {
                    message: "VecDeque::front called on empty deque".into(),
                    line: span.line,
                    column: span.column,
                }),
            },
            "back" => match d.back() {
                Some(val) => Ok(val.clone()),
                None => Err(FerriError::Runtime {
                    message: "VecDeque::back called on empty deque".into(),
                    line: span.line,
                    column: span.column,
                }),
            },
            "push_front" => {
                check_arg_count("VecDeque::push_front", 1, &args, span)?;
                let val = args.into_iter().next().unwrap();
                let mut new_d = d;
                new_d.push_front(val);
                self.mutate_variable(receiver_expr, Value::VecDeque(new_d), env, span)?;
                Ok(Value::Unit)
            }
            "push_back" => {
                check_arg_count("VecDeque::push_back", 1, &args, span)?;
                let val = args.into_iter().next().unwrap();
                let mut new_d = d;
                new_d.push_back(val);
                self.mutate_variable(receiver_expr, Value::VecDeque(new_d), env, span)?;
                Ok(Value::Unit)
            }
            "pop_front" => {
                let mut new_d = d;
                let popped = new_d.pop_front();
                self.mutate_variable(receiver_expr, Value::VecDeque(new_d), env, span)?;
                match popped {
                    Some(val) => Ok(Value::some(val)),
                    None => Ok(Value::none()),
                }
            }
            "pop_back" => {
                let mut new_d = d;
                let popped = new_d.pop_back();
                self.mutate_variable(receiver_expr, Value::VecDeque(new_d), env, span)?;
                match popped {
                    Some(val) => Ok(Value::some(val)),
                    None => Ok(Value::none()),
                }
            }
            "clone" => Ok(Value::VecDeque(d)),
            "to_vec" => Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(d.into_iter().collect())))),
            _ => self.try_to_json_method(Value::VecDeque(d), method, span, "VecDeque"),
        }
    }
}
