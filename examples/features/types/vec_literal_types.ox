// === Feature: vec function element type checking ===
// `[...]` must be homogeneous — mixing element types is rejected.

#[test]
fn test_vec_macro_homogeneous_ints() {
    val v = [1, 2, 3];
    assert::eq(v.len(), 3);
}

#[test]
fn test_vec_macro_homogeneous_strings() {
    val v = ["a".to_string(), "b".to_string()];
    assert::eq(v.len(), 2);
}

#[test]
fn test_vec_macro_empty_ok() {
    val v: List<Int> = [];
    assert::eq(v.len(), 0);
}

#[compile_error]
fn test_vec_macro_mixed_int_string_rejected() {
    // The original bug: `[1, 2, 3, "what the hell"]` used to be accepted.
    val _ = [1, 2, 3, "what the hell"];
}

#[compile_error]
fn test_vec_macro_mixed_int_bool_rejected() {
    val _ = [1, true];
}

#[compile_error]
fn test_vec_macro_mixed_float_string_rejected() {
    val _ = [1.5, "hello"];
}

// === List<T> generic-argument enforcement ===

#[test]
fn test_vec_i64_accepts_int_elements() {
    val v: List<Int> = [1, 2, 3];
    assert::eq(v.len(), 3);
}

#[test]
fn test_vec_string_accepts_string_elements() {
    val v: List<String> = ["a".to_string(), "b".to_string()];
    assert::eq(v.len(), 2);
}

#[compile_error]
fn test_vec_i64_rejects_string_elements() {
    val _v: List<Int> = ["hi".to_string()];
}

#[compile_error]
fn test_vec_string_rejects_int_elements() {
    val _v: List<String> = [1, 2, 3];
}

#[compile_error]
fn test_vec_push_wrong_arg_type_rejected() {
    var v: List<Int> = [1, 2, 3];
    v.push("hello".to_string());
}

#[test]
fn test_vec_index_returns_element_type() {
    val v: List<Int> = [10, 20, 30];
    val x: Int = v[1];
    assert::eq(x, 20);
}

#[compile_error]
fn test_vec_index_element_type_mismatch_rejected() {
    val v: List<Int> = [10, 20, 30];
    val _x: String = v[1];
}
