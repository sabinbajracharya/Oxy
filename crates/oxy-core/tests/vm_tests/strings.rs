//! String/char methods, substrings, f-string interpolation.
//!
//! Shared helpers and imports come from the parent module (`use super::*`).

use super::*;

#[test]
fn test_char_is_digit() {
    let output = run_and_capture(
        r#"
fn main() {
    println("{}", '5'.is_digit());
    println("{}", 'a'.is_digit());
}
"#,
    );
    assert_eq!(output, vec!["true\n", "false\n"]);
}

#[test]
fn test_char_is_alphabetic() {
    let output = run_and_capture(
        r#"
fn main() {
    println("{}", 'a'.is_alphabetic());
    println("{}", '5'.is_alphabetic());
}
"#,
    );
    assert_eq!(output, vec!["true\n", "false\n"]);
}

#[test]
fn test_char_is_alphanumeric() {
    let output = run_and_capture(
        r#"
fn main() {
    println("{}", 'a'.is_alphanumeric());
    println("{}", '5'.is_alphanumeric());
    println("{}", ' '.is_alphanumeric());
}
"#,
    );
    assert_eq!(output, vec!["true\n", "true\n", "false\n"]);
}

#[test]
fn test_char_is_whitespace() {
    let output = run_and_capture(
        r#"
fn main() {
    println("{}", ' '.is_whitespace());
    println("{}", '\t'.is_whitespace());
    println("{}", 'a'.is_whitespace());
}
"#,
    );
    assert_eq!(output, vec!["true\n", "true\n", "false\n"]);
}

#[test]
fn test_char_is_lowercase() {
    let output = run_and_capture(
        r#"
fn main() {
    println("{}", 'a'.is_lowercase());
    println("{}", 'A'.is_lowercase());
}
"#,
    );
    assert_eq!(output, vec!["true\n", "false\n"]);
}

#[test]
fn test_char_is_uppercase() {
    let output = run_and_capture(
        r#"
fn main() {
    println("{}", 'A'.is_uppercase());
    println("{}", 'a'.is_uppercase());
}
"#,
    );
    assert_eq!(output, vec!["true\n", "false\n"]);
}

#[test]
fn test_char_to_uppercase() {
    let output = run_and_capture(
        r#"
fn main() {
    println("{}", 'a'.to_uppercase());
    println("{}", 'A'.to_uppercase());
}
"#,
    );
    assert_eq!(output, vec!["A\n", "A\n"]);
}

#[test]
fn test_char_to_lowercase() {
    let output = run_and_capture(
        r#"
fn main() {
    println("{}", 'A'.to_lowercase());
    println("{}", 'a'.to_lowercase());
}
"#,
    );
    assert_eq!(output, vec!["a\n", "a\n"]);
}

#[test]
fn test_string_char_at() {
    let output = run_and_capture(
        r#"
fn main() {
    let s = "hello";
    println("{}", s.char_at(0));
    println("{}", s.char_at(4));
}
"#,
    );
    assert_eq!(output, vec!["h\n", "o\n"]);
}

#[test]
fn test_string_substring() {
    let output = run_and_capture(
        r#"
fn main() {
    let s = "hello world";
    println("{}", s.substring(0, 5));
    println("{}", s.substring(6, 11));
}
"#,
    );
    assert_eq!(output, vec!["hello\n", "world\n"]);
}

#[test]
fn test_string_index_bracket() {
    let output = run_and_capture(
        r#"
fn main() {
    let s = "abc";
    println("{}", s[0]);
    println("{}", s[2]);
}
"#,
    );
    assert_eq!(output, vec!["a\n", "c\n"]);
}

#[test]
fn test_fstring_basic() {
    let out =
        run_and_capture(r#"fn main() { let name = "World"; println("{}", f"Hello {name}!"); }"#);
    assert_eq!(out, vec!["Hello World!\n"]);
}

#[test]
fn test_fstring_expression() {
    let out = run_and_capture(r#"fn main() { let x = 10; println("{}", f"x + 5 = {x + 5}"); }"#);
    assert_eq!(out, vec!["x + 5 = 15\n"]);
}

#[test]
fn test_fstring_multiple_interpolations() {
    let out = run_and_capture(
        r#"fn main() { let a = 1; let b = 2; println("{}", f"{a} + {b} = {a + b}"); }"#,
    );
    assert_eq!(out, vec!["1 + 2 = 3\n"]);
}

#[test]
fn test_fstring_no_interpolation() {
    let out = run_and_capture(r#"fn main() { println("{}", f"plain string"); }"#);
    assert_eq!(out, vec!["plain string\n"]);
}

#[test]
fn test_fstring_escaped_braces() {
    let out = run_and_capture(r#"fn main() { println("{}", f"use {{braces}}"); }"#);
    assert_eq!(out, vec!["use {braces}\n"]);
}

#[test]
fn test_fstring_method_call() {
    let out =
        run_and_capture(r#"fn main() { let v = [1, 2, 3]; println("{}", f"len = {v.len()}"); }"#);
    assert_eq!(out, vec!["len = 3\n"]);
}

#[test]
fn test_fstring_nested_function() {
    let out = run_and_capture(
        r#"fn double(x: Int) -> Int { x * 2 } fn main() { println("{}", f"double(5) = {double(5)}"); }"#,
    );
    assert_eq!(out, vec!["double(5) = 10\n"]);
}

#[test]
fn test_fstring_in_variable() {
    let out =
        run_and_capture(r#"fn main() { let greeting = f"Hi {1 + 1}"; println("{}", greeting); }"#);
    assert_eq!(out, vec!["Hi 2\n"]);
}
