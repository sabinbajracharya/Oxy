//! JSON serialization/deserialization functions (`json::` pseudo-module).
//!
//! Provides `json::serialize`, `json::parse`, `json::from_struct`, etc.
//! These are dispatched from path calls like `json::serialize(value)`.

use std::collections::HashMap;

use crate::ast::StructKind;
use crate::errors::check_arg_count;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

use super::Interpreter;

impl Interpreter {
    /// Dispatch a `json::function()` call.
    pub(crate) fn call_json_function(
        &self,
        func_name: &str,
        args: &[Value],
        span: &Span,
    ) -> Result<Value, FerriError> {
        match func_name {
            "serialize" | "to_string" => {
                check_arg_count(&format!("json::{func_name}"), 1, args, span)?;
                match crate::json::serialize(&args[0]) {
                    Ok(json) => Ok(Value::ok(Value::String(json))),
                    Err(e) => Ok(Value::err(Value::String(e))),
                }
            }
            "to_string_pretty" => {
                check_arg_count("json::to_string_pretty", 1, args, span)?;
                match crate::json::serialize_pretty(&args[0]) {
                    Ok(json) => Ok(Value::ok(Value::String(json))),
                    Err(e) => Ok(Value::err(Value::String(e))),
                }
            }
            "deserialize" | "parse" | "from_str" => {
                check_arg_count(&format!("json::{func_name}"), 1, args, span)?;
                let s = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!(
                                "json::{func_name}() expects a string, got {}",
                                other.type_name()
                            ),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                match crate::json::deserialize(&s) {
                    Ok(val) => Ok(Value::ok(val)),
                    Err(e) => Ok(Value::err(Value::String(e))),
                }
            }
            "from_struct" => {
                check_arg_count("json::from_struct", 2, args, span)?;
                let json_str = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!(
                                "json::from_struct() first argument must be a string, got {}",
                                other.type_name()
                            ),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                let struct_name = match &args[1] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!(
                                "json::from_struct() second argument must be a string, got {}",
                                other.type_name()
                            ),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                self.json_from_struct(&json_str, &struct_name, span)
            }
            _ => Err(FerriError::Runtime {
                message: format!("unknown json function: {func_name}"),
                line: span.line,
                column: span.column,
            }),
        }
    }

    /// Deserialize a JSON string into a named Oxide struct.
    fn json_from_struct(
        &self,
        json_str: &str,
        struct_name: &str,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let parsed = match crate::json::deserialize(json_str) {
            Ok(val) => val,
            Err(e) => return Ok(Value::err(Value::String(e))),
        };

        let map = match parsed {
            Value::HashMap(m) => m,
            _ => {
                return Ok(Value::err(Value::String(
                    "JSON value is not an object".to_string(),
                )))
            }
        };

        let sdef = match self.struct_defs.get(struct_name) {
            Some(sd) => sd.clone(),
            None => {
                return Err(FerriError::Runtime {
                    message: format!("unknown struct type: {struct_name}"),
                    line: span.line,
                    column: span.column,
                })
            }
        };

        match &sdef.kind {
            StructKind::Named(fields) => {
                let mut result_fields = HashMap::new();
                for field in fields {
                    if let Some(val) = map.get(&field.name) {
                        result_fields.insert(field.name.clone(), val.clone());
                    } else {
                        result_fields.insert(field.name.clone(), Value::Unit);
                    }
                }
                Ok(Value::ok(Value::Struct {
                    name: struct_name.to_string(),
                    fields: result_fields,
                }))
            }
            _ => Ok(Value::err(Value::String(format!(
                "json::from_struct only supports named-field structs, not {struct_name}"
            )))),
        }
    }
}
