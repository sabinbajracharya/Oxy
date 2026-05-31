//! Path manipulation standard library module.
//!
//! Lexical path operations that mirror `std::path::Path` without touching
//! the filesystem. Use `std::fs` for I/O queries (exists, is_file, etc.).
//!
//! All functions take and return `String`. Path separators in inputs are
//! tolerated in either `/` or `\` form on Windows; outputs use the
//! platform-native separator (`MAIN_SEPARATOR`).

use std::cell::RefCell;
use std::path::{Component, Path, PathBuf, MAIN_SEPARATOR};
use std::rc::Rc;

use crate::errors::{check_arg_count, expect_string, runtime_error, PipelineError};
use crate::lexer::Span;
use crate::types::Value;

/// Dispatch `std::path::` function calls.
pub fn call(
    func_name: &str,
    args: &[Value],
    span: &Span,
    _cb: crate::stdlib::registry::ClosureInvoker<'_>,
) -> Result<Value, PipelineError> {
    match func_name {
        "join" => {
            check_arg_count("std::path::join", 1, args, span)?;
            let parts = expect_vec_of_strings(&args[0], "std::path::join", span)?;
            let mut buf = PathBuf::new();
            for p in parts {
                buf.push(p);
            }
            Ok(Value::String(buf.to_string_lossy().into_owned()))
        }
        "dirname" => {
            check_arg_count("std::path::dirname", 1, args, span)?;
            let path = expect_string(&args[0], "std::path::dirname", span)?;
            let s = Path::new(path)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            Ok(Value::String(s))
        }
        "basename" => {
            check_arg_count("std::path::basename", 1, args, span)?;
            let path = expect_string(&args[0], "std::path::basename", span)?;
            let s = Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            Ok(Value::String(s))
        }
        "stem" => {
            check_arg_count("std::path::stem", 1, args, span)?;
            let path = expect_string(&args[0], "std::path::stem", span)?;
            let s = Path::new(path)
                .file_stem()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            Ok(Value::String(s))
        }
        "extension" => {
            check_arg_count("std::path::extension", 1, args, span)?;
            let path = expect_string(&args[0], "std::path::extension", span)?;
            match Path::new(path).extension() {
                Some(ext) => Ok(Value::some(Value::String(
                    ext.to_string_lossy().into_owned(),
                ))),
                None => Ok(Value::none()),
            }
        }
        "with_extension" => {
            check_arg_count("std::path::with_extension", 2, args, span)?;
            let path = expect_string(&args[0], "std::path::with_extension", span)?;
            let ext = expect_string(&args[1], "std::path::with_extension", span)?;
            let new_path = Path::new(path).with_extension(ext);
            Ok(Value::String(new_path.to_string_lossy().into_owned()))
        }
        "with_file_name" => {
            check_arg_count("std::path::with_file_name", 2, args, span)?;
            let path = expect_string(&args[0], "std::path::with_file_name", span)?;
            let name = expect_string(&args[1], "std::path::with_file_name", span)?;
            let new_path = Path::new(path).with_file_name(name);
            Ok(Value::String(new_path.to_string_lossy().into_owned()))
        }
        "is_absolute" => {
            check_arg_count("std::path::is_absolute", 1, args, span)?;
            let path = expect_string(&args[0], "std::path::is_absolute", span)?;
            Ok(Value::Bool(Path::new(path).is_absolute()))
        }
        "is_relative" => {
            check_arg_count("std::path::is_relative", 1, args, span)?;
            let path = expect_string(&args[0], "std::path::is_relative", span)?;
            Ok(Value::Bool(Path::new(path).is_relative()))
        }
        "components" => {
            check_arg_count("std::path::components", 1, args, span)?;
            let path = expect_string(&args[0], "std::path::components", span)?;
            let items: Vec<Value> = Path::new(path)
                .components()
                .map(|c| Value::String(c.as_os_str().to_string_lossy().into_owned()))
                .collect();
            Ok(Value::Vec(Rc::new(RefCell::new(items))))
        }
        "normalize" => {
            check_arg_count("std::path::normalize", 1, args, span)?;
            let path = expect_string(&args[0], "std::path::normalize", span)?;
            Ok(Value::String(normalize_lexically(path)))
        }
        "separator" => {
            check_arg_count("std::path::separator", 0, args, span)?;
            Ok(Value::String(MAIN_SEPARATOR.to_string()))
        }
        _ => Err(runtime_error(
            format!("unknown path function `std::path::{func_name}`"),
            span,
        )),
    }
}

/// Collapse `.` and `..` segments lexically. Does not touch the filesystem,
/// so it does not resolve symlinks. An absolute path stays absolute; a
/// relative path stays relative. Leading `..` segments on a relative path
/// are preserved (they can't be collapsed without a root).
fn normalize_lexically(path: &str) -> String {
    let p = Path::new(path);
    let mut stack: Vec<Component> = Vec::new();
    for comp in p.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => match stack.last() {
                Some(Component::Normal(_)) => {
                    stack.pop();
                }
                Some(Component::ParentDir) | None => stack.push(comp),
                Some(Component::RootDir) | Some(Component::Prefix(_)) => {}
                Some(Component::CurDir) => {
                    stack.pop();
                    stack.push(comp);
                }
            },
            _ => stack.push(comp),
        }
    }
    if stack.is_empty() {
        return ".".to_string();
    }
    let mut buf = PathBuf::new();
    for c in stack {
        buf.push(c.as_os_str());
    }
    buf.to_string_lossy().into_owned()
}

fn expect_vec_of_strings(v: &Value, func: &str, span: &Span) -> Result<Vec<String>, PipelineError> {
    match v {
        Value::Vec(items) => {
            let mut out = Vec::new();
            for item in items.borrow().iter() {
                match item {
                    Value::String(s) => out.push(s.clone()),
                    other => {
                        return Err(runtime_error(
                            format!(
                                "{func}: expected Vec<String>, got element of type `{}`",
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
            format!("{func}: expected Vec<String>, got `{}`", other.type_name()),
            span,
        )),
    }
}

#[cfg(test)]
mod tests {
    use crate::vm::run_compiled_capturing;

    fn run(src: &str) -> String {
        let (_, output) = run_compiled_capturing(src).expect("runtime error");
        output.join("")
    }

    #[test]
    fn test_path_join_two() {
        let out = run(r#"
fn main() {
    val p = std::path::join(["a".to_string(), "b".to_string()]);
    println("{}", p);
}
"#);
        assert!(out.trim_end().ends_with("b"));
        assert!(out.contains("a"));
    }

    #[test]
    fn test_path_basename() {
        let out = run(r#"
fn main() {
    val n = std::path::basename("/a/b/c.txt");
    println("{}", n);
}
"#);
        assert_eq!(out, "c.txt\n");
    }

    #[test]
    fn test_path_extension() {
        let out = run(r#"
fn main() {
    val e = std::path::extension("foo/bar.tar.gz");
    if val Some(s) = e { println("{}", s); } else { println("none"); }
}
"#);
        assert_eq!(out, "gz\n");
    }

    #[test]
    fn test_path_normalize_dots() {
        let out = run(r#"
fn main() {
    val n = std::path::normalize("a/b/./../c");
    println("{}", n);
}
"#);
        let trimmed = out.trim_end();
        assert!(trimmed == "a/c" || trimmed == "a\\c", "got: {trimmed}");
    }
}
