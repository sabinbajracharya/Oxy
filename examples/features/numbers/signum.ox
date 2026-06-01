// Tests for the signum() method on numeric types.
// Returns -1, 0, or 1 mirroring the sign of the receiver.

#[test]
fn test_signum_positive_int() {
    val n: Int = 42;
    assert::eq(n.signum(), 1);
}

#[test]
fn test_signum_negative_int() {
    val n: Int = -17;
    assert::eq(n.signum(), -1);
}

#[test]
fn test_signum_zero_int() {
    val n: Int = 0;
    assert::eq(n.signum(), 0);
}

#[test]
fn test_signum_byte_positive() {
    val b: Byte = 5;
    assert::eq(b.signum(), 1);
}

#[test]
fn test_signum_byte_zero() {
    val b: Byte = 0;
    assert::eq(b.signum(), 0);
}

#[test]
fn test_signum_float_positive() {
    val x: Float = 3.14;
    assert::eq(x.signum(), 1.0);
}

#[test]
fn test_signum_float_negative() {
    val x: Float = -2.5;
    assert::eq(x.signum(), -1.0);
}

#[test]
fn test_signum_literal() {
    assert::eq((-7).signum(), -1);
    assert::eq(100.signum(), 1);
}
