// === Feature: generic struct field type substitution ===
// `Box<T> { value: T }` — accessing `.value` after `Box<int>` should
// produce `int`, not Unknown.

struct Box<T> {
    value: T,
}

struct Pair<A, B> {
    first: A,
    second: B,
}

#[test]
fn test_generic_struct_init_ok() {
    let b: Box<int> = Box { value: 5 };
    assert_eq!(b.value, 5);
}

#[test]
fn test_generic_struct_string_field_ok() {
    let b: Box<String> = Box { value: "hi".to_string() };
    assert_eq!(b.value, "hi");
}

#[test]
fn test_two_param_generic_ok() {
    let p: Pair<int, String> = Pair { first: 1, second: "x".to_string() };
    assert_eq!(p.first, 1);
    assert_eq!(p.second, "x");
}

#[compile_error]
fn test_generic_struct_field_wrong_type_in_init() {
    // Box<int> can't be initialized with a String value.
    let _b: Box<int> = Box { value: "hello".to_string() };
}

#[compile_error]
fn test_generic_pair_field_wrong_type() {
    let _p: Pair<int, String> = Pair { first: "wrong".to_string(), second: "ok".to_string() };
}

#[compile_error]
fn test_substituted_field_use_wrong_type() {
    // `b.value` is now `int` (substituted from T), so assigning to a
    // String binding must fail.
    let b: Box<int> = Box { value: 5 };
    let _s: String = b.value;
}
