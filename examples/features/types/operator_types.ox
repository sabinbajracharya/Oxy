// === Feature: operator operand type checking ===

#[test]
fn test_unary_neg_on_int_ok() {
    let n: i64 = -5;
    assert_eq!(n, -5);
}

#[test]
fn test_unary_not_on_bool_ok() {
    let b = !true;
    assert_eq!(b, false);
}

#[test]
fn test_binary_compare_compatible_types_ok() {
    let a: i64 = 5;
    let b: i32 = 5;
    assert_eq!(a == b as i64, true);
}

#[test]
fn test_binary_string_concat_ok() {
    let s = "foo".to_string() + "bar";
    assert_eq!(s, "foobar".to_string());
}

#[compile_error]
fn test_unary_neg_on_string_rejected() {
    let _ = -"hello";
}

#[compile_error]
fn test_unary_not_on_int_rejected() {
    let _ = !5;
}

#[compile_error]
fn test_binary_compare_int_with_string_rejected() {
    let _ = 1 == "x";
}

#[compile_error]
fn test_binary_compare_bool_with_int_rejected() {
    let _ = true == 1;
}
