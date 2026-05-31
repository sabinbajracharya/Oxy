// === Feature: `as` cast and `?` operator type checking ===

#[test]
fn test_cast_int_to_float_ok() {
    val n = 5;
    val f = n as Float;
    assert_eq(f, 5.0);
}

#[test]
fn test_cast_float_to_int_ok() {
    val f = 3.7;
    val n = f as Int;
    assert_eq(n, 3);
}

#[compile_error]
fn test_cast_to_unknown_type_rejected() {
    val n = 5;
    val _ = n as BogusType;
}
