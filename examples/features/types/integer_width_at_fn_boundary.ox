// === Feature: `Byte` is enforced at function boundaries ===
// `Int` and `Byte` are Oxy's two integer types. A function declared
// `-> Byte` must wrap its return value to Byte range (0..=255). A
// `Byte` parameter must wrap incoming values to Byte range too. Without
// these boundary coercions, the declared types would be silently
// erased — `fn f(n: Byte) -> Byte` called with `300` would carry
// `Int(300)` through the body and return some out-of-range `Int`.

fn returns_byte(n: Int) -> Byte {
    n
}

fn takes_byte(n: Byte) -> Int {
    n as Int
}

fn fib_byte(n: Byte) -> Byte {
    if n <= 1 { return n; }
    fib_byte(n - 1) + fib_byte(n - 2)
}

#[test]
fn test_return_wraps_to_byte() {
    assert::eq(returns_byte(300) as Int, 44);
    assert::eq(returns_byte(256) as Int, 0);
    assert::eq(returns_byte(-1) as Int, 255);
}

#[test]
fn test_param_truncated_at_entry() {
    // 300 passed to a Byte param becomes 44 inside the fn.
    assert::eq(takes_byte(300), 44);
}

#[test]
fn test_recursive_byte_fn_keeps_width() {
    // fib(13) = 233 — still in Byte range.
    assert::eq(fib_byte(13) as Int, 233);
}
