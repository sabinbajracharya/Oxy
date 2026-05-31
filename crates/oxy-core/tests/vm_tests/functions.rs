//! Function definition, calls, returns, arity checking.
//!
//! Shared helpers and imports come from the parent module (`use super::*`).

use super::*;

#[test]
fn test_function_call() {
    let output = run_and_capture(
        r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let result = add(3, 4);
    println!("{}", result);
}
"#,
    );
    assert_eq!(output, vec!["7\n"]);
}

#[test]
fn test_function_return() {
    let output = run_and_capture(
        r#"
fn early(x: int) -> int {
    if x > 0 {
        return x;
    }
    return 0;
}

fn main() {
    println!("{}", early(5));
    println!("{}", early(-1));
}
"#,
    );
    assert_eq!(output, vec!["5\n", "0\n"]);
}

#[test]
fn test_tail_expression() {
    let output = run_and_capture(
        r#"
fn double(x: int) -> int {
    x * 2
}

fn main() {
    println!("{}", double(21));
}
"#,
    );
    assert_eq!(output, vec!["42\n"]);
}

#[test]
fn test_wrong_arg_count() {
    let result = run_compiled(
        r#"
fn foo(a: int) -> int { a }
fn main() { foo(1, 2); }
"#,
    );
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("expected 1, got 2"));
}

#[test]
fn test_recursive_function() {
    let output = run_and_capture(
        r#"
fn factorial(n: int) -> int {
    if n <= 1 {
        return 1;
    }
    n * factorial(n - 1)
}

fn main() {
    println!("{}", factorial(5));
}
"#,
    );
    assert_eq!(output, vec!["120\n"]);
}
