// === Feature: reject unknown field/method access on builtin types ===

#[test]
fn test_vec_known_method_ok() {
    let v: List<Int> = [1, 2, 3];
    assert_eq(v.len(), 3);
}

#[test]
fn test_string_known_method_ok() {
    let s = "hello".to_string();
    assert_eq(s.len(), 5);
}

#[compile_error]
fn test_vec_unknown_field_rejected() {
    let v: List<Int> = [1, 2, 3];
    let _ = v.bogus_field;
}

#[compile_error]
fn test_array_unknown_field_rejected() {
    let arr: [Int; 2] = [1, 2];
    let _ = arr.sdfsdf;
}

#[compile_error]
fn test_string_unknown_field_rejected() {
    let s = "hello".to_string();
    let _ = s.nonexistent;
}

#[compile_error]
fn test_int_unknown_field_rejected() {
    let n: Int = 42;
    let _ = n.foo;
}
