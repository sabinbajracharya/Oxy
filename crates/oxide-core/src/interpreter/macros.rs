//! Macro implementation for `println!`, `print!`, `vec!`, `format!`, `dbg!`, etc.
//!
//! Oxide macros mirror Rust's standard macros. They are invoked with `!`
//! syntax (e.g. `println!("hello {}", name)`) and handled specially by the
//! interpreter rather than going through normal function dispatch.

use crate::ast::Expr;
use crate::env::Env;
use crate::errors::FerriError;
use crate::types::Value;

use super::format::debug_format;
use super::Interpreter;

impl Interpreter {
    /// Evaluate a macro call (e.g. `println!("x = {}", x)`).
    pub(crate) fn eval_macro_call(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &Env,
        line: usize,
        col: usize,
    ) -> Result<Value, FerriError> {
        match name {
            "println" => {
                let output = self.format_macro_args(args, env, line, col)?;
                self.write_output(&output);
                self.write_output("\n");
                Ok(Value::Unit)
            }
            "print" => {
                let output = self.format_macro_args(args, env, line, col)?;
                self.write_output(&output);
                Ok(Value::Unit)
            }
            "vec" => {
                let vals: Vec<Value> = args
                    .iter()
                    .map(|a| self.eval_expr(a, env))
                    .collect::<Result<_, _>>()?;
                Ok(Value::Vec(vals))
            }
            "format" => {
                let output = self.format_macro_args(args, env, line, col)?;
                Ok(Value::String(output))
            }
            "eprintln" => {
                let output = self.format_macro_args(args, env, line, col)?;
                eprintln!("{output}");
                Ok(Value::Unit)
            }
            "panic" => {
                let output = if args.is_empty() {
                    "explicit panic".to_string()
                } else {
                    self.format_macro_args(args, env, line, col)?
                };
                Err(FerriError::Runtime {
                    message: format!("panic: {output}"),
                    line,
                    column: col,
                })
            }
            "todo" => Err(FerriError::Runtime {
                message: "not yet implemented".to_string(),
                line,
                column: col,
            }),
            "unimplemented" => Err(FerriError::Runtime {
                message: "not implemented".to_string(),
                line,
                column: col,
            }),
            "dbg" => {
                // dbg! prints debug output and returns the value
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("dbg!() takes 1 argument, got {}", args.len()),
                        line,
                        column: col,
                    });
                }
                let val = self.eval_expr(&args[0], env)?;
                let debug = debug_format(&val);
                self.write_output(&format!("[dbg] {debug}\n"));
                Ok(val)
            }
            "assert" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(FerriError::Runtime {
                        message: format!("assert!() takes 1-2 arguments, got {}", args.len()),
                        line,
                        column: col,
                    });
                }
                let val = self.eval_expr(&args[0], env)?;
                if !val.is_truthy() {
                    let msg = if args.len() == 2 {
                        let m = self.eval_expr(&args[1], env)?;
                        format!("assertion failed: {m}")
                    } else {
                        "assertion failed".to_string()
                    };
                    return Err(FerriError::Runtime {
                        message: msg,
                        line,
                        column: col,
                    });
                }
                Ok(Value::Unit)
            }
            "assert_eq" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(FerriError::Runtime {
                        message: format!("assert_eq!() takes 2-3 arguments, got {}", args.len()),
                        line,
                        column: col,
                    });
                }
                let left = self.eval_expr(&args[0], env)?;
                let right = self.eval_expr(&args[1], env)?;
                if left != right {
                    let msg = if args.len() == 3 {
                        let m = self.eval_expr(&args[2], env)?;
                        format!("assertion failed: `left == right`\n  left: `{}`\n right: `{}`\n    {m}", debug_format(&left), debug_format(&right))
                    } else {
                        format!(
                            "assertion failed: `left == right`\n  left: `{}`\n right: `{}`",
                            debug_format(&left),
                            debug_format(&right)
                        )
                    };
                    return Err(FerriError::Runtime {
                        message: msg,
                        line,
                        column: col,
                    });
                }
                Ok(Value::Unit)
            }
            "assert_ne" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(FerriError::Runtime {
                        message: format!("assert_ne!() takes 2-3 arguments, got {}", args.len()),
                        line,
                        column: col,
                    });
                }
                let left = self.eval_expr(&args[0], env)?;
                let right = self.eval_expr(&args[1], env)?;
                if left == right {
                    let msg = if args.len() == 3 {
                        let m = self.eval_expr(&args[2], env)?;
                        format!("assertion failed: `left != right`\n  left: `{}`\n right: `{}`\n    {m}", debug_format(&left), debug_format(&right))
                    } else {
                        format!(
                            "assertion failed: `left != right`\n  left: `{}`\n right: `{}`",
                            debug_format(&left),
                            debug_format(&right)
                        )
                    };
                    return Err(FerriError::Runtime {
                        message: msg,
                        line,
                        column: col,
                    });
                }
                Ok(Value::Unit)
            }
            _ => Err(FerriError::Runtime {
                message: format!("unknown macro '{name}!'"),
                line,
                column: col,
            }),
        }
    }

    /// Format arguments using a format string (like Rust's `format!`).
    ///
    /// Handles `{}` (display), `{:?}` (debug), `{{` (escaped `{`),
    /// and `}}` (escaped `}`).
    pub(crate) fn format_macro_args(
        &mut self,
        args: &[Expr],
        env: &Env,
        line: usize,
        col: usize,
    ) -> Result<String, FerriError> {
        if args.is_empty() {
            return Ok(String::new());
        }

        // First argument should be a format string
        let fmt_val = self.eval_expr(&args[0], env)?;
        let Value::String(fmt_str) = fmt_val else {
            // If not a string, just print the value
            return Ok(self.format_value(&fmt_val));
        };

        let mut result = String::new();
        let mut arg_idx = 1;
        let mut chars = fmt_str.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                if chars.peek() == Some(&'{') {
                    // Escaped `{{` → literal `{`
                    chars.next();
                    result.push('{');
                } else if chars.peek() == Some(&'}') {
                    // `{}` placeholder
                    chars.next();
                    if arg_idx >= args.len() {
                        return Err(FerriError::Runtime {
                            message: "not enough arguments for format string".into(),
                            line,
                            column: col,
                        });
                    }
                    let val = self.eval_expr(&args[arg_idx], env)?;
                    result.push_str(&self.format_value(&val));
                    arg_idx += 1;
                } else if chars.peek() == Some(&':') {
                    // `{:?}` debug format — consume until `}`
                    for c in chars.by_ref() {
                        if c == '}' {
                            break;
                        }
                    }
                    if arg_idx >= args.len() {
                        return Err(FerriError::Runtime {
                            message: "not enough arguments for format string".into(),
                            line,
                            column: col,
                        });
                    }
                    let val = self.eval_expr(&args[arg_idx], env)?;
                    result.push_str(&debug_format(&val));
                    arg_idx += 1;
                } else {
                    result.push(ch);
                }
            } else if ch == '}' && chars.peek() == Some(&'}') {
                // Escaped `}}` → literal `}`
                chars.next();
                result.push('}');
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    }

    /// Format a value, using its Display trait impl if available.
    pub(crate) fn format_value(&mut self, val: &Value) -> String {
        let type_name = match val {
            Value::Struct { name, .. } => Some(name.clone()),
            Value::EnumVariant { enum_name, .. } => Some(enum_name.clone()),
            _ => None,
        };
        if let Some(tn) = type_name {
            let key = (tn.clone(), "Display".to_string());
            if self.trait_impls.contains_key(&key) {
                if let Some(method_def) = self.find_trait_method(&tn, "fmt") {
                    use crate::env::Environment;
                    let func_env = Environment::child(&self.env);
                    func_env
                        .borrow_mut()
                        .define("self".to_string(), val.clone(), true);
                    let prev = self.current_self_type.take();
                    self.current_self_type = Some(tn);
                    let result = self.eval_block(&method_def.body, &func_env);
                    self.current_self_type = prev;
                    match result {
                        Ok(Value::String(s)) => return s,
                        Err(FerriError::Return(v)) => {
                            if let Value::String(s) = *v {
                                return s;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        format!("{val}")
    }
}
