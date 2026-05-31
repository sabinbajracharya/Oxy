// === Feature: `[...]` creates a growable List<T> ===
// List<T> coerces to [T; N] fixed-size array annotations via accepts().

#[test]
fn test_list_literal_matches_declared() {
    val arr: [Int; 3] = [1, 2, 3];
    assert_eq(arr.len(), 3);
}

#[test]
fn test_list_literal_int_promotion_ok() {
    val arr: [Int; 2] = [1, 2];
    assert_eq(arr[0], 1);
}

#[compile_error]
fn test_array_literal_wrong_element_type() {
    val _: [Int; 2] = [1, "name"];
}

#[compile_error]
fn test_array_literal_string_for_int_array() {
    val _: [Int; 1] = ["hello"];
}
