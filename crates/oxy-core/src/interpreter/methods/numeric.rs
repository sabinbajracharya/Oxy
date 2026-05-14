//! Numeric method implementations for Integer and Float values.
//!
//! Supports: abs, sqrt, floor, ceil, round, pow, sin, cos, tan,
//! min, max, clamp, to_json, to_json_pretty.

use crate::errors::check_arg_count;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

use super::super::Interpreter;

impl Interpreter {
    /// Handle method calls on Integer and Float values.
    pub(crate) fn call_numeric_method(
        &self,
        receiver: Value,
        method: &str,
        args: Vec<Value>,
        span: &Span,
    ) -> Result<Value, FerriError> {
        match method {
            "abs" => match &receiver {
                Value::Integer(n) => Ok(Value::Integer(n.abs())),
                Value::Float(f) => Ok(crate::stdlib::math::float_to_value(f.abs())),
                _ => unreachable!(),
            },
            "sqrt" => {
                let x = crate::stdlib::math::value_to_f64(&receiver, span)?;
                Ok(crate::stdlib::math::float_to_value(x.sqrt()))
            }
            "floor" => {
                let x = crate::stdlib::math::value_to_f64(&receiver, span)?;
                Ok(crate::stdlib::math::float_to_value(x.floor()))
            }
            "ceil" => {
                let x = crate::stdlib::math::value_to_f64(&receiver, span)?;
                Ok(crate::stdlib::math::float_to_value(x.ceil()))
            }
            "round" => {
                let x = crate::stdlib::math::value_to_f64(&receiver, span)?;
                Ok(crate::stdlib::math::float_to_value(x.round()))
            }
            "pow" => {
                check_arg_count("pow", 1, &args, span)?;
                let base = crate::stdlib::math::value_to_f64(&receiver, span)?;
                let exp = crate::stdlib::math::value_to_f64(&args[0], span)?;
                Ok(crate::stdlib::math::float_to_value(base.powf(exp)))
            }
            "sin" => {
                let x = crate::stdlib::math::value_to_f64(&receiver, span)?;
                Ok(crate::stdlib::math::float_to_value(x.sin()))
            }
            "cos" => {
                let x = crate::stdlib::math::value_to_f64(&receiver, span)?;
                Ok(crate::stdlib::math::float_to_value(x.cos()))
            }
            "tan" => {
                let x = crate::stdlib::math::value_to_f64(&receiver, span)?;
                Ok(crate::stdlib::math::float_to_value(x.tan()))
            }
            "min" => {
                check_arg_count("min", 1, &args, span)?;
                match (&receiver, &args[0]) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(*a.min(b))),
                    _ => {
                        let a = crate::stdlib::math::value_to_f64(&receiver, span)?;
                        let b = crate::stdlib::math::value_to_f64(&args[0], span)?;
                        Ok(crate::stdlib::math::float_to_value(a.min(b)))
                    }
                }
            }
            "max" => {
                check_arg_count("max", 1, &args, span)?;
                match (&receiver, &args[0]) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(*a.max(b))),
                    _ => {
                        let a = crate::stdlib::math::value_to_f64(&receiver, span)?;
                        let b = crate::stdlib::math::value_to_f64(&args[0], span)?;
                        Ok(crate::stdlib::math::float_to_value(a.max(b)))
                    }
                }
            }
            "clamp" => {
                check_arg_count("clamp", 2, &args, span)?;
                match (&receiver, &args[0], &args[1]) {
                    (Value::Integer(x), Value::Integer(min), Value::Integer(max)) => {
                        Ok(Value::Integer(*x.max(min).min(max)))
                    }
                    _ => {
                        let x = crate::stdlib::math::value_to_f64(&receiver, span)?;
                        let min = crate::stdlib::math::value_to_f64(&args[0], span)?;
                        let max = crate::stdlib::math::value_to_f64(&args[1], span)?;
                        Ok(crate::stdlib::math::float_to_value(x.max(min).min(max)))
                    }
                }
            }
            _ => {
                // Fall back to to_json methods
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
