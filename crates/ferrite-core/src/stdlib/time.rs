//! Time standard library module.
//!
//! Provides access to wall-clock time and elapsed-time measurement.

use crate::errors::check_arg_count;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::stdlib::math::value_to_f64;
use crate::types::Value;

/// Dispatch time:: function calls.
pub fn call(func_name: &str, args: &[Value], span: &Span) -> Result<Value, FerriError> {
    match func_name {
        "now" => {
            check_arg_count("time::now", 0, args, span)?;
            let dur = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap();
            Ok(Value::Float(dur.as_secs_f64()))
        }
        "millis" => {
            check_arg_count("time::millis", 0, args, span)?;
            let dur = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap();
            Ok(Value::Integer(dur.as_millis() as i64))
        }
        "elapsed" => {
            check_arg_count("time::elapsed", 1, args, span)?;
            let start = value_to_f64(&args[0], span)?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();
            Ok(Value::Float(now - start))
        }
        _ => Err(FerriError::Runtime {
            message: format!("unknown time function `time::{func_name}`"),
            line: span.line,
            column: span.column,
        }),
    }
}
