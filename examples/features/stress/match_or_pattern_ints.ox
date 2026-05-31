// === STRESS: match with or-pattern on integer literals + ranges + guards ===

fn describe(n: int) -> String {
    match n {
        0 => "zero".to_string(),
        1 | 2 => "one or two".to_string(),
        3..=9 => "small digit".to_string(),
        n if n > 0 => f"positive {n}",
        _ => "negative".to_string(),
    }
}

#[test]
fn test_zero() { assert_eq(describe(0), "zero"); }
#[test]
fn test_one() { assert_eq(describe(1), "one or two"); }
#[test]
fn test_two() { assert_eq(describe(2), "one or two"); }
#[test]
fn test_small_digit() { assert_eq(describe(5), "small digit"); }
#[test]
fn test_big_positive() { assert_eq(describe(42), "positive 42"); }
#[test]
fn test_negative() { assert_eq(describe(-3), "negative"); }

// The guard-fail-then-prelude-Pop bug only manifested when the surrounding
// caller had values on its stack that the spurious Pop could eat. Exercise
// it explicitly with a multi-arg println-style call wrapping the match.
fn wrap(n: int) -> String {
    format("{}: {}", n, describe(n))
}

#[test]
fn test_multi_arg_format_around_match_with_guard_fail() {
    assert_eq(wrap(0), "0: zero");
    assert_eq(wrap(2), "2: one or two");
    assert_eq(wrap(5), "5: small digit");
    assert_eq(wrap(42), "42: positive 42");
    assert_eq(wrap(-3), "-3: negative");
}
