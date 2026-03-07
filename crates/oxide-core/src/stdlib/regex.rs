//! Regular expression standard library module.
//!
//! Provides pattern matching, searching, and replacement using Rust's `regex` crate.
//! Patterns follow Rust/Perl-style regex syntax.

use crate::errors::{check_arg_count, expect_string, runtime_error, FerriError};
use crate::lexer::Span;
use crate::types::Value;

/// Dispatch `std::regex::` function calls.
pub fn call(func_name: &str, args: &[Value], span: &Span) -> Result<Value, FerriError> {
    match func_name {
        "is_match" => {
            check_arg_count("std::regex::is_match", 2, args, span)?;
            let pattern = expect_string(&args[0], "std::regex::is_match(pattern)", span)?;
            let text = expect_string(&args[1], "std::regex::is_match(text)", span)?;
            let re = compile_regex(pattern, span)?;
            Ok(Value::Bool(re.is_match(text)))
        }
        "find" => {
            check_arg_count("std::regex::find", 2, args, span)?;
            let pattern = expect_string(&args[0], "std::regex::find(pattern)", span)?;
            let text = expect_string(&args[1], "std::regex::find(text)", span)?;
            let re = compile_regex(pattern, span)?;
            match re.find(text) {
                Some(m) => Ok(Value::some(match_to_value(m))),
                None => Ok(Value::none()),
            }
        }
        "find_all" => {
            check_arg_count("std::regex::find_all", 2, args, span)?;
            let pattern = expect_string(&args[0], "std::regex::find_all(pattern)", span)?;
            let text = expect_string(&args[1], "std::regex::find_all(text)", span)?;
            let re = compile_regex(pattern, span)?;
            let matches: Vec<Value> = re.find_iter(text).map(match_to_value).collect();
            Ok(Value::Vec(matches))
        }
        "captures" => {
            check_arg_count("std::regex::captures", 2, args, span)?;
            let pattern = expect_string(&args[0], "std::regex::captures(pattern)", span)?;
            let text = expect_string(&args[1], "std::regex::captures(text)", span)?;
            let re = compile_regex(pattern, span)?;
            match re.captures(text) {
                Some(caps) => {
                    let mut map = std::collections::HashMap::new();
                    // Numeric groups
                    for (i, m) in caps.iter().enumerate() {
                        if let Some(m) = m {
                            map.insert(i.to_string(), Value::String(m.as_str().to_string()));
                        }
                    }
                    // Named groups
                    for name in re.capture_names().flatten() {
                        if let Some(m) = caps.name(name) {
                            map.insert(name.to_string(), Value::String(m.as_str().to_string()));
                        }
                    }
                    Ok(Value::some(Value::HashMap(map)))
                }
                None => Ok(Value::none()),
            }
        }
        "replace" => {
            check_arg_count("std::regex::replace", 3, args, span)?;
            let pattern = expect_string(&args[0], "std::regex::replace(pattern)", span)?;
            let text = expect_string(&args[1], "std::regex::replace(text)", span)?;
            let replacement = expect_string(&args[2], "std::regex::replace(replacement)", span)?;
            let re = compile_regex(pattern, span)?;
            Ok(Value::String(re.replace(text, replacement).into_owned()))
        }
        "replace_all" => {
            check_arg_count("std::regex::replace_all", 3, args, span)?;
            let pattern = expect_string(&args[0], "std::regex::replace_all(pattern)", span)?;
            let text = expect_string(&args[1], "std::regex::replace_all(text)", span)?;
            let replacement =
                expect_string(&args[2], "std::regex::replace_all(replacement)", span)?;
            let re = compile_regex(pattern, span)?;
            Ok(Value::String(
                re.replace_all(text, replacement).into_owned(),
            ))
        }
        "split" => {
            check_arg_count("std::regex::split", 2, args, span)?;
            let pattern = expect_string(&args[0], "std::regex::split(pattern)", span)?;
            let text = expect_string(&args[1], "std::regex::split(text)", span)?;
            let re = compile_regex(pattern, span)?;
            let parts: Vec<Value> = re
                .split(text)
                .map(|s| Value::String(s.to_string()))
                .collect();
            Ok(Value::Vec(parts))
        }
        _ => Err(runtime_error(
            format!("unknown regex function `std::regex::{func_name}`"),
            span,
        )),
    }
}

/// Compile a regex pattern, returning a user-friendly error on invalid syntax.
fn compile_regex(pattern: &str, span: &Span) -> Result<regex::Regex, FerriError> {
    regex::Regex::new(pattern)
        .map_err(|e| runtime_error(format!("invalid regex pattern `{pattern}`: {e}"), span))
}

/// Convert a regex `Match` into a Oxide struct with text, start, end fields.
fn match_to_value(m: regex::Match<'_>) -> Value {
    let mut fields = std::collections::HashMap::new();
    fields.insert("text".to_string(), Value::String(m.as_str().to_string()));
    fields.insert("start".to_string(), Value::Integer(m.start() as i64));
    fields.insert("end".to_string(), Value::Integer(m.end() as i64));
    Value::Struct {
        name: "Match".to_string(),
        fields,
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
    fn test_regex_is_match_true() {
        let out = run("fn main() {\n\
             let matched = std::regex::is_match(\"\\\\d+\", \"abc123def\");\n\
             println!(\"{}\", matched);\n\
             }");
        assert_eq!(out, "true\n");
    }

    #[test]
    fn test_regex_is_match_false() {
        let out = run("fn main() {\n\
             let matched = std::regex::is_match(\"\\\\d+\", \"abcdef\");\n\
             println!(\"{}\", matched);\n\
             }");
        assert_eq!(out, "false\n");
    }

    #[test]
    fn test_regex_find_some() {
        let out = run("fn main() {\n\
             let result = std::regex::find(\"\\\\d+\", \"abc123def\");\n\
             if let Some(m) = result {\n\
             println!(\"{}\", m.text);\n\
             println!(\"{}\", m.start);\n\
             println!(\"{}\", m.end);\n\
             }\n\
             }");
        assert_eq!(out, "123\n3\n6\n");
    }

    #[test]
    fn test_regex_find_none() {
        let out = run("fn main() {\n\
             let result = std::regex::find(\"\\\\d+\", \"abcdef\");\n\
             if let Some(m) = result {\n\
             println!(\"found\");\n\
             } else {\n\
             println!(\"none\");\n\
             }\n\
             }");
        assert_eq!(out, "none\n");
    }

    #[test]
    fn test_regex_find_all() {
        let out = run("fn main() {\n\
             let matches = std::regex::find_all(\"\\\\d+\", \"a1b22c333\");\n\
             for m in matches {\n\
             println!(\"{}\", m.text);\n\
             }\n\
             }");
        assert_eq!(out, "1\n22\n333\n");
    }

    #[test]
    fn test_regex_captures_named() {
        let out = run(
            "fn main() {\n\
             let result = std::regex::captures(\"(?P<year>\\\\d{4})-(?P<month>\\\\d{2})\", \"date: 2024-07-01\");\n\
             if let Some(caps) = result {\n\
             println!(\"{}\", caps.get(\"year\").unwrap());\n\
             println!(\"{}\", caps.get(\"month\").unwrap());\n\
             }\n\
             }",
        );
        assert_eq!(out, "2024\n07\n");
    }

    #[test]
    fn test_regex_captures_none() {
        let out = run("fn main() {\n\
             let result = std::regex::captures(\"(\\\\d+)-(\\\\d+)\", \"no match here\");\n\
             if let Some(caps) = result {\n\
             println!(\"found\");\n\
             } else {\n\
             println!(\"none\");\n\
             }\n\
             }");
        assert_eq!(out, "none\n");
    }

    #[test]
    fn test_regex_replace_first() {
        let out = run("fn main() {\n\
             let result = std::regex::replace(\"\\\\d+\", \"a1b2c3\", \"X\");\n\
             println!(\"{}\", result);\n\
             }");
        assert_eq!(out, "aXb2c3\n");
    }

    #[test]
    fn test_regex_replace_all() {
        let out = run("fn main() {\n\
             let result = std::regex::replace_all(\"\\\\d+\", \"a1b2c3\", \"X\");\n\
             println!(\"{}\", result);\n\
             }");
        assert_eq!(out, "aXbXcX\n");
    }

    #[test]
    fn test_regex_split() {
        let out = run(r#"
fn main() {
    let parts = std::regex::split("[,;]+", "a,b;;c,d");
    for p in parts {
        println!("{}", p);
    }
}
"#);
        assert_eq!(out, "a\nb\nc\nd\n");
    }

    #[test]
    fn test_regex_invalid_pattern() {
        let result = run_capturing(
            r#"
fn main() {
    let x = std::regex::is_match("[invalid", "text");
}
"#,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid regex"));
    }
}
