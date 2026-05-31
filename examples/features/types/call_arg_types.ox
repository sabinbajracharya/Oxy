// === Feature: function-call argument type checking ===
// The type checker must reject calls whose arguments don't match the
// declared parameter types (or whose arity is wrong).

fn sum(x: Int) -> Int {
    x + x
}

fn add(a: Int, b: Int) -> Int {
    a + b
}

fn greet(name: String) -> String {
    f"hello, {name}"
}

#[test]
fn test_call_matching_arg_types() {
    val a: Int = 21;
    assert_eq(sum(a), 42);
}

#[test]
fn test_call_int_promotion_ok() {
    // Any-integer-accepts-any-integer is allowed (wrapping at runtime).
    val n: Int = 10;
    assert_eq(add(n, 5), 15);
}

#[test]
fn test_call_string_arg_ok() {
    assert_eq(greet("world".to_string()), "hello, world".to_string());
}

#[compile_error]
fn test_call_string_for_int_param_rejected() {
    // The original bug: sum expects Int but got a String → "stringstring".
    // After the fix this must be a compile-time TypeError.
    val _ = sum("string");
}

#[compile_error]
fn test_call_string_for_int_param_via_let_rejected() {
    val s = "string".to_string();
    val _ = sum(s);
}

#[compile_error]
fn test_call_int_for_string_param_rejected() {
    val _ = greet(42);
}

#[compile_error]
fn test_call_too_few_args_rejected() {
    val _ = add(1);
}

#[compile_error]
fn test_call_too_many_args_rejected() {
    val _ = add(1, 2, 3);
}
