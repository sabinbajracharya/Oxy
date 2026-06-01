// === Feature: Error Handling — Try Operator (?) ===
// The `?` operator propagates errors: on `Some(v)` / `Ok(v)` it unwraps `v`,
// on `None` / `Err(e)` it returns early from the enclosing function.

// === ? on Option ===

fn try_option_some() -> Option {
    val x = Some(42)?;
    Some(x + 1)
}

#[test]
fn test_try_option_some() {
    val r = try_option_some();
    assert::true(r.is_some());
    assert::eq(r.unwrap(), 43);
}

fn try_option_none() -> Option {
    val x = None?;
    Some(x)
}

#[test]
fn test_try_option_none() {
    val r = try_option_none();
    assert::true(r.is_none());
}

// === ? on Result ===

fn try_result_ok() -> Result {
    val x = Ok(10)?;
    Ok(x + 1)
}

#[test]
fn test_try_result_ok() {
    val r = try_result_ok();
    assert::true(r.is_ok());
}

fn try_result_err() -> Result {
    val x = Err("boom")?;
    Ok(x)
}

#[test]
fn test_try_result_err() {
    val r = try_result_err();
    assert::true(r.is_err());
}

// === Chaining ? operators ===

fn chain_try() -> Result {
    val a = Ok(10)?;
    val b = Ok(20)?;
    val c = Ok(30)?;
    Ok(a + b + c)
}

#[test]
fn test_chain_try() {
    val r = chain_try();
    assert::true(r.is_ok());
    assert::eq(r.unwrap(), 60);
}

// === ? propagates first error ===

fn first_error_short_circuits() -> Result {
    val a = Ok(10)?;
    val b = Err("b fails")?;
    val c = Ok(30)?;
    Ok(a + b + c)
}

#[test]
fn test_first_error_short_circuits() {
    val r = first_error_short_circuits();
    assert::true(r.is_err());
}

// === ? in middle of chain ===

fn middle_error() -> Result {
    val a = Ok(1)?;
    val b = Ok(2)?;
    val c = Err("c fails")?;
    Ok(a + b + c)
}

#[test]
fn test_middle_error() {
    val r = middle_error();
    assert::true(r.is_err());
}

// === ? with mixed Option and Result ===

fn try_option_chain() -> Option {
    val x = Some(5)?;
    val y = Some(10)?;
    Some(x + y)
}

#[test]
fn test_try_option_chain() {
    val r = try_option_chain();
    assert::eq(r.unwrap(), 15);
}
