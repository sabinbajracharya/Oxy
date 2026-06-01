//! Error types for the Oxy language.

use crate::diagnostics::codes;
use crate::diagnostics::{
    span_from_line_column, Diagnostic, DiagnosticCategory, DiagnosticSeverity,
};
use crate::types::Value;

/// Errors surfaced anywhere in the compile/run pipeline (lexer → parser →
/// type checker → runtime), plus the non-error control-flow signals
/// (`Return`/`Break`/`Continue`) that ride the same `Result` channel and are
/// caught at function/loop boundaries.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PipelineError {
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
    /// Type error with source location.
    #[error("[{line}:{column}] type error: {message}")]
    TypeError {
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
    /// Structured diagnostic payload for rich multi-span notes/help/fix-its.
    #[error("[{code}] {message}", code = diagnostic.code, message = diagnostic.message)]
    Diagnostic { diagnostic: Box<Diagnostic> },
    /// Control flow: `return` statement carrying a value.
    /// Not a real error — caught at function call boundaries.
    #[error("return outside of function")]
    Return(Box<Value>),
    /// Control flow: `break` with optional label and value.
    /// Not a real error — caught at loop boundaries.
    #[error("break outside of loop")]
    Break(Option<String>, Option<Box<Value>>),
    /// Control flow: `continue` with optional label.
    /// Not a real error — caught at loop boundaries.
    #[error("continue outside of loop")]
    Continue(Option<String>),
}

impl PipelineError {
    /// Wrap a first-class diagnostic as a pipeline error.
    pub fn from_diagnostic(diagnostic: Diagnostic) -> Self {
        Self::Diagnostic {
            diagnostic: Box::new(diagnostic),
        }
    }

    /// Convert any pipeline error variant to a structured diagnostic.
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            PipelineError::Lexer {
                message,
                line,
                column,
            } => Diagnostic::error(codes::LEX_INVALID_TOKEN, DiagnosticCategory::Lexer, message)
                .with_primary_span(span_from_line_column(*line, *column)),
            PipelineError::Parser {
                message,
                line,
                column,
            } => {
                Diagnostic::error(codes::PAR_UNEXPECTED_TOKEN, DiagnosticCategory::Parser, message)
                    .with_primary_span(span_from_line_column(*line, *column))
            }
            PipelineError::TypeError {
                message,
                line,
                column,
            } => Diagnostic::error(codes::TYP_MISMATCH, DiagnosticCategory::TypeChecker, message)
                .with_primary_span(span_from_line_column(*line, *column)),
            PipelineError::Runtime {
                message,
                line,
                column,
            } => Diagnostic::error(codes::RUN_FAILURE, DiagnosticCategory::Runtime, message)
                .with_primary_span(span_from_line_column(*line, *column)),
            PipelineError::Diagnostic { diagnostic } => (**diagnostic).clone(),
            PipelineError::Return(_) | PipelineError::Break(_, _) | PipelineError::Continue(_) => {
                Diagnostic::new(
                    codes::CTL_FLOW_SIGNAL,
                    DiagnosticSeverity::Note,
                    DiagnosticCategory::Other,
                    self.to_string(),
                )
            }
        }
    }

    /// Return the primary source location when available.
    pub fn line_column(&self) -> Option<(usize, usize)> {
        self.to_diagnostic().line_column()
    }
}

/// Shorthand constructor for `PipelineError::Runtime`.
pub fn runtime_error(message: impl Into<String>, span: &crate::lexer::Span) -> PipelineError {
    PipelineError::Runtime {
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
) -> Result<(), PipelineError> {
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
) -> Result<&'a str, PipelineError> {
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
) -> Result<i64, PipelineError> {
    match val {
        crate::types::Value::I64(n) => Ok(*n),
        _ => Err(runtime_error(
            format!("{context}: expected integer, got {}", val.type_name()),
            span,
        )),
    }
}

// Re-exported from their canonical homes for backward compatibility.
pub use crate::util::{edit_distance, suggest_name};
pub use crate::vm::CallFrame;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Span;

    #[test]
    fn legacy_variants_convert_to_structured_diagnostic() {
        let err = PipelineError::Parser {
            message: "unexpected token".to_string(),
            line: 3,
            column: 5,
        };
        let diag = err.to_diagnostic();
        assert_eq!(diag.code, codes::PAR_UNEXPECTED_TOKEN);
        assert_eq!(diag.line_column(), Some((3, 5)));
    }

    #[test]
    fn structured_variant_roundtrips() {
        let d = Diagnostic::error(codes::TYP_MISMATCH, DiagnosticCategory::TypeChecker, "bad type")
            .with_primary_span(Span::new(10, 11, 2, 8))
            .with_help("adjust the annotation");
        let err = PipelineError::from_diagnostic(d.clone());
        assert_eq!(err.to_diagnostic(), d);
    }
}
