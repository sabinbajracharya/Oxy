// vm/call.rs — Stdlib dispatch helper.
//
// Extracted from vm/mod.rs to keep that file focused on the Vm struct and its
// execution loop.

use crate::stdlib::registry::{ClosureInvoker, ModuleCall};
use crate::types::Value;

/// Call a stdlib module function, converting FerriError to String. The
/// caller supplies a `ClosureInvoker` so modules that need to run user
/// closures (e.g. server route handlers) can drive them back through the VM.
pub(super) fn call_stdlib(
    f: ModuleCall,
    func: &str,
    args: &[Value],
    cb: ClosureInvoker<'_>,
) -> Result<Value, String> {
    let span = crate::lexer::Span {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    };
    f(func, args, &span, cb).map_err(|e| format!("{e}"))
}
