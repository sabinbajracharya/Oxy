// === Feature: reject unknown field/method access on builtin types ===

#[test]
fn test_vec_known_method_ok() {
    val v: List<Int> = [1, 2, 3];
    assert_eq(v.len(), 3);
}

#[test]
fn test_string_known_method_ok() {
    val s = "hello".to_string();
    assert_eq(s.len(), 5);
}

#[compile_error]
fn test_vec_unknown_field_rejected() {
    val v: List<Int> = [1, 2, 3];
    val _ = v.bogus_field;
}

#[compile_error]
fn test_array_unknown_field_rejected() {
    val arr: [Int; 2] = [1, 2];
    val _ = arr.sdfsdf;
}

#[compile_error]
fn test_string_unknown_field_rejected() {
    val s = "hello".to_string();
    val _ = s.nonexistent;
}

#[compile_error]
fn test_int_unknown_field_rejected() {
    val n: Int = 42;
    val _ = n.foo;
}
