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
