// === Feature: Error Handling — Option Adapters ===
// Option combinators that transform, chain, and compose optional values.
// Methods: map, and_then, or, or_else, filter.

// === map ===

#[test]
fn test_map_some() {
    val x = Some(5);
    val doubled = x.map(|v| v * 2);
    assert::eq(doubled.unwrap(), 10);
}

#[test]
fn test_map_none() {
    val x = None;
    val result = x.map(|v| v * 2);
    assert::true(result.is_none());
}

#[test]
fn test_map_type_change() {
    val x = Some(42);
    val s = x.map(|v| v.to_string());
    assert::eq(s.unwrap(), "42");
}

// === and_then ===

#[test]
fn test_and_then_some() {
    val x = Some(5);
    val result = x.and_then(|v| {
        if v > 0 {
            Some(v * 10)
        } else {
            None
        }
    });
    assert::eq(result.unwrap(), 50);
}

#[test]
fn test_and_then_returns_none() {
    val x = Some(-1);
    val result = x.and_then(|v| {
        if v > 0 {
            Some(v)
        } else {
            None
        }
    });
    assert::true(result.is_none());
}

#[test]
fn test_and_then_on_none() {
    val x = None;
    val result = x.and_then(|v| Some(v));
    assert::true(result.is_none());
}

// === chaining map + and_then ===

#[test]
fn test_chained_adapters() {
    val x = Some(3);
    val result = x.map(|v| v + 1)
                  .and_then(|v| Some(v * 2));
    assert::eq(result.unwrap(), 8);
}

// === or ===

#[test]
fn test_or_some() {
    val x = Some(1);
    val result = x.or(Some(99));
    assert::eq(result.unwrap(), 1);
}

#[test]
fn test_or_none() {
    val x = None;
    val result = x.or(Some(42));
    assert::eq(result.unwrap(), 42);
}

#[test]
fn test_or_none_none() {
    val x = None;
    val result = x.or(None);
    assert::true(result.is_none());
}

// === or_else ===

#[test]
fn test_or_else_some() {
    val x = Some(1);
    val result = x.or_else(|| Some(99));
    assert::eq(result.unwrap(), 1);
}

#[test]
fn test_or_else_none() {
    val x = None;
    val result = x.or_else(|| Some(42));
    assert::eq(result.unwrap(), 42);
}
