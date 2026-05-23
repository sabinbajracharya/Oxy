// === Feature: array literal type checking against [T; N] ===
// Array literals must match their declared element type and length.

#[test]
fn test_array_literal_matches_declared() {
    let arr: [int; 3] = [1, 2, 3];
    assert_eq!(arr.len(), 3);
}

#[test]
fn test_array_literal_int_promotion_ok() {
    let arr: [int; 2] = [1, 2];
    assert_eq!(arr[0], 1);
}

#[compile_error]
fn test_array_literal_wrong_length_too_few() {
    let _: [int; 3] = [1, 2];
}

#[compile_error]
fn test_array_literal_wrong_length_too_many() {
    let _: [int; 2] = [1, 2, 3, 4];
}

#[compile_error]
fn test_array_literal_wrong_element_type() {
    let _: [int; 2] = [1, "name"];
}

#[compile_error]
fn test_array_literal_string_for_int_array() {
    let _: [int; 1] = ["hello"];
}
