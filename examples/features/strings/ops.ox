// === Feature: Strings — Operators ===
// String operators: concatenation with `+`, comparison with `==`/`!=`/
// `<`/`>`/`<=`/`>=`, and iteration with `for` loops.

// === Concatenation: String + String ===

#[test]
fn test_concat_two_strings() {
    val s = "hello " + "world";
    assert::eq(s, "hello world");
}

#[test]
fn test_concat_empty_strings() {
    assert::eq("" + "", "");
    assert::eq("hello" + "", "hello");
    assert::eq("" + "world", "world");
}

#[test]
fn test_concat_multiple() {
    val s = "a" + "b" + "c";
    assert::eq(s, "abc");
}

// === Concatenation: String + other types ===

#[test]
fn test_concat_string_and_int() {
    val s = "value: " + "42";
    assert::true(s.contains("42"));
}

// === Equality Comparison ===

#[test]
fn test_eq_equal() {
    assert::true("hello" == "hello");
    assert::true("" == "");
}

#[test]
fn test_eq_not_equal() {
    assert::true("hello" != "world");
    assert::true("abc" != "ABC");
    assert::true("" != " ");
}

#[test]
fn test_eq_double_negation() {
    assert::true(!("hello" == "world"));
}

// === Lexicographic Ordering ===

#[test]
fn test_less_than() {
    assert::true("a" < "b");
    assert::true("abc" < "abd");
    assert::true("" < "a");
    assert::true(!("b" < "a"));
    assert::true(!("abc" < "abc"));
}

#[test]
fn test_greater_than() {
    assert::true("b" > "a");
    assert::true("hello" > "hell");
    assert::true(!("a" > "b"));
}

#[test]
fn test_less_equal() {
    assert::true("a" <= "b");
    assert::true("a" <= "a");
    assert::true("" <= "");
    assert::true(!("b" <= "a"));
}

#[test]
fn test_greater_equal() {
    assert::true("b" >= "a");
    assert::true("b" >= "b");
    assert::true(!("a" >= "b"));
}

// === Case Sensitivity in Comparison ===

#[test]
fn test_case_sensitive_ordering() {
    assert::true("A" < "a");
    assert::true("ABC" < "abc");
    assert::true("hello" != "HELLO");
}

// === For Loop Iteration ===

#[test]
fn test_for_loop_chars() {
    var count = 0;
    for c in "hello" {
        count = count + 1;
    }
    assert::eq(count, 5);
}

#[test]
fn test_for_loop_empty() {
    var count = 0;
    for c in "" {
        count = count + 1;
    }
    assert::eq(count, 0);
}

// === String in Let Binding ===

#[test]
fn test_let_string() {
    var s = "hello";
    s = "world";
    assert::eq(s, "world");
}
