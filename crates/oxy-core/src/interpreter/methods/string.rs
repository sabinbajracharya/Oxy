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
                // push_str is immutable in Oxy — returns new string
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
            "char_at" => {
                check_arg_count("String::char_at", 1, &args, span)?;
                let i = match &args[0] {
                    Value::Integer(n) => *n as usize,
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected integer, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                s.chars()
                    .nth(i)
                    .map(Value::Char)
                    .ok_or_else(|| FerriError::Runtime {
                        message: format!(
                            "char_at: index {i} out of bounds (len {})",
                            s.chars().count()
                        ),
                        line: span.line,
                        column: span.column,
                    })
            }
            "substring" => {
                check_arg_count("String::substring", 2, &args, span)?;
                let start = match &args[0] {
                    Value::Integer(n) => *n as usize,
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected integer, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                let end = match &args[1] {
                    Value::Integer(n) => *n as usize,
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected integer, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                let chars: Vec<char> = s.chars().collect();
                if start > end || end > chars.len() {
                    return Err(FerriError::Runtime {
                        message: format!(
                            "substring: invalid range {start}..{end} (len {})",
                            chars.len()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
                Ok(Value::String(chars[start..end].iter().collect()))
            }
            "parse_int" => {
                check_arg_count("String::parse_int", 0, &args, span)?;
                let trimmed = s.trim();
                let result = if let Some(hex) = trimmed
                    .strip_prefix("0x")
                    .or_else(|| trimmed.strip_prefix("0X"))
                {
                    i64::from_str_radix(hex, 16)
                } else if let Some(bin) = trimmed
                    .strip_prefix("0b")
                    .or_else(|| trimmed.strip_prefix("0B"))
                {
                    i64::from_str_radix(bin, 2)
                } else if let Some(oct) = trimmed
                    .strip_prefix("0o")
                    .or_else(|| trimmed.strip_prefix("0O"))
                {
                    i64::from_str_radix(oct, 8)
                } else {
                    trimmed.parse::<i64>()
                };
                match result {
                    Ok(n) => Ok(Value::ok(Value::Integer(n))),
                    Err(_) => Ok(Value::err(Value::String(format!(
                        "cannot parse \"{}\" as integer",
                        s
                    )))),
                }
            }
            "parse_float" => {
                check_arg_count("String::parse_float", 0, &args, span)?;
                match s.trim().parse::<f64>() {
                    Ok(n) => Ok(Value::ok(Value::Float(n))),
                    Err(_) => Ok(Value::err(Value::String(format!(
                        "cannot parse \"{}\" as float",
                        s
                    )))),
                }
            }
            "clone" => Ok(Value::String(s)),
            "to_string" => Ok(Value::String(s)),
            _ => self.try_to_json_method(Value::String(s), method, span, "String"),
        }
    }
}
