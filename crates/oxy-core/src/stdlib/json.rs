//! `json::*` dispatcher: routes `json::parse`, `json::serialize`,
//! `json::to_string`, `json::to_string_pretty`, `json::deserialize`,
//! `json::from_str`, `json::from_struct` to the JSON codec in `crate::json`.
//!
//! Registered in `stdlib::registry::MODULES` so that
//! `compiler::helpers::is_builtin_path` accepts any `json::*` path and
//! the VM dispatches through this `call` function.

use crate::errors::PipelineError;
use crate::lexer::Span;
use crate::types::Value;

pub fn call(
    func_name: &str,
    args: &[Value],
    _span: &Span,
    _cb: crate::stdlib::registry::ClosureInvoker<'_>,
) -> Result<Value, PipelineError> {
    let map_err = |e: String| Value::err(Value::String(e));
    let result: Value = match func_name {
        "parse" => match crate::json::deserialize(&format_first(args)) {
            Ok(val) => Value::ok(val),
            Err(e) => map_err(format!("json::parse: {e}")),
        },
        "serialize" | "to_string" => {
            match crate::json::serialize(args.first().unwrap_or(&Value::Unit)) {
                Ok(s) => Value::ok(Value::String(s)),
                Err(e) => map_err(e),
            }
        }
        "to_string_pretty" => {
            match crate::json::serialize_pretty(args.first().unwrap_or(&Value::Unit)) {
                Ok(s) => Value::ok(Value::String(s)),
                Err(e) => map_err(e),
            }
        }
        "deserialize" | "from_str" => match crate::json::deserialize(&format_first(args)) {
            Ok(val) => Value::ok(val),
            Err(e) => map_err(format!("json error: {e}")),
        },
        "from_struct" => {
            let s = format_first(args);
            let type_name = args.get(1).map(|v| v.to_string()).unwrap_or_default();
            match crate::json::deserialize(&s) {
                Ok(val) => {
                    if !type_name.is_empty() {
                        if let Value::Struct { fields, .. } = &val {
                            Value::ok(Value::Struct {
                                name: type_name,
                                fields: fields.clone(),
                            })
                        } else {
                            Value::ok(val)
                        }
                    } else {
                        Value::ok(val)
                    }
                }
                Err(e) => map_err(format!("json error: {e}")),
            }
        }
        other => {
            return Err(PipelineError::Runtime {
                message: format!("unknown json function `json::{other}`"),
                line: 0,
                column: 0,
            });
        }
    };
    Ok(result)
}

fn format_first(args: &[Value]) -> String {
    args.first().map(|v| format!("{v}")).unwrap_or_default()
}
