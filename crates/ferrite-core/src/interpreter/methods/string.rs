//! String method implementations.
//!
//! Supports: len, is_empty, contains, to_uppercase, to_lowercase, trim,
//! starts_with, ends_with, replace, chars, split, repeat, push_str, clone.

use crate::errors::check_arg_count;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

use super::super::Interpreter;

impl Interpreter {
    /// Handle method calls on String values.
    pub(crate) fn call_string_method(
        &mut self,
        receiver: Value,
        method: &str,
        args: Vec<Value>,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let Value::String(s) = receiver else {
            unreachable!()
        };
        match method {
            "len" => Ok(Value::Integer(s.len() as i64)),
            "is_empty" => Ok(Value::Bool(s.is_empty())),
            "contains" => {
                check_arg_count("String::contains", 1, &args, span)?;
                let needle = match &args[0] {
                    Value::String(s) => s.clone(),
                    Value::Char(c) => c.to_string(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!(
                                "String::contains() expects a string or char, got {}",
                                other.type_name()
                            ),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                Ok(Value::Bool(s.contains(&needle)))
            }
            "to_uppercase" => Ok(Value::String(s.to_uppercase())),
            "to_lowercase" => Ok(Value::String(s.to_lowercase())),
            "trim" => Ok(Value::String(s.trim().to_string())),
            "starts_with" => {
                check_arg_count("String::starts_with", 1, &args, span)?;
                let prefix = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected string, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                Ok(Value::Bool(s.starts_with(&prefix)))
            }
            "ends_with" => {
                check_arg_count("String::ends_with", 1, &args, span)?;
                let suffix = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected string, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                Ok(Value::Bool(s.ends_with(&suffix)))
            }
            "replace" => {
                check_arg_count("String::replace", 2, &args, span)?;
                let from = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected string, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                let to = match &args[1] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected string, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                Ok(Value::String(s.replace(&from, &to)))
            }
            "chars" => {
                let chars: Vec<Value> = s.chars().map(Value::Char).collect();
                Ok(Value::Vec(chars))
            }
            "split" => {
                check_arg_count("String::split", 1, &args, span)?;
                let delim = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected string, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                let parts: Vec<Value> = s
                    .split(&delim)
                    .map(|p| Value::String(p.to_string()))
                    .collect();
                Ok(Value::Vec(parts))
            }
            "repeat" => {
                check_arg_count("String::repeat", 1, &args, span)?;
                let n = match &args[0] {
                    Value::Integer(n) => *n as usize,
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected integer, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                Ok(Value::String(s.repeat(n)))
            }
            "push_str" => {
                // push_str is immutable in Ferrite — returns new string
                check_arg_count("String::push_str", 1, &args, span)?;
                let suffix = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected string, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                let mut new_s = s;
                new_s.push_str(&suffix);
                Ok(Value::String(new_s))
            }
            "clone" => Ok(Value::String(s)),
            "to_string" => Ok(Value::String(s)),
            _ => self.try_to_json_method(Value::String(s), method, span, "String"),
        }
    }
}
