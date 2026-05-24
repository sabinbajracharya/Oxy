// Tests for the signum() method on numeric types.
// Returns -1, 0, or 1 mirroring the sign of the receiver.

#[test]
fn test_signum_positive_int() {
    let n: int = 42;
    assert_eq!(n.signum(), 1);
}

#[test]
fn test_signum_negative_int() {
    let n: int = -17;
    assert_eq!(n.signum(), -1);
}

#[test]
fn test_signum_zero_int() {
    let n: int = 0;
    assert_eq!(n.signum(), 0);
}

#[test]
fn test_signum_byte_positive() {
    let b: byte = 5;
    assert_eq!(b.signum(), 1);
}

#[test]
fn test_signum_byte_zero() {
    let b: byte = 0;
    assert_eq!(b.signum(), 0);
}

#[test]
fn test_signum_float_positive() {
    let x: float = 3.14;
    assert_eq!(x.signum(), 1.0);
}

#[test]
fn test_signum_float_negative() {
    let x: float = -2.5;
    assert_eq!(x.signum(), -1.0);
}

#[test]
fn test_signum_literal() {
    assert_eq!((-7).signum(), -1);
    assert_eq!(100.signum(), 1);
}
