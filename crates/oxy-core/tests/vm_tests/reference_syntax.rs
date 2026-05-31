//! `&`/`&mut` rejection — Oxy is dynamic Rust (see CLAUDE.md).
//!
//! Shared helpers and imports come from the parent module (`use super::*`).

use super::*;

#[test]
fn test_function_call_with_string_param() {
    let output = run_and_capture(
        r#"
fn greet(name: String) {
    println("Hello, {}!", name);
}
fn main() {
    let name = "Oxy";
    greet(name);
}
"#,
    );
    assert_eq!(output, vec!["Hello, Oxy!\n"]);
}

#[test]
fn test_reject_amp_in_type_position() {
    let result = run_compiled(r#"fn greet(name: &str) { println("{}", name); } fn main() {}"#);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("references are not supported"),
        "expected fix-it error, got: {}",
        msg
    );
}

#[test]
fn test_reject_amp_self_in_method_receiver() {
    let result = run_compiled(
        r#"
struct Foo { n: int }
impl Foo {
    fn get(&self) -> int { self.n }
}
fn main() {}
"#,
    );
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("references are not supported"),
        "expected fix-it error, got: {}",
        msg
    );
}

#[test]
fn test_reject_amp_prefix_expression() {
    let result = run_compiled(r#"fn main() { let x = 5; let r = &x; println("{}", r); }"#);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("`&` prefix operator is not supported"),
        "expected fix-it error, got: {}",
        msg
    );
}

#[test]
fn test_self_method_works() {
    let output = run_and_capture(
        r#"
struct Counter { n: int }
impl Counter {
    fn bump(self) -> int {
        self.n = self.n + 1;
        self.n
    }
}
fn main() {
    let c = Counter { n: 5 };
    println("{}", c.bump());
}"#,
    );
    assert_eq!(output, vec!["6\n"]);
}

#[test]
fn test_param_reassign_works() {
    let output = run_and_capture(
        r#"
fn double_in_place(x: int) -> int {
    x = x * 2;
    x
}
fn main() {
    println("{}", double_in_place(21));
}"#,
    );
    assert_eq!(output, vec!["42\n"]);
}

#[test]
fn test_self_field_assign_works() {
    // self is always mutable — field assignment works without `mut self`.
    let output = run_and_capture(
        r#"
struct Counter { n: int }
impl Counter {
    fn try_bump(self) -> int {
        self.n = self.n + 1;
        self.n
    }
}
fn main() {
    let c = Counter { n: 5 };
    println("{}", c.try_bump());
}"#,
    );
    assert_eq!(output, vec!["6\n"]);
}

#[test]
fn test_immutable_let_field_assign_rejected() {
    // `let x = Struct { ... }; x.field = Y;` must error — same logic.
    let result = run_compiled(
        r#"
struct Point { x: int }
fn main() {
    let p = Point { x: 1 };
    p.x = 42;
}"#,
    );
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("immutable variable `p`") && msg.contains("let mut"),
        "expected fix-it error, got: {}",
        msg
    );
}

#[test]
fn test_mut_let_field_assign_works() {
    // `let mut p = ...; p.x = ...;` should be permitted (the binding is mut).
    let output = run_and_capture(
        r#"
struct Point { x: int }
fn main() {
    let mut p = Point { x: 1 };
    p.x = 42;
    println("{}", p.x);
}"#,
    );
    assert_eq!(output, vec!["42\n"]);
}
