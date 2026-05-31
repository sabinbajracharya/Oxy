// === Feature: `as` cast and `?` operator type checking ===

#[test]
fn test_cast_int_to_float_ok() {
    let n = 5;
    let f = n as float;
    assert_eq(f, 5.0);
}

#[test]
fn test_cast_float_to_int_ok() {
    let f = 3.7;
    let n = f as int;
    assert_eq(n, 3);
}

#[compile_error]
fn test_cast_to_unknown_type_rejected() {
    let n = 5;
    let _ = n as BogusType;
}
