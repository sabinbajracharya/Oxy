// === Feature: index and range operand type checking ===

#[test]
fn test_vec_index_with_int_ok() {
    val v = [10, 20, 30];
    assert_eq(v[1], 20);
}

#[test]
fn test_string_index_with_int_ok() {
    val s = "hi".to_string();
    assert_eq(s[0], 'h');
}

#[test]
fn test_range_with_ints_ok() {
    var total = 0;
    for i in 1..=3 {
        total += i;
    }
    assert_eq(total, 6);
}

#[compile_error]
fn test_vec_index_with_string_rejected() {
    val v = [1, 2, 3];
    val _ = v["zero"];
}

#[compile_error]
fn test_vec_index_with_bool_rejected() {
    val v = [1, 2, 3];
    val _ = v[true];
}

#[compile_error]
fn test_range_with_strings_rejected() {
    val _r = "a"..="z";
}
