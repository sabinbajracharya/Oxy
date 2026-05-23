// === Feature: index and range operand type checking ===

#[test]
fn test_vec_index_with_int_ok() {
    let v = vec![10, 20, 30];
    assert_eq!(v[1], 20);
}

#[test]
fn test_string_index_with_int_ok() {
    let s = "hi".to_string();
    assert_eq!(s[0], 'h');
}

#[test]
fn test_range_with_ints_ok() {
    let mut total = 0;
    for i in 1..=3 {
        total += i;
    }
    assert_eq!(total, 6);
}

#[compile_error]
fn test_vec_index_with_string_rejected() {
    let v = vec![1, 2, 3];
    let _ = v["zero"];
}

#[compile_error]
fn test_vec_index_with_bool_rejected() {
    let v = vec![1, 2, 3];
    let _ = v[true];
}

#[compile_error]
fn test_range_with_strings_rejected() {
    let _r = "a"..="z";
}
