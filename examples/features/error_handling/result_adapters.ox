// === Feature: Error Handling — Result Adapters ===
// Result combinators that transform and chain fallible operations.
// Methods: map, map_err, and_then, or_else.

// === map ===

#[test]
fn test_map_ok() {
    let r: Result = Ok(5);
    let doubled = r.map(|v| v * 2);
    assert!(doubled.is_ok());
    assert_eq!(doubled.unwrap(), 10);
}

#[test]
fn test_map_err() {
    let r: Result = Err("fail");
    let result = r.map(|v| v * 2);
    assert!(result.is_err());
}

// === map_err ===

#[test]
fn test_map_err_on_err() {
    let r: Result = Err("fail");
    let result = r.map_err(|e| e.to_uppercase());
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "FAIL");
}

#[test]
fn test_map_err_on_ok() {
    let r: Result = Ok(42);
    let result = r.map_err(|e| e.to_string());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

// === and_then ===

#[test]
fn test_and_then_ok() {
    let r: Result = Ok(5);
    let result = r.and_then(|v| Ok(v * 10));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 50);
}

#[test]
fn test_and_then_chaining() {
    let r: Result = Ok(3);
    let result = r.and_then(|v| Ok(v + 1))
                  .and_then(|v| Ok(v * 2));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 8);
}

#[test]
fn test_and_then_on_err() {
    let r: Result = Err("fail");
    let result = r.and_then(|v| Ok(v));
    assert!(result.is_err());
}

// === or_else ===

#[test]
fn test_or_else_ok() {
    let r: Result = Ok(42);
    let result = r.or_else(|_| Ok(99));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn test_or_else_err() {
    let r: Result = Err("fail");
    let result = r.or_else(|e| {
        if e == "fail" {
            Ok(42)
        } else {
            Err("other error")
        }
    });
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

// === Chaining map_err + and_then ===

#[test]
fn test_chain_map_err_and_then() {
    let r: Result = Err("fail");
    let result = r.map_err(|e| e.to_uppercase())
                  .or_else(|_| Ok(42));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}
