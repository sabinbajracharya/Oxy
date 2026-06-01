// === Feature: operator operand type checking ===

#[test]
fn test_unary_neg_on_int_ok() {
    val n: Int = -5;
    assert::eq(n, -5);
}

#[test]
fn test_unary_not_on_bool_ok() {
    val b = !true;
    assert::eq(b, false);
}

#[test]
fn test_binary_compare_compatible_types_ok() {
    val a: Int = 5;
    val b: Int = 5;
    assert::eq(a == b as Int, true);
}

#[test]
fn test_binary_string_concat_ok() {
    val s = "foo".to_string() + "bar";
    assert::eq(s, "foobar".to_string());
}

#[compile_error]
fn test_unary_neg_on_string_rejected() {
    val _ = -"hello";
}

#[compile_error]
fn test_unary_not_on_int_rejected() {
    val _ = !5;
}

#[compile_error]
fn test_binary_compare_int_with_string_rejected() {
    val _ = 1 == "x";
}

#[compile_error]
fn test_binary_compare_bool_with_int_rejected() {
    val _ = true == 1;
}
