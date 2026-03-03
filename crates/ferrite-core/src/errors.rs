//! Error types for the Ferrite language.

use crate::types::Value;

/// Errors produced by the Ferrite interpreter.
#[derive(Debug, Clone, thiserror::Error)]
pub enum FerriError {
    /// Lexer error with source location.
    #[error("[{line}:{column}] {message}")]
    Lexer {
        message: String,
        line: usize,
        column: usize,
    },
    /// Parser error with source location.
    #[error("[{line}:{column}] {message}")]
    Parser {
        message: String,
        line: usize,
        column: usize,
    },
    /// Runtime error with source location.
    #[error("[{line}:{column}] runtime error: {message}")]
    Runtime {
        message: String,
        line: usize,
        column: usize,
    },
    /// Control flow: `return` statement carrying a value.
    /// Not a real error — caught at function call boundaries.
    #[error("return outside of function")]
    Return(Box<Value>),
    /// Control flow: `break` with optional value.
    /// Not a real error — caught at loop boundaries.
    #[error("break outside of loop")]
    Break(Option<Box<Value>>),
    /// Control flow: `continue`.
    /// Not a real error — caught at loop boundaries.
    #[error("continue outside of loop")]
    Continue,
}

/// Shorthand constructor for `FerriError::Runtime`.
pub fn runtime_error(message: impl Into<String>, span: &crate::lexer::Span) -> FerriError {
    FerriError::Runtime {
        message: message.into(),
        line: span.line,
        column: span.column,
    }
}

/// Validates that a function/method received the expected number of arguments.
pub fn check_arg_count(
    name: &str,
    expected: usize,
    args: &[crate::types::Value],
    span: &crate::lexer::Span,
) -> Result<(), FerriError> {
    if args.len() != expected {
        return Err(runtime_error(
            format!("{name}() takes {expected} argument(s), got {}", args.len()),
            span,
        ));
    }
    Ok(())
}

/// Extracts a `&str` from a `Value::String`, or returns a typed runtime error.
pub fn expect_string<'a>(
    val: &'a crate::types::Value,
    context: &str,
    span: &crate::lexer::Span,
) -> Result<&'a str, FerriError> {
    match val {
        crate::types::Value::String(s) => Ok(s.as_str()),
        _ => Err(runtime_error(
            format!("{context}: expected string, got {}", val.type_name()),
            span,
        )),
    }
}

/// Extracts an `i64` from a `Value::Integer`, or returns a typed runtime error.
pub fn expect_integer(
    val: &crate::types::Value,
    context: &str,
    span: &crate::lexer::Span,
) -> Result<i64, FerriError> {
    match val {
        crate::types::Value::Integer(n) => Ok(*n),
        _ => Err(runtime_error(
            format!("{context}: expected integer, got {}", val.type_name()),
            span,
        )),
    }
}
