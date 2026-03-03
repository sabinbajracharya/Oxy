//! Process standard library module.
//!
//! Provides process control and command execution mirroring `std::process`.

use crate::errors::{check_arg_count, expect_integer, expect_string, runtime_error, FerriError};
use crate::lexer::Span;
use crate::types::Value;

/// Dispatch `std::process::` function calls.
pub fn call(func_name: &str, args: &[Value], span: &Span) -> Result<Value, FerriError> {
    match func_name {
        "exit" => {
            check_arg_count("std::process::exit", 1, args, span)?;
            let code = expect_integer(&args[0], "std::process::exit", span)?;
            std::process::exit(code as i32);
        }
        "command" => {
            check_arg_count("std::process::command", 1, args, span)?;
            let program = expect_string(&args[0], "std::process::command", span)?;
            run_command(program, &[], span)
        }
        "command_with_args" => {
            check_arg_count("std::process::command_with_args", 2, args, span)?;
            let program = expect_string(&args[0], "std::process::command_with_args", span)?;
            let cmd_args = match &args[1] {
                Value::Vec(v) => v.iter().map(|a| format!("{a}")).collect::<Vec<_>>(),
                _ => {
                    return Err(runtime_error(
                        "std::process::command_with_args(): second argument must be a Vec",
                        span,
                    ))
                }
            };
            let str_args: Vec<&str> = cmd_args.iter().map(|s| s.as_str()).collect();
            run_command(program, &str_args, span)
        }
        _ => Err(runtime_error(
            format!("unknown process function `std::process::{func_name}`"),
            span,
        )),
    }
}

/// Execute a command and return a struct with stdout, stderr, and status code.
fn run_command(program: &str, args: &[&str], _span: &Span) -> Result<Value, FerriError> {
    match std::process::Command::new(program).args(args).output() {
        Ok(output) => {
            let mut fields = std::collections::HashMap::new();
            fields.insert(
                "stdout".to_string(),
                Value::String(String::from_utf8_lossy(&output.stdout).into_owned()),
            );
            fields.insert(
                "stderr".to_string(),
                Value::String(String::from_utf8_lossy(&output.stderr).into_owned()),
            );
            fields.insert(
                "status".to_string(),
                Value::Integer(output.status.code().unwrap_or(-1) as i64),
            );
            fields.insert("success".to_string(), Value::Bool(output.status.success()));
            Ok(Value::ok(Value::Struct {
                name: "CommandOutput".to_string(),
                fields,
            }))
        }
        Err(e) => Ok(Value::err(Value::String(e.to_string()))),
    }
}

#[cfg(test)]
mod tests {
    use crate::interpreter::run_capturing;

    fn run(src: &str) -> String {
        let (_, output) = run_capturing(src).expect("runtime error");
        output.join("")
    }

    #[test]
    fn test_process_command_echo() {
        let out = run(r#"
fn main() {
    let result = std::process::command("echo");
    if let Ok(output) = result {
        println!("{}", output.success);
    } else {
        println!("err");
    }
}
"#);
        assert_eq!(out, "true\n");
    }

    #[test]
    fn test_process_command_with_args() {
        let out = run(r#"
fn main() {
    let result = std::process::command_with_args("echo", vec!["hello", "world"]);
    if let Ok(output) = result {
        let trimmed = output.stdout.trim();
        println!("{}", trimmed);
    }
}
"#);
        assert_eq!(out, "hello world\n");
    }

    #[test]
    fn test_process_command_nonexistent() {
        let out = run(r#"
fn main() {
    let result = std::process::command("nonexistent_program_xyz_12345");
    if let Ok(output) = result {
        println!("ok");
    } else {
        println!("err");
    }
}
"#);
        assert_eq!(out, "err\n");
    }

    #[test]
    fn test_process_command_status_code() {
        let out = run(r#"
fn main() {
    let result = std::process::command("true");
    if let Ok(output) = result {
        println!("{}", output.status);
    }
}
"#);
        assert_eq!(out, "0\n");
    }

    #[test]
    fn test_process_command_failure_status() {
        let out = run(r#"
fn main() {
    let result = std::process::command("false");
    if let Ok(output) = result {
        println!("{}", output.success);
    }
}
"#);
        assert_eq!(out, "false\n");
    }
}
