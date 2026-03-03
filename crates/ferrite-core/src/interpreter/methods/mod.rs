//! Method dispatch for built-in types.
//!
//! Routes `.method()` calls on values to the appropriate type-specific
//! handler (Vec, String, HashMap, Option/Result, numeric, HTTP types).

mod hashmap;
mod numeric;
mod option_result;
mod string;
mod vec;

use crate::ast::Expr;
use crate::env::Env;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::{Value, OPTION_TYPE, RESULT_TYPE};

use super::Interpreter;

impl Interpreter {
    /// Top-level method dispatch: route a `.method()` call to the right handler.
    ///
    /// Dispatch order: Vec → String → HashMap → Option/Result →
    /// HTTP types → user impl methods → numeric methods → to_json fallback.
    pub(crate) fn call_method(
        &mut self,
        receiver: Value,
        method: &str,
        args: Vec<Value>,
        receiver_expr: &Expr,
        env: &Env,
        span: &Span,
    ) -> Result<Value, FerriError> {
        match &receiver {
            Value::Vec(_) => self.call_vec_method(receiver, method, args, receiver_expr, env, span),
            Value::String(_) => self.call_string_method(receiver, method, args, span),
            Value::HashMap(_) => {
                self.call_hashmap_method(receiver, method, args, receiver_expr, env, span)
            }
            Value::Tuple(_) => self.try_to_json_method(receiver, method, span, "tuple"),
            Value::Struct { name, .. }
            | Value::EnumVariant {
                enum_name: name, ..
            } => {
                // Built-in Option/Result methods
                if let Value::EnumVariant { enum_name, .. } = &receiver {
                    if enum_name == OPTION_TYPE || enum_name == RESULT_TYPE {
                        if let Some(result) =
                            self.call_option_result_method(&receiver, method, &args, span)?
                        {
                            return Ok(result);
                        }
                    }
                }
                // Built-in HttpResponse methods
                if let Value::Struct {
                    name: sname,
                    fields,
                } = &receiver
                {
                    if sname == "HttpResponse" {
                        return self.call_http_response_method(fields, method, span);
                    }
                    if sname == "HttpRequestBuilder" {
                        return self.call_http_builder_method(fields.clone(), method, &args, span);
                    }
                }
                let type_name = name.clone();
                self.call_user_method(receiver, &type_name, method, args, receiver_expr, env, span)
            }
            Value::Integer(_) | Value::Float(_) => {
                self.call_numeric_method(receiver, method, args, span)
            }
            _ => {
                // Built-in .to_json() and .to_json_pretty() on all values
                if method == "to_json" || method == "to_json_pretty" {
                    let result = if method == "to_json" {
                        crate::json::serialize(&receiver)
                    } else {
                        crate::json::serialize_pretty(&receiver)
                    };
                    return match result {
                        Ok(json) => Ok(Value::ok(Value::String(json))),
                        Err(e) => Ok(Value::err(Value::String(e))),
                    };
                }
                Err(FerriError::Runtime {
                    message: format!("no method `{method}` on type {}", receiver.type_name()),
                    line: span.line,
                    column: span.column,
                })
            }
        }
    }
}
