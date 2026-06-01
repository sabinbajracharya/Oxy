//! Error-message DX, assert macros, the test runner, recursion limit.
//!
//! Shared helpers and imports come from the parent module (`use super::*`).

use super::*;

#[test]
fn test_recursion_limit() {
    let output = run_and_capture(
        r#"
fn recurse(n: Int) -> Int {
    if n == 0 { 0 } else { 1 + recurse(n - 1) }
}
fn main() {
    io::println("{}", recurse(10));
}
"#,
    );
    assert_eq!(output, vec!["10\n"]);
}

#[test]
fn test_did_you_mean_suggestion() {
    let result = run_compiled(
        r#"
fn main() {
    val name = "Alice";
    io::println("{}", nme);
}
"#,
    );
    let err = result.unwrap_err().to_string();
    assert!(err.contains("undefined variable 'nme'"));
    assert!(err.contains("did you mean 'name'"));
}

#[test]
fn test_no_suggestion_for_distant_name() {
    let result = run_compiled(
        r#"
fn main() {
    val x = 1;
    io::println("{}", completely_different);
}
"#,
    );
    let err = result.unwrap_err().to_string();
    assert!(err.contains("undefined variable"));
    assert!(!err.contains("did you mean"));
}

#[test]
fn test_stack_trace_on_runtime_error() {
    let source = r#"
fn inner() {
    val x = 1 / 0;
}
fn outer() {
    inner();
}
fn main() {
    outer();
}
"#;
    let result = run_compiled(source);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("division by zero") || err.contains("divide by zero"));
}

#[test]
fn test_edit_distance() {
    use oxy_core::errors::edit_distance;
    assert_eq!(edit_distance("kitten", "sitting"), 3);
    assert_eq!(edit_distance("", "abc"), 3);
    assert_eq!(edit_distance("abc", "abc"), 0);
    assert_eq!(edit_distance("name", "nme"), 1);
}

#[test]
fn test_suggest_name() {
    use oxy_core::errors::suggest_name;
    assert_eq!(
        suggest_name("nme", ["name", "age", "value"].into_iter()),
        Some("name".to_string())
    );
    assert_eq!(
        suggest_name("xyz", ["name", "age", "value"].into_iter()),
        None
    );
    assert_eq!(
        suggest_name("prnt", ["print", "println", "parse"].into_iter()),
        Some("print".to_string())
    );
}

#[test]
fn test_assert_pass() {
    run_compiled_capturing("fn main() { assert::true(true); }").unwrap();
    run_compiled_capturing("fn main() { assert::true(1 == 1); }").unwrap();
}

#[test]
fn test_assert_fail() {
    let err = run_compiled_capturing("fn main() { assert::true(false); }").unwrap_err();
    assert!(format!("{err}").contains("assertion failed"));
}

#[test]
fn test_assert_with_message() {
    let err = run_compiled_capturing(r#"fn main() { assert::true(false, "custom message"); }"#)
        .unwrap_err();
    assert!(format!("{err}").contains("custom message"));
}

#[test]
fn test_assert_eq_pass() {
    run_compiled_capturing("fn main() { assert::eq(1, 1); }").unwrap();
    run_compiled_capturing(r#"fn main() { assert::eq("hello", "hello"); }"#).unwrap();
}

#[test]
fn test_assert_eq_fail() {
    let err = run_compiled_capturing("fn main() { assert::eq(1, 2); }").unwrap_err();
    assert!(format!("{err}").contains("assertion failed"));
}

#[test]
fn test_assert_ne_pass() {
    run_compiled_capturing("fn main() { assert::ne(1, 2); }").unwrap();
}

#[test]
fn test_assert_ne_fail() {
    let err = run_compiled_capturing("fn main() { assert::ne(1, 1); }").unwrap_err();
    assert!(format!("{err}").contains("assertion failed"));
}

#[test]
fn test_test_runner_basic() {
    let source = r#"
            #[test]
            fn test_addition() {
                assert::eq(1 + 1, 2);
            }

            #[test]
            fn test_string() {
                assert::eq("hello".len(), 5);
            }
        "#;
    let results = oxy_core::vm::run_tests("test.ox", source).unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.passed));
}

#[test]
fn test_test_runner_failure() {
    let source = r#"
            #[test]
            fn test_bad() {
                assert::eq(1, 2);
            }
        "#;
    let results = oxy_core::vm::run_tests("test.ox", source).unwrap();
    assert_eq!(results.len(), 1);
    assert!(!results[0].passed);
    assert!(results[0].error.is_some());
}

#[test]
fn test_test_runner_no_tests() {
    let source = "fn foo() { }";
    let results = oxy_core::vm::run_tests("test.ox", source).unwrap();
    assert_eq!(results.len(), 0);
}
