// === Feature: vec! macro element type checking ===
// `vec![...]` must be homogeneous — mixing element types is rejected.

#[test]
fn test_vec_macro_homogeneous_ints() {
    let v = vec![1, 2, 3];
    assert_eq!(v.len(), 3);
}

#[test]
fn test_vec_macro_homogeneous_strings() {
    let v = vec!["a".to_string(), "b".to_string()];
    assert_eq!(v.len(), 2);
}

#[test]
fn test_vec_macro_empty_ok() {
    let v: Vec<i64> = vec![];
    assert_eq!(v.len(), 0);
}

#[compile_error]
fn test_vec_macro_mixed_int_string_rejected() {
    // The original bug: `vec![1, 2, 3, "what the hell"]` used to be accepted.
    let _ = vec![1, 2, 3, "what the hell"];
}

#[compile_error]
fn test_vec_macro_mixed_int_bool_rejected() {
    let _ = vec![1, true];
}

#[compile_error]
fn test_vec_macro_mixed_float_string_rejected() {
    let _ = vec![1.5, "hello"];
}
