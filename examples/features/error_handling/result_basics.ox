// === Feature: Error Handling — Result Basics ===
// Result represents success (`Ok`) or failure (`Err`).
// Methods: is_ok, is_err, unwrap, unwrap_err, expect, unwrap_or.

// === Construction ===

#[test]
fn test_ok_construction() {
    let r: Result = Ok(42);
    assert(r.is_ok());
    assert(!r.is_err());
}

#[test]
fn test_err_construction() {
    let r: Result = Err("oops");
    assert(r.is_err());
    assert(!r.is_ok());
}

// === is_ok / is_err ===

#[test]
fn test_is_ok() {
    assert(Ok(1).is_ok());
    assert(!Err("fail").is_ok());
}

#[test]
fn test_is_err() {
    assert(Err("fail").is_err());
    assert(!Ok(42).is_err());
}

// === unwrap ===

#[test]
fn test_unwrap_ok() {
    let r: Result = Ok(42);
    assert_eq(r.unwrap(), 42);
}

// NOTE: Err.unwrap() panics — expected behavior.

// === unwrap_err ===

#[test]
fn test_unwrap_err() {
    let r: Result = Err("error message");
    assert_eq(r.unwrap_err(), "error message");
}

// NOTE: Ok.unwrap_err() panics.

// === expect ===

#[test]
fn test_expect_ok() {
    let r: Result = Ok(100);
    assert_eq(r.expect("should succeed"), 100);
}

// === unwrap_or ===

#[test]
fn test_unwrap_or_ok() {
    let r: Result = Ok(10);
    assert_eq(r.unwrap_or(99), 10);
}

#[test]
fn test_unwrap_or_err() {
    let r: Result = Err("fail");
    assert_eq(r.unwrap_or(42), 42);
}

// === unwrap_or_else ===

#[test]
fn test_unwrap_or_else_ok() {
    let r: Result = Ok(10);
    let result = r.unwrap_or_else(|_| 99);
    assert_eq(result, 10);
}

#[test]
fn test_unwrap_or_else_err() {
    let r: Result = Err("fail");
    let result = r.unwrap_or_else(|e| {
        if e == "fail" {
            42
        } else {
            0
        }
    });
    assert_eq(result, 42);
}

// === ok / err (conversion to Option) ===

#[test]
fn test_ok_method() {
    let r: Result = Ok(42);
    let opt = r.ok();
    assert(opt.is_some());
    assert_eq(opt.unwrap(), 42);
}

#[test]
fn test_err_method() {
    let r: Result = Err("fail");
    let opt = r.err();
    assert(opt.is_some());
    assert_eq(opt.unwrap(), "fail");
}

// === Result as function return ===

fn parse_number(s: String) -> Result {
    let r = s.parse_int();
    if r.is_ok() {
        r
    } else {
        Err("not a number")
    }
}

#[test]
fn test_result_return() {
    let r = parse_number("42");
    assert(r.is_ok());

    let r2 = parse_number("notanumber");
    assert(r2.is_err());
}
