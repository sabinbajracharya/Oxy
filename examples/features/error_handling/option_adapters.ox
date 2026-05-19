// === Feature: Error Handling — Option Adapters ===
// Option combinators that transform, chain, and compose optional values.
// Methods: map, and_then, or, or_else, filter.

// === map ===

#[test]
fn test_map_some() {
    let x = Some(5);
    let doubled = x.map(|v| v * 2);
    assert_eq!(doubled.unwrap(), 10);
}

#[test]
fn test_map_none() {
    let x = None;
    let result = x.map(|v| v * 2);
    assert!(result.is_none());
}

#[test]
fn test_map_type_change() {
    let x = Some(42);
    let s = x.map(|v| v.to_string());
    assert_eq!(s.unwrap(), "42");
}

// === and_then ===

#[test]
fn test_and_then_some() {
    let x = Some(5);
    let result = x.and_then(|v| {
        if v > 0 {
            Some(v * 10)
        } else {
            None
        }
    });
    assert_eq!(result.unwrap(), 50);
}

#[test]
fn test_and_then_returns_none() {
    let x = Some(-1);
    let result = x.and_then(|v| {
        if v > 0 {
            Some(v)
        } else {
            None
        }
    });
    assert!(result.is_none());
}

#[test]
fn test_and_then_on_none() {
    let x = None;
    let result = x.and_then(|v| Some(v));
    assert!(result.is_none());
}

// === chaining map + and_then ===

#[test]
fn test_chained_adapters() {
    let x = Some(3);
    let result = x.map(|v| v + 1)
                  .and_then(|v| Some(v * 2));
    assert_eq!(result.unwrap(), 8);
}

// === or ===

#[test]
fn test_or_some() {
    let x = Some(1);
    let result = x.or(Some(99));
    assert_eq!(result.unwrap(), 1);
}

#[test]
fn test_or_none() {
    let x = None;
    let result = x.or(Some(42));
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn test_or_none_none() {
    let x = None;
    let result = x.or(None);
    assert!(result.is_none());
}

// === or_else ===

#[test]
fn test_or_else_some() {
    let x = Some(1);
    let result = x.or_else(|| Some(99));
    assert_eq!(result.unwrap(), 1);
}

#[test]
fn test_or_else_none() {
    let x = None;
    let result = x.or_else(|| Some(42));
    assert_eq!(result.unwrap(), 42);
}
