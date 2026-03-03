//! File system standard library module.
//!
//! Provides file and directory operations mirroring `std::fs`.
//! All fallible operations return `Result<T, String>`.

use crate::errors::{check_arg_count, expect_string, runtime_error, FerriError};
use crate::lexer::Span;
use crate::types::Value;

/// Dispatch `std::fs::` function calls.
pub fn call(func_name: &str, args: &[Value], span: &Span) -> Result<Value, FerriError> {
    match func_name {
        // --- File content operations ---
        "read_to_string" => {
            check_arg_count("std::fs::read_to_string", 1, args, span)?;
            let path = expect_string(&args[0], "std::fs::read_to_string", span)?;
            Ok(io_result(std::fs::read_to_string(path)))
        }
        "write" => {
            check_arg_count("std::fs::write", 2, args, span)?;
            let path = expect_string(&args[0], "std::fs::write", span)?;
            let content = format!("{}", args[1]);
            Ok(io_unit_result(std::fs::write(path, content)))
        }
        "append" => {
            check_arg_count("std::fs::append", 2, args, span)?;
            let path = expect_string(&args[0], "std::fs::append", span)?;
            let content = format!("{}", args[1]);
            Ok(io_unit_result(
                std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(path)
                    .and_then(|mut f| std::io::Write::write_all(&mut f, content.as_bytes())),
            ))
        }

        // --- File/directory queries ---
        "exists" => {
            check_arg_count("std::fs::exists", 1, args, span)?;
            let path = expect_string(&args[0], "std::fs::exists", span)?;
            Ok(Value::Bool(std::path::Path::new(path).exists()))
        }
        "is_file" => {
            check_arg_count("std::fs::is_file", 1, args, span)?;
            let path = expect_string(&args[0], "std::fs::is_file", span)?;
            Ok(Value::Bool(std::path::Path::new(path).is_file()))
        }
        "is_dir" => {
            check_arg_count("std::fs::is_dir", 1, args, span)?;
            let path = expect_string(&args[0], "std::fs::is_dir", span)?;
            Ok(Value::Bool(std::path::Path::new(path).is_dir()))
        }

        // --- Directory operations ---
        "create_dir" => {
            check_arg_count("std::fs::create_dir", 1, args, span)?;
            let path = expect_string(&args[0], "std::fs::create_dir", span)?;
            Ok(io_unit_result(std::fs::create_dir(path)))
        }
        "create_dir_all" => {
            check_arg_count("std::fs::create_dir_all", 1, args, span)?;
            let path = expect_string(&args[0], "std::fs::create_dir_all", span)?;
            Ok(io_unit_result(std::fs::create_dir_all(path)))
        }
        "read_dir" => {
            check_arg_count("std::fs::read_dir", 1, args, span)?;
            let path = expect_string(&args[0], "std::fs::read_dir", span)?;
            match std::fs::read_dir(path) {
                Ok(entries) => {
                    let mut names = Vec::new();
                    for entry in entries.flatten() {
                        names.push(Value::String(
                            entry.file_name().to_string_lossy().into_owned(),
                        ));
                    }
                    Ok(Value::ok(Value::Vec(names)))
                }
                Err(e) => Ok(Value::err(Value::String(e.to_string()))),
            }
        }

        // --- File manipulation ---
        "remove_file" => {
            check_arg_count("std::fs::remove_file", 1, args, span)?;
            let path = expect_string(&args[0], "std::fs::remove_file", span)?;
            Ok(io_unit_result(std::fs::remove_file(path)))
        }
        "remove_dir" => {
            check_arg_count("std::fs::remove_dir", 1, args, span)?;
            let path = expect_string(&args[0], "std::fs::remove_dir", span)?;
            Ok(io_unit_result(std::fs::remove_dir(path)))
        }
        "remove_dir_all" => {
            check_arg_count("std::fs::remove_dir_all", 1, args, span)?;
            let path = expect_string(&args[0], "std::fs::remove_dir_all", span)?;
            Ok(io_unit_result(std::fs::remove_dir_all(path)))
        }
        "rename" => {
            check_arg_count("std::fs::rename", 2, args, span)?;
            let from = expect_string(&args[0], "std::fs::rename", span)?;
            let to = expect_string(&args[1], "std::fs::rename", span)?;
            Ok(io_unit_result(std::fs::rename(from, to)))
        }
        "copy" => {
            check_arg_count("std::fs::copy", 2, args, span)?;
            let from = expect_string(&args[0], "std::fs::copy", span)?;
            let to = expect_string(&args[1], "std::fs::copy", span)?;
            match std::fs::copy(from, to) {
                Ok(bytes) => Ok(Value::ok(Value::Integer(bytes as i64))),
                Err(e) => Ok(Value::err(Value::String(e.to_string()))),
            }
        }
        "canonicalize" => {
            check_arg_count("std::fs::canonicalize", 1, args, span)?;
            let path = expect_string(&args[0], "std::fs::canonicalize", span)?;
            match std::fs::canonicalize(path) {
                Ok(p) => Ok(Value::ok(Value::String(p.to_string_lossy().into_owned()))),
                Err(e) => Ok(Value::err(Value::String(e.to_string()))),
            }
        }

        // --- Metadata ---
        "metadata" => {
            check_arg_count("std::fs::metadata", 1, args, span)?;
            let path = expect_string(&args[0], "std::fs::metadata", span)?;
            match std::fs::metadata(path) {
                Ok(meta) => {
                    let mut fields = std::collections::HashMap::new();
                    fields.insert("size".to_string(), Value::Integer(meta.len() as i64));
                    fields.insert("is_file".to_string(), Value::Bool(meta.is_file()));
                    fields.insert("is_dir".to_string(), Value::Bool(meta.is_dir()));
                    fields.insert(
                        "readonly".to_string(),
                        Value::Bool(meta.permissions().readonly()),
                    );
                    if let Ok(modified) = meta.modified() {
                        if let Ok(dur) = modified.duration_since(std::time::UNIX_EPOCH) {
                            fields.insert("modified".to_string(), Value::Float(dur.as_secs_f64()));
                        }
                    }
                    Ok(Value::ok(Value::Struct {
                        name: "Metadata".to_string(),
                        fields,
                    }))
                }
                Err(e) => Ok(Value::err(Value::String(e.to_string()))),
            }
        }

        _ => Err(runtime_error(
            format!("unknown fs function `std::fs::{func_name}`"),
            span,
        )),
    }
}

/// Convert a `std::io::Result<String>` into a Ferrite `Result` value.
fn io_result(result: std::io::Result<String>) -> Value {
    match result {
        Ok(content) => Value::ok(Value::String(content)),
        Err(e) => Value::err(Value::String(e.to_string())),
    }
}

/// Convert a `std::io::Result<()>` into a Ferrite `Result` value.
fn io_unit_result(result: std::io::Result<()>) -> Value {
    match result {
        Ok(()) => Value::ok(Value::Unit),
        Err(e) => Value::err(Value::String(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use crate::interpreter::run_capturing;

    fn run(src: &str) -> String {
        let (_, output) = run_capturing(src).expect("runtime error");
        output.join("")
    }

    /// Return a cross-platform temp directory path (no trailing slash).
    fn tmp() -> String {
        let mut p = std::env::temp_dir()
            .to_string_lossy()
            .to_string()
            .replace('\\', "/");
        if p.ends_with('/') {
            p.pop();
        }
        p
    }

    #[test]
    fn test_fs_write_and_read_roundtrip() {
        let t = tmp();
        let out = run(&format!(
            r#"
fn main() {{
    let path = "{t}/ferrite_test_roundtrip_9a1b.txt";
    let w = std::fs::write(path, "hello ferrite");
    let result = std::fs::read_to_string(path);
    if let Ok(content) = result {{
        println!("{{}}", content);
    }}
    let d = std::fs::remove_file(path);
}}
"#
        ));
        assert_eq!(out, "hello ferrite\n");
    }

    #[test]
    fn test_fs_exists_true() {
        let t = tmp();
        let out = run(&format!(
            r#"
fn main() {{
    let path = "{t}/ferrite_test_exists_7c2d.txt";
    let w = std::fs::write(path, "data");
    println!("{{}}", std::fs::exists(path));
    let d = std::fs::remove_file(path);
}}
"#
        ));
        assert_eq!(out, "true\n");
    }

    #[test]
    fn test_fs_exists_false() {
        let t = tmp();
        let out = run(&format!(
            r#"
fn main() {{
    println!("{{}}", std::fs::exists("{t}/ferrite_nonexistent_file_xyz_00.txt"));
}}
"#
        ));
        assert_eq!(out, "false\n");
    }

    #[test]
    fn test_fs_is_file_and_is_dir() {
        let t = tmp();
        let out = run(&format!(
            r#"
fn main() {{
    let path = "{t}/ferrite_test_isfile_3e4f.txt";
    let w = std::fs::write(path, "test");
    println!("{{}}", std::fs::is_file(path));
    println!("{{}}", std::fs::is_dir(path));
    println!("{{}}", std::fs::is_dir("{t}"));
    let d = std::fs::remove_file(path);
}}
"#
        ));
        assert_eq!(out, "true\nfalse\ntrue\n");
    }

    #[test]
    fn test_fs_create_dir_and_remove_dir() {
        let t = tmp();
        let out = run(&format!(
            r#"
fn main() {{
    let dir = "{t}/ferrite_test_dir_a8b9";
    let c = std::fs::create_dir(dir);
    println!("{{}}", std::fs::is_dir(dir));
    let d = std::fs::remove_dir(dir);
    println!("{{}}", std::fs::exists(dir));
}}
"#
        ));
        assert_eq!(out, "true\nfalse\n");
    }

    #[test]
    fn test_fs_create_dir_all() {
        let t = tmp();
        let out = run(&format!(
            r#"
fn main() {{
    let dir = "{t}/ferrite_test_nested_c1d2/sub1/sub2";
    let c = std::fs::create_dir_all(dir);
    println!("{{}}", std::fs::is_dir(dir));
    let d = std::fs::remove_dir_all("{t}/ferrite_test_nested_c1d2");
    println!("{{}}", std::fs::exists("{t}/ferrite_test_nested_c1d2"));
}}
"#
        ));
        assert_eq!(out, "true\nfalse\n");
    }

    #[test]
    fn test_fs_rename() {
        let t = tmp();
        let out = run(&format!(
            r#"
fn main() {{
    let src = "{t}/ferrite_test_rename_src_5e6f.txt";
    let dst = "{t}/ferrite_test_rename_dst_5e6f.txt";
    let w = std::fs::write(src, "rename me");
    let r = std::fs::rename(src, dst);
    println!("{{}}", std::fs::exists(src));
    let result = std::fs::read_to_string(dst);
    if let Ok(content) = result {{
        println!("{{}}", content);
    }}
    let d = std::fs::remove_file(dst);
}}
"#
        ));
        assert_eq!(out, "false\nrename me\n");
    }

    #[test]
    fn test_fs_copy() {
        let t = tmp();
        let out = run(&format!(
            r#"
fn main() {{
    let src = "{t}/ferrite_test_copy_src_7g8h.txt";
    let dst = "{t}/ferrite_test_copy_dst_7g8h.txt";
    let w = std::fs::write(src, "copy me");
    let result = std::fs::copy(src, dst);
    if let Ok(bytes) = result {{
        println!("{{}}", bytes > 0);
    }}
    let read_result = std::fs::read_to_string(dst);
    if let Ok(content) = read_result {{
        println!("{{}}", content);
    }}
    let d1 = std::fs::remove_file(src);
    let d2 = std::fs::remove_file(dst);
}}
"#
        ));
        assert_eq!(out, "true\ncopy me\n");
    }

    #[test]
    fn test_fs_metadata_size() {
        let t = tmp();
        let out = run(&format!(
            r#"
fn main() {{
    let path = "{t}/ferrite_test_meta_9i0j.txt";
    let w = std::fs::write(path, "12345");
    let result = std::fs::metadata(path);
    if let Ok(meta) = result {{
        println!("{{}}", meta.size);
        println!("{{}}", meta.is_file);
        println!("{{}}", meta.is_dir);
    }}
    let d = std::fs::remove_file(path);
}}
"#
        ));
        assert_eq!(out, "5\ntrue\nfalse\n");
    }

    #[test]
    fn test_fs_read_dir() {
        let t = tmp();
        let out = run(&format!(
            r#"
fn main() {{
    let dir = "{t}/ferrite_test_readdir_k1l2";
    let c = std::fs::create_dir(dir);
    let w1 = std::fs::write("{t}/ferrite_test_readdir_k1l2/a.txt", "a");
    let w2 = std::fs::write("{t}/ferrite_test_readdir_k1l2/b.txt", "b");
    let result = std::fs::read_dir(dir);
    if let Ok(entries) = result {{
        println!("{{}}", entries.len());
    }}
    let d = std::fs::remove_dir_all(dir);
}}
"#
        ));
        assert_eq!(out, "2\n");
    }

    #[test]
    fn test_fs_append() {
        let t = tmp();
        let out = run(&format!(
            r#"
fn main() {{
    let path = "{t}/ferrite_test_append_m3n4.txt";
    let w = std::fs::write(path, "hello");
    let a = std::fs::append(path, " world");
    let result = std::fs::read_to_string(path);
    if let Ok(content) = result {{
        println!("{{}}", content);
    }}
    let d = std::fs::remove_file(path);
}}
"#
        ));
        assert_eq!(out, "hello world\n");
    }

    #[test]
    fn test_fs_canonicalize() {
        let t = tmp();
        let out = run(&format!(
            r#"
fn main() {{
    let result = std::fs::canonicalize("{t}");
    if let Ok(path) = result {{
        let len = path.len();
        println!("{{}}", len > 0);
    }} else {{
        println!("err");
    }}
}}
"#
        ));
        assert_eq!(out, "true\n");
    }

    #[test]
    fn test_fs_remove_file() {
        let t = tmp();
        let out = run(&format!(
            r#"
fn main() {{
    let path = "{t}/ferrite_test_remove_o5p6.txt";
    let w = std::fs::write(path, "delete me");
    println!("{{}}", std::fs::exists(path));
    let d = std::fs::remove_file(path);
    println!("{{}}", std::fs::exists(path));
}}
"#
        ));
        assert_eq!(out, "true\nfalse\n");
    }
}
