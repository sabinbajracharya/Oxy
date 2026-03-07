//! Error types for the Oxide language.

use crate::types::Value;

/// Errors produced by the Oxide interpreter.
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

/// Compute Levenshtein edit distance between two strings.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();
    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];
    for (i, row) in matrix.iter_mut().enumerate().take(a_len + 1) {
        row[0] = i;
    }
    for (j, val) in matrix[0].iter_mut().enumerate().take(b_len + 1) {
        *val = j;
    }
    for (i, ca) in a.chars().enumerate() {
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                .min(matrix[i + 1][j] + 1)
                .min(matrix[i][j] + cost);
        }
    }
    matrix[a_len][b_len]
}

/// Find the closest match to `name` from `candidates` using edit distance.
/// Returns `Some(candidate)` if a reasonably close match is found (distance ≤ 3
/// and less than half the name length).
pub fn suggest_name<'a>(
    name: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let max_dist = (name.len() / 2).clamp(1, 3);
    candidates
        .into_iter()
        .filter(|c| *c != name)
        .map(|c| (c, edit_distance(name, c)))
        .filter(|(_, d)| *d <= max_dist)
        .min_by_key(|(_, d)| *d)
        .map(|(c, _)| c.to_string())
}

/// A single frame in the interpreter's call stack.
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// Function name (or "<main>" for top-level).
    pub name: String,
    /// Source line where the call was made.
    pub line: usize,
    /// Source column where the call was made.
    pub column: usize,
}

impl std::fmt::Display for CallFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "  in `{}` at line {}:{}",
            self.name, self.line, self.column
        )
    }
}
