// === Feature: vec function element type checking ===
// `vec(...)` must be homogeneous — mixing element types is rejected.

#[test]
fn test_vec_macro_homogeneous_ints() {
    let v = vec(1, 2, 3);
    assert_eq(v.len(), 3);
}

#[test]
fn test_vec_macro_homogeneous_strings() {
    let v = vec("a".to_string(), "b".to_string());
    assert_eq(v.len(), 2);
}

#[test]
fn test_vec_macro_empty_ok() {
    let v: Vec<int> = vec();
    assert_eq(v.len(), 0);
}

#[compile_error]
fn test_vec_macro_mixed_int_string_rejected() {
    // The original bug: `vec(1, 2, 3, "what the hell")` used to be accepted.
    let _ = vec(1, 2, 3, "what the hell");
}

#[compile_error]
fn test_vec_macro_mixed_int_bool_rejected() {
    let _ = vec(1, true);
}

#[compile_error]
fn test_vec_macro_mixed_float_string_rejected() {
    let _ = vec(1.5, "hello");
}

// === Vec<T> generic-argument enforcement ===

#[test]
fn test_vec_i64_accepts_int_elements() {
    let v: Vec<int> = vec(1, 2, 3);
    assert_eq(v.len(), 3);
}

#[test]
fn test_vec_string_accepts_string_elements() {
    let v: Vec<String> = vec("a".to_string(), "b".to_string());
    assert_eq(v.len(), 2);
}

#[compile_error]
fn test_vec_i64_rejects_string_elements() {
    let _v: Vec<int> = vec("hi".to_string());
}

#[compile_error]
fn test_vec_string_rejects_int_elements() {
    let _v: Vec<String> = vec(1, 2, 3);
}

#[compile_error]
fn test_vec_push_wrong_arg_type_rejected() {
    let mut v: Vec<int> = vec(1, 2, 3);
    v.push("hello".to_string());
}

#[test]
fn test_vec_index_returns_element_type() {
    let v: Vec<int> = vec(10, 20, 30);
    let x: int = v[1];
    assert_eq(x, 20);
}

#[compile_error]
fn test_vec_index_element_type_mismatch_rejected() {
    let v: Vec<int> = vec(10, 20, 30);
    let _x: String = v[1];
}
