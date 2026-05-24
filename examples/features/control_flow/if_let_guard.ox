// === Feature: if let with && guard ===
// `if let Pat = expr && guard` is sugar for matching the pattern AND
// requiring an extra boolean condition. On pattern miss OR guard false,
// execution falls through to the else branch.

fn main() {}

// === Guard passes ===

fn positive_only(opt: Option<int>) -> int {
    if let Some(v) = opt && v > 0 {
        v
    } else {
        -1
    }
}

#[test]
fn test_iflet_guard_some_positive() {
    assert_eq!(positive_only(Some(10)), 10);
}

#[test]
fn test_iflet_guard_some_negative_fails_guard() {
    assert_eq!(positive_only(Some(-5)), -1);
}

#[test]
fn test_iflet_guard_none_fails_pattern() {
    assert_eq!(positive_only(None), -1);
}

// === Guard without else ===

fn extract_positive(opt: Option<int>) -> int {
    let mut result = 0;
    if let Some(v) = opt && v > 0 {
        result = v;
    }
    result
}

#[test]
fn test_iflet_guard_no_else_matches() {
    assert_eq!(extract_positive(Some(7)), 7);
}

#[test]
fn test_iflet_guard_no_else_guard_fails() {
    assert_eq!(extract_positive(Some(-3)), 0);
}

#[test]
fn test_iflet_guard_no_else_none() {
    assert_eq!(extract_positive(None), 0);
}

// === Result with guard ===

fn ok_in_range(r: Result<int, String>) -> bool {
    if let Ok(n) = r && n >= 0 && n < 100 {
        true
    } else {
        false
    }
}

#[test]
fn test_iflet_result_guard_in_range() {
    assert_eq!(ok_in_range(Ok(50)), true);
}

#[test]
fn test_iflet_result_guard_out_of_range() {
    assert_eq!(ok_in_range(Ok(200)), false);
}

#[test]
fn test_iflet_result_guard_negative() {
    assert_eq!(ok_in_range(Ok(-1)), false);
}

#[test]
fn test_iflet_result_guard_err() {
    assert_eq!(ok_in_range(Err("oops".to_string())), false);
}

// === Guard using bound variable in string method ===

fn long_name(opt: Option<String>) -> bool {
    if let Some(s) = opt && s.len() > 3 {
        true
    } else {
        false
    }
}

#[test]
fn test_iflet_guard_string_long() {
    assert_eq!(long_name(Some("hello".to_string())), true);
}

#[test]
fn test_iflet_guard_string_short() {
    assert_eq!(long_name(Some("hi".to_string())), false);
}

#[test]
fn test_iflet_guard_string_none() {
    assert_eq!(long_name(None), false);
}

// === Guard with else if chain ===

fn classify(opt: Option<int>) -> String {
    if let Some(v) = opt && v > 100 {
        "big".to_string()
    } else if let Some(v) = opt && v > 0 {
        "small".to_string()
    } else {
        "other".to_string()
    }
}

#[test]
fn test_iflet_guard_else_if_big() {
    assert_eq!(classify(Some(200)), "big");
}

#[test]
fn test_iflet_guard_else_if_small() {
    assert_eq!(classify(Some(5)), "small");
}

#[test]
fn test_iflet_guard_else_if_negative() {
    assert_eq!(classify(Some(-1)), "other");
}

#[test]
fn test_iflet_guard_else_if_none() {
    assert_eq!(classify(None), "other");
}
