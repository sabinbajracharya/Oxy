use crate::errors::FerriError;
use crate::lexer::Span;
use crate::stdlib::math::value_to_f64;
use crate::types::Value;

/// Dispatch time:: function calls.
pub fn call(func_name: &str, args: &[Value], span: &Span) -> Result<Value, FerriError> {
    match func_name {
        "now" => {
            if !args.is_empty() {
                return Err(FerriError::Runtime {
                    message: "time::now() takes 0 arguments".into(),
                    line: span.line,
                    column: span.column,
                });
            }
            let dur = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap();
            Ok(Value::Float(dur.as_secs_f64()))
        }
        "millis" => {
            if !args.is_empty() {
                return Err(FerriError::Runtime {
                    message: "time::millis() takes 0 arguments".into(),
                    line: span.line,
                    column: span.column,
                });
            }
            let dur = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap();
            Ok(Value::Integer(dur.as_millis() as i64))
        }
        "elapsed" => {
            if args.len() != 1 {
                return Err(FerriError::Runtime {
                    message: "time::elapsed() takes 1 argument".into(),
                    line: span.line,
                    column: span.column,
                });
            }
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
