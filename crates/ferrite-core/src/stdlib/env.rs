//! Environment standard library module.
//!
//! Provides access to environment variables and process arguments.
//! Note: `std::env::args()` requires access to interpreter state (cli_args),
//! so it is handled in the interpreter's path dispatch. This module handles
//! the stateless functions.

use crate::errors::{check_arg_count, expect_string, runtime_error, FerriError};
use crate::lexer::Span;
use crate::types::Value;

/// Dispatch `std::env::` function calls (stateless ones).
pub fn call(func_name: &str, args: &[Value], span: &Span) -> Result<Value, FerriError> {
    match func_name {
        "var" => {
            check_arg_count("std::env::var", 1, args, span)?;
            let name = expect_string(&args[0], "std::env::var", span)?;
            match std::env::var(name) {
                Ok(val) => Ok(Value::some(Value::String(val))),
                Err(_) => Ok(Value::none()),
            }
        }
        "vars" => {
            check_arg_count("std::env::vars", 0, args, span)?;
            let map: std::collections::HashMap<String, Value> = std::env::vars()
                .map(|(k, v)| (k, Value::String(v)))
                .collect();
            Ok(Value::HashMap(map))
        }
        "current_dir" => {
            check_arg_count("std::env::current_dir", 0, args, span)?;
            match std::env::current_dir() {
                Ok(p) => Ok(Value::ok(Value::String(p.to_string_lossy().into_owned()))),
                Err(e) => Ok(Value::err(Value::String(e.to_string()))),
            }
        }
        "set_current_dir" => {
            check_arg_count("std::env::set_current_dir", 1, args, span)?;
            let path = expect_string(&args[0], "std::env::set_current_dir", span)?;
            match std::env::set_current_dir(path) {
                Ok(()) => Ok(Value::ok(Value::Unit)),
                Err(e) => Ok(Value::err(Value::String(e.to_string()))),
            }
        }
        "home_dir" => {
            check_arg_count("std::env::home_dir", 0, args, span)?;
            #[allow(deprecated)]
            match std::env::home_dir() {
                Some(p) => Ok(Value::some(Value::String(p.to_string_lossy().into_owned()))),
                None => Ok(Value::none()),
            }
        }
        _ => Err(runtime_error(
            format!("unknown env function `std::env::{func_name}`"),
            span,
        )),
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
    fn test_env_var_existing() {
        let out = run(r#"
fn main() {
    let val = std::env::var("PATH");
    if let Some(v) = val {
        println!("found");
    } else {
        println!("missing");
    }
}
"#);
        assert_eq!(out, "found\n");
    }

    #[test]
    fn test_env_var_nonexistent() {
        let out = run(r#"
fn main() {
    let val = std::env::var("FERRITE_NONEXISTENT_VAR_XYZ_12345");
    if let Some(v) = val {
        println!("found");
    } else {
        println!("none");
    }
}
"#);
        assert_eq!(out, "none\n");
    }

    #[test]
    fn test_env_vars_returns_hashmap() {
        let out = run(r#"
fn main() {
    let vars = std::env::vars();
    let len = vars.len();
    println!("{}", len > 0);
}
"#);
        assert_eq!(out, "true\n");
    }

    #[test]
    fn test_env_current_dir() {
        let out = run(r#"
fn main() {
    let result = std::env::current_dir();
    if let Ok(dir) = result {
        let len = dir.len();
        println!("{}", len > 0);
    } else {
        println!("err");
    }
}
"#);
        assert_eq!(out, "true\n");
    }

    #[test]
    fn test_env_home_dir() {
        let out = run(r#"
fn main() {
    let result = std::env::home_dir();
    if let Some(dir) = result {
        let len = dir.len();
        println!("{}", len > 0);
    } else {
        println!("none");
    }
}
"#);
        assert_eq!(out, "true\n");
    }
}
