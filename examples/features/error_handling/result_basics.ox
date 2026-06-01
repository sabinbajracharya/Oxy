// === Feature: Error Handling — Result Basics ===
// Result represents success (`Ok`) or failure (`Err`).
// Methods: is_ok, is_err, unwrap, unwrap_err, expect, unwrap_or.

// === Construction ===

#[test]
fn test_ok_construction() {
    val r: Result = Ok(42);
    assert::true(r.is_ok());
    assert::true(!r.is_err());
}

#[test]
fn test_err_construction() {
    val r: Result = Err("oops");
    assert::true(r.is_err());
    assert::true(!r.is_ok());
}

// === is_ok / is_err ===

#[test]
fn test_is_ok() {
    assert::true(Ok(1).is_ok());
    assert::true(!Err("fail").is_ok());
}

#[test]
fn test_is_err() {
    assert::true(Err("fail").is_err());
    assert::true(!Ok(42).is_err());
}

// === unwrap ===

#[test]
fn test_unwrap_ok() {
    val r: Result = Ok(42);
    assert::eq(r.unwrap(), 42);
}

// NOTE: Err.unwrap() panics — expected behavior.

// === unwrap_err ===

#[test]
fn test_unwrap_err() {
    val r: Result = Err("error message");
    assert::eq(r.unwrap_err(), "error message");
}

// NOTE: Ok.unwrap_err() panics.

// === expect ===

#[test]
fn test_expect_ok() {
    val r: Result = Ok(100);
    assert::eq(r.expect("should succeed"), 100);
}

// === unwrap_or ===

#[test]
fn test_unwrap_or_ok() {
    val r: Result = Ok(10);
    assert::eq(r.unwrap_or(99), 10);
}

#[test]
fn test_unwrap_or_err() {
    val r: Result = Err("fail");
    assert::eq(r.unwrap_or(42), 42);
}

// === unwrap_or_else ===

#[test]
fn test_unwrap_or_else_ok() {
    val r: Result = Ok(10);
    val result = r.unwrap_or_else(|_| 99);
    assert::eq(result, 10);
}

#[test]
fn test_unwrap_or_else_err() {
    val r: Result = Err("fail");
    val result = r.unwrap_or_else(|e| {
        if e == "fail" {
            42
        } else {
            0
        }
    });
    assert::eq(result, 42);
}

// === ok / err (conversion to Option) ===

#[test]
fn test_ok_method() {
    val r: Result = Ok(42);
    val opt = r.ok();
    assert::true(opt.is_some());
    assert::eq(opt.unwrap(), 42);
}

#[test]
fn test_err_method() {
    val r: Result = Err("fail");
    val opt = r.err();
    assert::true(opt.is_some());
    assert::eq(opt.unwrap(), "fail");
}

// === Result as function return ===

fn parse_number(s: String) -> Result {
    val r = s.parse_int();
    if r.is_ok() {
        r
    } else {
        Err("not a number")
    }
}

#[test]
fn test_result_return() {
    val r = parse_number("42");
    assert::true(r.is_ok());

    val r2 = parse_number("notanumber");
    assert::true(r2.is_err());
}
