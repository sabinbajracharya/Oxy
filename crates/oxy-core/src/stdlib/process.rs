//! Process standard library module.
//!
//! Provides process control and command execution mirroring `std::process`.

use crate::errors::{check_arg_count, expect_integer, expect_string, runtime_error, PipelineError};
use crate::lexer::Span;
use crate::types::Value;

/// Dispatch `std::process::` function calls.
pub fn call(
    func_name: &str,
    args: &[Value],
    span: &Span,
    cb: crate::stdlib::registry::ClosureInvoker<'_>,
) -> Result<Value, PipelineError> {
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
                Value::Vec(rc) => rc
                    .borrow()
                    .iter()
                    .map(|a| format!("{a}"))
                    .collect::<Vec<_>>(),
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
        "spawn" => {
            check_arg_count("std::process::spawn", 3, args, span)?;
            let program = expect_string(&args[0], "std::process::spawn", span)?.to_string();
            let cmd_args: Vec<String> = match &args[1] {
                Value::Vec(rc) => rc.borrow().iter().map(|a| format!("{a}")).collect(),
                _ => {
                    return Err(runtime_error(
                        "std::process::spawn(): second argument must be a Vec",
                        span,
                    ))
                }
            };
            run_spawn(&program, &cmd_args, &args[2], cb)
        }
        _ => Err(runtime_error(
            format!("unknown process function `std::process::{func_name}`"),
            span,
        )),
    }
}

/// Execute a command and return a struct with stdout, stderr, and status code.
fn run_command(program: &str, args: &[&str], _span: &Span) -> Result<Value, PipelineError> {
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
                Value::I64(output.status.code().unwrap_or(-1) as i64),
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

/// Spawn a long-running command and stream stdout/stderr line-by-line through
/// the user callback. The callback receives `(line: String, stream: String)`
/// where `stream` is `"stdout"` or `"stderr"`. Returns a `CommandOutput` with
/// `status`/`success` (stdout/stderr fields are empty since output is streamed).
fn run_spawn(
    program: &str,
    args: &[String],
    callback: &Value,
    cb: crate::stdlib::registry::ClosureInvoker<'_>,
) -> Result<Value, PipelineError> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    use std::sync::mpsc;
    use std::thread;

    let mut child = match Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return Ok(Value::err(Value::String(e.to_string()))),
    };

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    let (tx, rx) = mpsc::channel::<(&'static str, String)>();
    let tx_out = tx.clone();
    let tx_err = tx;

    let out_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if tx_out.send(("stdout", line)).is_err() {
                break;
            }
        }
    });
    let err_handle = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            if tx_err.send(("stderr", line)).is_err() {
                break;
            }
        }
    });

    // Receive lines on the main thread (where the VM lives) and invoke the
    // user closure. If the callback errors we keep draining the channel so
    // the reader threads can finish cleanly, then surface the error.
    let mut cb_error: Option<String> = None;
    while let Ok((stream, line)) = rx.recv() {
        if cb_error.is_some() {
            continue;
        }
        let cb_args = [Value::String(line), Value::String(stream.to_string())];
        if let Err(e) = cb(callback, &cb_args) {
            cb_error = Some(e);
        }
    }

    let _ = out_handle.join();
    let _ = err_handle.join();

    let status = match child.wait() {
        Ok(s) => s,
        Err(e) => return Ok(Value::err(Value::String(e.to_string()))),
    };

    if let Some(e) = cb_error {
        return Ok(Value::err(Value::String(format!("callback error: {e}"))));
    }

    let mut fields = std::collections::HashMap::new();
    fields.insert("stdout".to_string(), Value::String(String::new()));
    fields.insert("stderr".to_string(), Value::String(String::new()));
    fields.insert(
        "status".to_string(),
        Value::I64(status.code().unwrap_or(-1) as i64),
    );
    fields.insert("success".to_string(), Value::Bool(status.success()));
    Ok(Value::ok(Value::Struct {
        name: "CommandOutput".to_string(),
        fields,
    }))
}

#[cfg(test)]
mod tests {
    use crate::vm::run_compiled_capturing;

    fn run(src: &str) -> String {
        let (_, output) = run_compiled_capturing(src).expect("runtime error");
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

    // `printf` on Windows resolves to a builtin / Git Bash variant that
    // interprets `%s` and newlines differently from POSIX `printf`, so this
    // test is Unix-only. The behavior under test (one callback invocation
    // per stdout line) is generic and exercised on Linux + macOS in CI.
    #[cfg(not(windows))]
    #[test]
    fn test_process_spawn_streams_stdout_lines() {
        // `printf` (the binary) emits two lines on stdout; the callback should
        // be invoked once per line, in order.
        let out = run(r#"
fn main() {
    let mut lines = vec![];
    let result = std::process::spawn(
        "printf",
        vec!["%s\n%s\n", "first", "second"],
        |line, stream| {
            lines.push(stream);
            lines.push(line);
        },
    );
    if let Ok(output) = result {
        println!("{}", output.success);
        for l in lines {
            println!("{}", l);
        }
    } else {
        println!("err");
    }
}
"#);
        assert_eq!(out, "true\nstdout\nfirst\nstdout\nsecond\n");
    }

    #[test]
    fn test_process_spawn_separates_stdout_and_stderr() {
        // `sh -c` lets us deterministically write to both streams.
        let out = run(r#"
fn main() {
    let mut tagged = vec![];
    let _ = std::process::spawn(
        "sh",
        vec!["-c", "echo out1; echo err1 1>&2; echo out2"],
        |line, stream| {
            tagged.push(stream + ":" + line);
        },
    );
    // Sort for determinism — interleaving between stdout/stderr is racy.
    tagged.sort();
    for t in tagged {
        println!("{}", t);
    }
}
"#);
        assert_eq!(out, "stderr:err1\nstdout:out1\nstdout:out2\n");
    }

    #[test]
    fn test_process_spawn_status_on_failure() {
        let out = run(r#"
fn main() {
    let result = std::process::spawn("false", vec![], |_line, _stream| {});
    if let Ok(output) = result {
        println!("{} {}", output.success, output.status);
    } else {
        println!("err");
    }
}
"#);
        assert_eq!(out, "false 1\n");
    }

    #[test]
    fn test_process_spawn_nonexistent_program() {
        let out = run(r#"
fn main() {
    let result = std::process::spawn(
        "nonexistent_program_xyz_98765",
        vec![],
        |_line, _stream| {},
    );
    if let Ok(_) = result {
        println!("ok");
    } else {
        println!("err");
    }
}
"#);
        assert_eq!(out, "err\n");
    }
}
