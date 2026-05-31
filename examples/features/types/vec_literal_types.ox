// === Feature: vec function element type checking ===
// `list(...)` must be homogeneous — mixing element types is rejected.

#[test]
fn test_vec_macro_homogeneous_ints() {
    let v = list(1, 2, 3);
    assert_eq(v.len(), 3);
}

#[test]
fn test_vec_macro_homogeneous_strings() {
    let v = list("a".to_string(), "b".to_string());
    assert_eq(v.len(), 2);
}

#[test]
fn test_vec_macro_empty_ok() {
    let v: List<Int> = list();
    assert_eq(v.len(), 0);
}

#[compile_error]
fn test_vec_macro_mixed_int_string_rejected() {
    // The original bug: `list(1, 2, 3, "what the hell")` used to be accepted.
    let _ = list(1, 2, 3, "what the hell");
}

#[compile_error]
fn test_vec_macro_mixed_int_bool_rejected() {
    let _ = list(1, true);
}

#[compile_error]
fn test_vec_macro_mixed_float_string_rejected() {
    let _ = list(1.5, "hello");
}

// === List<T> generic-argument enforcement ===

#[test]
fn test_vec_i64_accepts_int_elements() {
    let v: List<Int> = list(1, 2, 3);
    assert_eq(v.len(), 3);
}

#[test]
fn test_vec_string_accepts_string_elements() {
    let v: List<String> = list("a".to_string(), "b".to_string());
    assert_eq(v.len(), 2);
}

#[compile_error]
fn test_vec_i64_rejects_string_elements() {
    let _v: List<Int> = list("hi".to_string());
}

#[compile_error]
fn test_vec_string_rejects_int_elements() {
    let _v: List<String> = list(1, 2, 3);
}

#[compile_error]
fn test_vec_push_wrong_arg_type_rejected() {
    let mut v: List<Int> = list(1, 2, 3);
    v.push("hello".to_string());
}

#[test]
fn test_vec_index_returns_element_type() {
    let v: List<Int> = list(10, 20, 30);
    let x: Int = v[1];
    assert_eq(x, 20);
}

#[compile_error]
fn test_vec_index_element_type_mismatch_rejected() {
    let v: List<Int> = list(10, 20, 30);
    let _x: String = v[1];
}
