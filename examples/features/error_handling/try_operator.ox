// === Feature: Error Handling — Try Operator (?) ===
// The `?` operator propagates errors: on `Some(v)` / `Ok(v)` it unwraps `v`,
// on `None` / `Err(e)` it returns early from the enclosing function.

// === ? on Option ===

fn try_option_some() -> Option {
    let x = Some(42)?;
    Some(x + 1)
}

#[test]
fn test_try_option_some() {
    let r = try_option_some();
    assert!(r.is_some());
    assert_eq!(r.unwrap(), 43);
}

fn try_option_none() -> Option {
    let x = None?;
    Some(x)
}

#[test]
fn test_try_option_none() {
    let r = try_option_none();
    assert!(r.is_none());
}

// === ? on Result ===

fn try_result_ok() -> Result {
    let x: Result = Ok(10)?;
    Ok(x + 1)
}

#[test]
fn test_try_result_ok() {
    let r = try_result_ok();
    assert!(r.is_ok());
}

fn try_result_err() -> Result {
    let x: Result = Err("boom")?;
    Ok(x)
}

#[test]
fn test_try_result_err() {
    let r = try_result_err();
    assert!(r.is_err());
}

// === Chaining ? operators ===

fn chain_try() -> Result {
    let a: Result = Ok(10)?;
    let b: Result = Ok(20)?;
    let c: Result = Ok(30)?;
    Ok(a + b + c)
}

#[test]
fn test_chain_try() {
    let r = chain_try();
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), 60);
}

// === ? propagates first error ===

fn first_error_short_circuits() -> Result {
    let a: Result = Ok(10)?;
    let b: Result = Err("b fails")?;
    let c: Result = Ok(30)?;
    Ok(a + b + c)
}

#[test]
fn test_first_error_short_circuits() {
    let r = first_error_short_circuits();
    assert!(r.is_err());
}

// === ? in middle of chain ===

fn middle_error() -> Result {
    let a: Result = Ok(1)?;
    let b: Result = Ok(2)?;
    let c: Result = Err("c fails")?;
    Ok(a + b + c)
}

#[test]
fn test_middle_error() {
    let r = middle_error();
    assert!(r.is_err());
}

// === ? with mixed Option and Result ===

fn try_option_chain() -> Option {
    let x = Some(5)?;
    let y = Some(10)?;
    Some(x + y)
}

#[test]
fn test_try_option_chain() {
    let r = try_option_chain();
    assert_eq!(r.unwrap(), 15);
}
