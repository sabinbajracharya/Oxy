// === Feature: generic struct field type substitution ===
// `Box<T> { value: T }` — accessing `.value` after `Box<Int>` should
// produce `Int`, not Unknown.

struct Box<T> {
    value: T,
}

struct Pair<A, B> {
    first: A,
    second: B,
}

#[test]
fn test_generic_struct_init_ok() {
    val b: Box<Int> = Box { value: 5 };
    assert_eq(b.value, 5);
}

#[test]
fn test_generic_struct_string_field_ok() {
    val b: Box<String> = Box { value: "hi".to_string() };
    assert_eq(b.value, "hi");
}

#[test]
fn test_two_param_generic_ok() {
    val p: Pair<Int, String> = Pair { first: 1, second: "x".to_string() };
    assert_eq(p.first, 1);
    assert_eq(p.second, "x");
}

#[compile_error]
fn test_generic_struct_field_wrong_type_in_init() {
    // Box<Int> can't be initialized with a String value.
    val _b: Box<Int> = Box { value: "hello".to_string() };
}

#[compile_error]
fn test_generic_pair_field_wrong_type() {
    val _p: Pair<Int, String> = Pair { first: "wrong".to_string(), second: "ok".to_string() };
}

#[compile_error]
fn test_substituted_field_use_wrong_type() {
    // `b.value` is now `Int` (substituted from T), so assigning to a
    // String binding must fail.
    val b: Box<Int> = Box { value: 5 };
    val _s: String = b.value;
}
