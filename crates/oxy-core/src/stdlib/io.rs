//! Standard input module.
//!
//! Reads from stdin for CLI scripts (Advent of Code, pipelines).
//! All operations return `Result<_, String>`.

use std::io::{BufRead, Read};

use crate::errors::{check_arg_count, runtime_error, PipelineError};
use crate::lexer::Span;
use crate::types::Value;

pub fn call(
    func_name: &str,
    args: &[Value],
    span: &Span,
    _cb: crate::stdlib::registry::ClosureInvoker<'_>,
) -> Result<Value, PipelineError> {
    match func_name {
        "read_to_string" => {
            check_arg_count("std::io::read_to_string", 0, args, span)?;
            let mut buf = String::new();
            match std::io::stdin().lock().read_to_string(&mut buf) {
                Ok(_) => Ok(Value::ok(Value::String(buf))),
                Err(e) => Ok(Value::err(Value::String(e.to_string()))),
            }
        }
        "read_line" => {
            check_arg_count("std::io::read_line", 0, args, span)?;
            let mut buf = String::new();
            match std::io::stdin().lock().read_line(&mut buf) {
                Ok(0) => Ok(Value::ok(Value::none())),
                Ok(_) => {
                    if buf.ends_with('\n') {
                        buf.pop();
                        if buf.ends_with('\r') {
                            buf.pop();
                        }
                    }
                    Ok(Value::ok(Value::some(Value::String(buf))))
                }
                Err(e) => Ok(Value::err(Value::String(e.to_string()))),
            }
        }
        other => Err(runtime_error(
            format!("no function 'std::io::{other}'"),
            span,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span() -> Span {
        Span {
            start: 0,
            end: 0,
            line: 1,
            column: 1,
        }
    }

    fn no_cb() -> impl FnMut(&Value, &[Value]) -> Result<Value, String> {
        |_, _| unreachable!("io module does not invoke closures")
    }

    #[test]
    fn test_read_to_string_rejects_args() {
        let mut cb = no_cb();
        let r = call(
            "read_to_string",
            &[Value::String("x".into())],
            &span(),
            &mut cb,
        );
        assert!(r.is_err());
    }

    #[test]
    fn test_read_line_rejects_args() {
        let mut cb = no_cb();
        let r = call("read_line", &[Value::I64(1)], &span(), &mut cb);
        assert!(r.is_err());
    }

    #[test]
    fn test_unknown_function() {
        let mut cb = no_cb();
        let r = call("nope", &[], &span(), &mut cb);
        assert!(r.is_err());
    }
}
