// Tests for the signum() method on numeric types.
// Returns -1, 0, or 1 mirroring the sign of the receiver.

#[test]
fn test_signum_positive_int() {
    let n: Int = 42;
    assert_eq(n.signum(), 1);
}

#[test]
fn test_signum_negative_int() {
    let n: Int = -17;
    assert_eq(n.signum(), -1);
}

#[test]
fn test_signum_zero_int() {
    let n: Int = 0;
    assert_eq(n.signum(), 0);
}

#[test]
fn test_signum_byte_positive() {
    let b: Byte = 5;
    assert_eq(b.signum(), 1);
}

#[test]
fn test_signum_byte_zero() {
    let b: Byte = 0;
    assert_eq(b.signum(), 0);
}

#[test]
fn test_signum_float_positive() {
    let x: Float = 3.14;
    assert_eq(x.signum(), 1.0);
}

#[test]
fn test_signum_float_negative() {
    let x: Float = -2.5;
    assert_eq(x.signum(), -1.0);
}

#[test]
fn test_signum_literal() {
    assert_eq((-7).signum(), -1);
    assert_eq(100.signum(), 1);
}
