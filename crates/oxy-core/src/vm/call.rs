// vm/call.rs — Stdlib dispatch helper.
//
// Extracted from vm/mod.rs to keep that file focused on the Vm struct and its
// execution loop.

use crate::types::Value;

/// Call a stdlib module function, converting FerriError to String.
pub(super) fn call_stdlib(
    f: fn(&str, &[Value], &crate::lexer::Span) -> Result<Value, crate::errors::FerriError>,
    func: &str,
    args: &[Value],
) -> Result<Value, String> {
    let span = crate::lexer::Span {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    };
    f(func, args, &span).map_err(|e| format!("{e}"))
}
