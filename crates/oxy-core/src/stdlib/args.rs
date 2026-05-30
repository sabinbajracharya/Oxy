//! Argument parsing helper for CLI scripts.
//!
//! Turns argv into a structured `Args { program, flags, positionals }`
//! without requiring a schema/spec. The parsing rules are intentionally
//! small and unambiguous:
//!
//! ```text
//!   --key=val     flags["key"]   = "val"
//!   --key         flags["key"]   = ""        (presence / boolean)
//!   -k=val        flags["k"]     = "val"
//!   -k            flags["k"]     = ""
//!   --            terminator: everything after is positional
//!   -             positional (stdin convention)
//!   anything else positional
//! ```

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::errors::{check_arg_count, runtime_error, PipelineError};
use crate::lexer::Span;
use crate::types::Value;

/// Dispatch `std::args::` function calls.
pub fn call(
    func_name: &str,
    args: &[Value],
    span: &Span,
    _cb: crate::stdlib::registry::ClosureInvoker<'_>,
) -> Result<Value, PipelineError> {
    match func_name {
        "parse" => {
            check_arg_count("std::args::parse", 0, args, span)?;
            let argv = crate::stdlib::env::get_cli_args();
            Ok(build_args_value(parse_argv(&argv)))
        }
        "parse_from" => {
            check_arg_count("std::args::parse_from", 1, args, span)?;
            let argv = expect_vec_of_strings(&args[0], "std::args::parse_from", span)?;
            Ok(build_args_value(parse_argv(&argv)))
        }
        other => Err(runtime_error(
            format!("no function 'std::args::{other}'"),
            span,
        )),
    }
}

struct ParsedArgs {
    program: String,
    flags: HashMap<String, String>,
    positionals: Vec<String>,
}

/// Parse argv into program + flags + positionals. `argv[0]` is treated as
/// the program/script path; `argv[1..]` is parsed using the rules above.
fn parse_argv(argv: &[String]) -> ParsedArgs {
    let program = argv.first().cloned().unwrap_or_default();
    let mut flags = HashMap::new();
    let mut positionals = Vec::new();
    let mut after_terminator = false;
    for arg in argv.iter().skip(1) {
        if after_terminator {
            positionals.push(arg.clone());
            continue;
        }
        if arg == "--" {
            after_terminator = true;
            continue;
        }
        if arg == "-" || !arg.starts_with('-') {
            positionals.push(arg.clone());
            continue;
        }
        let stripped = arg
            .strip_prefix("--")
            .or_else(|| arg.strip_prefix('-'))
            .expect("arg starts with '-' (checked above)");
        if let Some(eq_idx) = stripped.find('=') {
            let key = &stripped[..eq_idx];
            let val = &stripped[eq_idx + 1..];
            flags.insert(key.to_string(), val.to_string());
        } else {
            flags.insert(stripped.to_string(), String::new());
        }
    }
    ParsedArgs {
        program,
        flags,
        positionals,
    }
}

fn build_args_value(p: ParsedArgs) -> Value {
    let mut fields = HashMap::new();
    fields.insert("program".to_string(), Value::String(p.program));
    let flags_map: HashMap<Value, Value> = p
        .flags
        .into_iter()
        .map(|(k, v)| (Value::String(k), Value::String(v)))
        .collect();
    fields.insert(
        "flags".to_string(),
        Value::HashMap(Rc::new(RefCell::new(flags_map))),
    );
    let positionals_vec: Vec<Value> = p.positionals.into_iter().map(Value::String).collect();
    fields.insert(
        "positionals".to_string(),
        Value::Vec(Rc::new(RefCell::new(positionals_vec))),
    );
    Value::Struct {
        name: "Args".to_string(),
        fields,
    }
}

fn expect_vec_of_strings(v: &Value, name: &str, span: &Span) -> Result<Vec<String>, PipelineError> {
    match v {
        Value::Vec(rc) => {
            let mut out = Vec::new();
            for (i, x) in rc.borrow().iter().enumerate() {
                match x {
                    Value::String(s) => out.push(s.clone()),
                    other => {
                        return Err(runtime_error(
                            format!(
                                "{name}: argv[{i}] expected String, got {}",
                                other.type_name()
                            ),
                            span,
                        ));
                    }
                }
            }
            Ok(out)
        }
        other => Err(runtime_error(
            format!("{name}: expected Vec<String>, got {}", other.type_name()),
            span,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> String {
        x.to_string()
    }

    #[test]
    fn parse_empty_argv() {
        let p = parse_argv(&[]);
        assert_eq!(p.program, "");
        assert!(p.flags.is_empty());
        assert!(p.positionals.is_empty());
    }

    #[test]
    fn parse_program_only() {
        let p = parse_argv(&[s("script.ox")]);
        assert_eq!(p.program, "script.ox");
        assert!(p.flags.is_empty());
        assert!(p.positionals.is_empty());
    }

    #[test]
    fn parse_long_flag_presence() {
        let p = parse_argv(&[s("p"), s("--verbose")]);
        assert_eq!(p.flags.get("verbose").map(String::as_str), Some(""));
    }

    #[test]
    fn parse_long_flag_with_value() {
        let p = parse_argv(&[s("p"), s("--name=alice")]);
        assert_eq!(p.flags.get("name").map(String::as_str), Some("alice"));
    }

    #[test]
    fn parse_short_flag_with_value() {
        let p = parse_argv(&[s("p"), s("-k=v")]);
        assert_eq!(p.flags.get("k").map(String::as_str), Some("v"));
    }

    #[test]
    fn parse_short_flag_presence() {
        let p = parse_argv(&[s("p"), s("-v")]);
        assert_eq!(p.flags.get("v").map(String::as_str), Some(""));
    }

    #[test]
    fn parse_positionals() {
        let p = parse_argv(&[s("p"), s("a"), s("b"), s("c")]);
        assert_eq!(p.positionals, vec!["a", "b", "c"]);
        assert!(p.flags.is_empty());
    }

    #[test]
    fn parse_mixed_flags_and_positionals() {
        let p = parse_argv(&[
            s("p"),
            s("--verbose"),
            s("file1"),
            s("--name=bob"),
            s("file2"),
        ]);
        assert_eq!(p.flags.get("verbose").map(String::as_str), Some(""));
        assert_eq!(p.flags.get("name").map(String::as_str), Some("bob"));
        assert_eq!(p.positionals, vec!["file1", "file2"]);
    }

    #[test]
    fn parse_double_dash_terminator() {
        let p = parse_argv(&[s("p"), s("--verbose"), s("--"), s("--not-a-flag"), s("x")]);
        assert_eq!(p.flags.get("verbose").map(String::as_str), Some(""));
        assert_eq!(p.positionals, vec!["--not-a-flag", "x"]);
    }

    #[test]
    fn parse_bare_dash_is_positional() {
        let p = parse_argv(&[s("p"), s("-")]);
        assert_eq!(p.positionals, vec!["-"]);
        assert!(p.flags.is_empty());
    }

    #[test]
    fn parse_value_containing_equals() {
        // Only the first `=` is the separator.
        let p = parse_argv(&[s("p"), s("--query=a=b=c")]);
        assert_eq!(p.flags.get("query").map(String::as_str), Some("a=b=c"));
    }

    #[test]
    fn parse_later_flag_overrides_earlier() {
        let p = parse_argv(&[s("p"), s("--name=first"), s("--name=second")]);
        assert_eq!(p.flags.get("name").map(String::as_str), Some("second"));
    }
}
