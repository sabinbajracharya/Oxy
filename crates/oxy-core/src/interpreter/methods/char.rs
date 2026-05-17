//! Char method implementations.
//!
//! Supports: is_digit, is_alphabetic, is_alphanumeric, is_whitespace,
//!           is_lowercase, is_uppercase, is_ascii, to_uppercase, to_lowercase,
//!           code, from_code, clone.

use crate::errors::check_arg_count;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

use super::super::Interpreter;

impl Interpreter {
    /// Handle method calls on Char values.
    pub(crate) fn call_char_method(
        &mut self,
        receiver: Value,
        method: &str,
        args: Vec<Value>,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let Value::Char(c) = receiver else {
            unreachable!()
        };
        match method {
            "is_digit" => {
                check_arg_count("char::is_digit", 0, &args, span)?;
                Ok(Value::Bool(c.is_ascii_digit()))
            }
            "is_alphabetic" => {
                check_arg_count("char::is_alphabetic", 0, &args, span)?;
                Ok(Value::Bool(c.is_alphabetic()))
            }
            "is_alphanumeric" => {
                check_arg_count("char::is_alphanumeric", 0, &args, span)?;
                Ok(Value::Bool(c.is_alphanumeric()))
            }
            "is_whitespace" => {
                check_arg_count("char::is_whitespace", 0, &args, span)?;
                Ok(Value::Bool(c.is_whitespace()))
            }
            "is_lowercase" => {
                check_arg_count("char::is_lowercase", 0, &args, span)?;
                Ok(Value::Bool(c.is_lowercase()))
            }
            "is_uppercase" => {
                check_arg_count("char::is_uppercase", 0, &args, span)?;
                Ok(Value::Bool(c.is_uppercase()))
            }
            "is_ascii" => {
                check_arg_count("char::is_ascii", 0, &args, span)?;
                Ok(Value::Bool(c.is_ascii()))
            }
            "to_uppercase" => {
                check_arg_count("char::to_uppercase", 0, &args, span)?;
                let upper: String = c.to_uppercase().collect();
                if upper.len() == 1 {
                    Ok(Value::Char(upper.chars().next().unwrap()))
                } else {
                    Ok(Value::String(upper))
                }
            }
            "to_lowercase" => {
                check_arg_count("char::to_lowercase", 0, &args, span)?;
                let lower: String = c.to_lowercase().collect();
                if lower.len() == 1 {
                    Ok(Value::Char(lower.chars().next().unwrap()))
                } else {
                    Ok(Value::String(lower))
                }
            }
            "clone" => Ok(Value::Char(c)),
            "code" => {
                check_arg_count("char::code", 0, &args, span)?;
                Ok(Value::Integer(c as i64))
            }
            _ => Err(FerriError::Runtime {
                message: format!("no method `{method}` on type char"),
                line: span.line,
                column: span.column,
            }),
        }
    }
}
