// === Feature: Strings — Operators ===
// String operators: concatenation with `+`, comparison with `==`/`!=`/
// `<`/`>`/`<=`/`>=`, and iteration with `for` loops.

// === Concatenation: String + String ===

#[test]
fn test_concat_two_strings() {
    let s = "hello " + "world";
    assert_eq!(s, "hello world");
}

#[test]
fn test_concat_empty_strings() {
    assert_eq!("" + "", "");
    assert_eq!("hello" + "", "hello");
    assert_eq!("" + "world", "world");
}

#[test]
fn test_concat_multiple() {
    let s = "a" + "b" + "c";
    assert_eq!(s, "abc");
}

// === Concatenation: String + other types ===

#[test]
fn test_concat_string_and_int() {
    let s = "value: " + "42";
    assert!(s.contains("42"));
}

// === Equality Comparison ===

#[test]
fn test_eq_equal() {
    assert!("hello" == "hello");
    assert!("" == "");
}

#[test]
fn test_eq_not_equal() {
    assert!("hello" != "world");
    assert!("abc" != "ABC");
    assert!("" != " ");
}

#[test]
fn test_eq_double_negation() {
    assert!(!("hello" == "world"));
}

// === Lexicographic Ordering ===

#[test]
fn test_less_than() {
    assert!("a" < "b");
    assert!("abc" < "abd");
    assert!("" < "a");
    assert!(!("b" < "a"));
    assert!(!("abc" < "abc"));
}

#[test]
fn test_greater_than() {
    assert!("b" > "a");
    assert!("hello" > "hell");
    assert!(!("a" > "b"));
}

#[test]
fn test_less_equal() {
    assert!("a" <= "b");
    assert!("a" <= "a");
    assert!("" <= "");
    assert!(!("b" <= "a"));
}

#[test]
fn test_greater_equal() {
    assert!("b" >= "a");
    assert!("b" >= "b");
    assert!(!("a" >= "b"));
}

// === Case Sensitivity in Comparison ===

#[test]
fn test_case_sensitive_ordering() {
    assert!("A" < "a");
    assert!("ABC" < "abc");
    assert!("hello" != "HELLO");
}

// === For Loop Iteration ===

#[test]
fn test_for_loop_chars() {
    let mut count = 0;
    for c in "hello" {
        count = count + 1;
    }
    assert_eq!(count, 5);
}

#[test]
fn test_for_loop_empty() {
    let mut count = 0;
    for c in "" {
        count = count + 1;
    }
    assert_eq!(count, 0);
}

// === String in Let Binding ===

#[test]
fn test_let_string() {
    let mut s = "hello";
    s = "world";
    assert_eq!(s, "world");
}
