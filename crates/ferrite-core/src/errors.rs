//! Error types for the Ferrite language.

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
}
