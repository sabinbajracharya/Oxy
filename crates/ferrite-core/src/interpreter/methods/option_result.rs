//! Option and Result method implementations.
//!
//! Option: is_some, is_none, unwrap, expect, unwrap_or, unwrap_or_else, map, and_then.
//! Result: is_ok, is_err, unwrap, expect, unwrap_err, unwrap_or, unwrap_or_else,
//!         map, map_err, and_then, ok, err.

use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::{
    Value, ERR_VARIANT, NONE_VARIANT, OK_VARIANT, OPTION_TYPE, RESULT_TYPE, SOME_VARIANT,
};

use super::super::Interpreter;

impl Interpreter {
    /// Handle built-in Option/Result methods.
    ///
    /// Returns `Ok(Some(value))` if the method was handled,
    /// `Ok(None)` if the method name wasn't recognized.
    pub(crate) fn call_option_result_method(
        &mut self,
        receiver: &Value,
        method: &str,
        args: &[Value],
        span: &Span,
    ) -> Result<Option<Value>, FerriError> {
        let Value::EnumVariant {
            enum_name,
            variant,
            data,
            ..
        } = receiver
        else {
            return Ok(None);
        };

        match (enum_name.as_str(), method) {
            // === Option methods ===
            (OPTION_TYPE, "is_some") => Ok(Some(Value::Bool(variant == SOME_VARIANT))),
            (OPTION_TYPE, "is_none") => Ok(Some(Value::Bool(variant == NONE_VARIANT))),
            (OPTION_TYPE, "unwrap") => {
                if variant == SOME_VARIANT {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else {
                    Err(FerriError::Runtime {
                        message: "called `Option::unwrap()` on a `None` value".to_string(),
                        line: span.line,
                        column: span.column,
                    })
                }
            }
            (OPTION_TYPE, "expect") => {
                if variant == SOME_VARIANT {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else {
                    let msg = args
                        .first()
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "Option::expect failed".to_string());
                    Err(FerriError::Runtime {
                        message: msg,
                        line: span.line,
                        column: span.column,
                    })
                }
            }
            (OPTION_TYPE, "unwrap_or") => {
                if variant == SOME_VARIANT {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else {
                    Ok(Some(args.first().cloned().unwrap_or(Value::Unit)))
                }
            }
            (OPTION_TYPE, "unwrap_or_else") => {
                if variant == SOME_VARIANT {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else if let Some(Value::Function(_)) = args.first() {
                    let result = self.call_function(&args[0], &[], span.line, span.column)?;
                    Ok(Some(result))
                } else {
                    Ok(Some(Value::Unit))
                }
            }
            (OPTION_TYPE, "map") => {
                if variant == SOME_VARIANT {
                    if let Some(func) = args.first() {
                        let inner = data.first().cloned().unwrap_or(Value::Unit);
                        let result = self.call_function(func, &[inner], span.line, span.column)?;
                        Ok(Some(Value::some(result)))
                    } else {
                        Ok(Some(receiver.clone()))
                    }
                } else {
                    // None.map(f) → None
                    Ok(Some(receiver.clone()))
                }
            }
            (OPTION_TYPE, "and_then") => {
                if variant == SOME_VARIANT {
                    if let Some(func) = args.first() {
                        let inner = data.first().cloned().unwrap_or(Value::Unit);
                        let result = self.call_function(func, &[inner], span.line, span.column)?;
                        Ok(Some(result))
                    } else {
                        Ok(Some(receiver.clone()))
                    }
                } else {
                    Ok(Some(receiver.clone()))
                }
            }

            // === Result methods ===
            (RESULT_TYPE, "is_ok") => Ok(Some(Value::Bool(variant == OK_VARIANT))),
            (RESULT_TYPE, "is_err") => Ok(Some(Value::Bool(variant == ERR_VARIANT))),
            (RESULT_TYPE, "unwrap") => {
                if variant == OK_VARIANT {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else {
                    let err_val = data
                        .first()
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "unknown error".to_string());
                    Err(FerriError::Runtime {
                        message: format!("called `Result::unwrap()` on an `Err` value: {err_val}"),
                        line: span.line,
                        column: span.column,
                    })
                }
            }
            (RESULT_TYPE, "expect") => {
                if variant == OK_VARIANT {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else {
                    let msg = args
                        .first()
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "Result::expect failed".to_string());
                    Err(FerriError::Runtime {
                        message: msg,
                        line: span.line,
                        column: span.column,
                    })
                }
            }
            (RESULT_TYPE, "unwrap_err") => {
                if variant == ERR_VARIANT {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else {
                    Err(FerriError::Runtime {
                        message: "called `Result::unwrap_err()` on an `Ok` value".to_string(),
                        line: span.line,
                        column: span.column,
                    })
                }
            }
            (RESULT_TYPE, "unwrap_or") => {
                if variant == OK_VARIANT {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else {
                    Ok(Some(args.first().cloned().unwrap_or(Value::Unit)))
                }
            }
            (RESULT_TYPE, "unwrap_or_else") => {
                if variant == OK_VARIANT {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else if let Some(Value::Function(_)) = args.first() {
                    let err_val = data.first().cloned().unwrap_or(Value::Unit);
                    let result =
                        self.call_function(&args[0], &[err_val], span.line, span.column)?;
                    Ok(Some(result))
                } else {
                    Ok(Some(Value::Unit))
                }
            }
            (RESULT_TYPE, "map") => {
                if variant == OK_VARIANT {
                    if let Some(func) = args.first() {
                        let inner = data.first().cloned().unwrap_or(Value::Unit);
                        let result = self.call_function(func, &[inner], span.line, span.column)?;
                        Ok(Some(Value::ok(result)))
                    } else {
                        Ok(Some(receiver.clone()))
                    }
                } else {
                    // Err(e).map(f) → Err(e)
                    Ok(Some(receiver.clone()))
                }
            }
            (RESULT_TYPE, "map_err") => {
                if variant == ERR_VARIANT {
                    if let Some(func) = args.first() {
                        let inner = data.first().cloned().unwrap_or(Value::Unit);
                        let result = self.call_function(func, &[inner], span.line, span.column)?;
                        Ok(Some(Value::err(result)))
                    } else {
                        Ok(Some(receiver.clone()))
                    }
                } else {
                    Ok(Some(receiver.clone()))
                }
            }
            (RESULT_TYPE, "and_then") => {
                if variant == OK_VARIANT {
                    if let Some(func) = args.first() {
                        let inner = data.first().cloned().unwrap_or(Value::Unit);
                        let result = self.call_function(func, &[inner], span.line, span.column)?;
                        Ok(Some(result))
                    } else {
                        Ok(Some(receiver.clone()))
                    }
                } else {
                    Ok(Some(receiver.clone()))
                }
            }
            (RESULT_TYPE, "ok") => {
                if variant == OK_VARIANT {
                    Ok(Some(Value::some(data[0].clone())))
                } else {
                    Ok(Some(Value::none()))
                }
            }
            (RESULT_TYPE, "err") => {
                if variant == ERR_VARIANT {
                    Ok(Some(Value::some(data[0].clone())))
                } else {
                    Ok(Some(Value::none()))
                }
            }

            _ => Ok(None),
        }
    }
}
