// === Feature: `byte` is enforced at function boundaries ===
// `int` and `byte` are Oxy's two integer types. A function declared
// `-> byte` must wrap its return value to byte range (0..=255). A
// `byte` parameter must wrap incoming values to byte range too. Without
// these boundary coercions, the declared types would be silently
// erased — `fn f(n: byte) -> byte` called with `300` would carry
// `int(300)` through the body and return some out-of-range `int`.

fn returns_byte(n: int) -> byte {
    n
}

fn takes_byte(n: byte) -> int {
    n as int
}

fn fib_byte(n: byte) -> byte {
    if n <= 1 { return n; }
    fib_byte(n - 1) + fib_byte(n - 2)
}

#[test]
fn test_return_wraps_to_byte() {
    assert_eq!(returns_byte(300) as int, 44);
    assert_eq!(returns_byte(256) as int, 0);
    assert_eq!(returns_byte(-1) as int, 255);
}

#[test]
fn test_param_truncated_at_entry() {
    // 300 passed to a byte param becomes 44 inside the fn.
    assert_eq!(takes_byte(300), 44);
}

#[test]
fn test_recursive_byte_fn_keeps_width() {
    // fib(13) = 233 — still in byte range.
    assert_eq!(fib_byte(13) as int, 233);
}
