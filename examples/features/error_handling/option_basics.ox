// === Feature: Error Handling — Option Basics ===
// Option represents a value that may or may not be present.
// `Some(x)` wraps a value, `None` is the absence.
// Methods: is_some, is_none, unwrap, expect, unwrap_or.

// === Construction ===

#[test]
fn test_some_construction() {
    val x = Some(42);
    assert(x.is_some());
    assert(!x.is_none());
}

#[test]
fn test_none_construction() {
    val x = None;
    assert(!x.is_some());
    assert(x.is_none());
}

// === is_some / is_none ===

#[test]
fn test_is_some() {
    assert(Some(1).is_some());
    assert(!None.is_some());
}

#[test]
fn test_is_none() {
    assert(None.is_none());
    assert(!Some("hello").is_none());
}

// === unwrap ===

#[test]
fn test_unwrap_some() {
    val x = Some(42);
    assert_eq(x.unwrap(), 42);
}

#[test]
fn test_unwrap_some_string() {
    val x = Some("hello");
    assert_eq(x.unwrap(), "hello");
}

// NOTE: None.unwrap() panics at runtime — this is expected behavior.
// A test for this would need #[should_panic] support.

// === expect ===

#[test]
fn test_expect_some() {
    val x = Some(100);
    assert_eq(x.expect("should have value"), 100);
}

// NOTE: None.expect("msg") panics with the message.

// === unwrap_or ===

#[test]
fn test_unwrap_or_some() {
    val x = Some(10);
    assert_eq(x.unwrap_or(99), 10);
}

#[test]
fn test_unwrap_or_none() {
    val x = None;
    assert_eq(x.unwrap_or(42), 42);
}

#[test]
fn test_unwrap_or_string() {
    val x = None;
    assert_eq(x.unwrap_or("default"), "default");
}

// === unwrap_or_else ===

#[test]
fn test_unwrap_or_else_some() {
    val x = Some(10);
    val result = x.unwrap_or_else(|| 99);
    assert_eq(result, 10);
}

#[test]
fn test_unwrap_or_else_none() {
    val x = None;
    val result = x.unwrap_or_else(|| 42);
    assert_eq(result, 42);
}

// === Option in if expressions ===

#[test]
fn test_option_in_condition() {
    val x = Some(42);
    var found = false;
    if x.is_some() {
        found = true;
    }
    assert(found);
}

// === Option as function return ===

fn safe_divide(a: Int, b: Int) -> Option<Int> {
    if b == 0 {
        None
    } else {
        Some(a / b)
    }
}

#[test]
fn test_option_return() {
    val r = safe_divide(10, 2);
    assert_eq(r.unwrap(), 5);

    val r2 = safe_divide(10, 0);
    assert(r2.is_none());
}
