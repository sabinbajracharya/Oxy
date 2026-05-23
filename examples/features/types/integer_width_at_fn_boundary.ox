// === Feature: declared integer widths wrap at function boundaries ===
// Before this fix, every integer in the VM was effectively `i64`. A
// function declared `-> u8` returning a large value silently leaked
// the inner `i64` value, and `fn f(n: u8)` called with `i64(300)`
// would carry 300 inside the body instead of `300 as u8 = 44`. Now
// the compiler emits `CastInt(declared_width)` at each entry param
// and before each `Return`.

fn ret_u8(n: i64) -> u8 {
    n
}

fn ret_u32(n: i64) -> u32 {
    n
}

fn ret_u64_neg(n: i64) -> u64 {
    n
}

fn ret_i8(n: i64) -> i8 {
    n
}

fn param_u8(n: u8) -> i64 {
    n as i64
}

fn fib_u8(n: u8) -> u8 {
    if n <= 1 { return n; }
    fib_u8(n - 1) + fib_u8(n - 2)
}

#[test]
fn test_return_wraps_to_u8() {
    assert_eq!(ret_u8(300) as i64, 44);
    assert_eq!(ret_u8(256) as i64, 0);
    assert_eq!(ret_u8(-1) as i64, 255);
}

#[test]
fn test_return_wraps_to_u32() {
    assert_eq!(ret_u32(-1) as i64, 4294967295);
}

#[test]
fn test_return_u64_reinterprets_negative_as_unsigned() {
    // -1 as u64 is u64::MAX. The point: it should not display as -1.
    let v = ret_u64_neg(-1);
    assert_eq!(v as i64, -1); // bit pattern preserved
    assert!(v > 0);            // but the u64 is positive
}

#[test]
fn test_return_wraps_to_i8() {
    assert_eq!(ret_i8(200) as i64, -56);
}

#[test]
fn test_param_truncated_at_entry() {
    // 300 passed to a u8 param becomes 44 inside the fn.
    assert_eq!(param_u8(300), 44);
}

#[test]
fn test_recursive_u8_fn_keeps_width() {
    // fib_u8(13) = 233 — still in u8 range; param u8 keeps widths sane.
    assert_eq!(fib_u8(13) as i64, 233);
}
