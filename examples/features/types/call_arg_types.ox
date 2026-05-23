// === Feature: function-call argument type checking ===
// The type checker must reject calls whose arguments don't match the
// declared parameter types (or whose arity is wrong).

fn sum(x: int) -> int {
    x + x
}

fn add(a: int, b: int) -> int {
    a + b
}

fn greet(name: String) -> String {
    f"hello, {name}"
}

#[test]
fn test_call_matching_arg_types() {
    let a: int = 21;
    assert_eq!(sum(a), 42);
}

#[test]
fn test_call_int_promotion_ok() {
    // Any-integer-accepts-any-integer is allowed (wrapping at runtime).
    let n: int = 10;
    assert_eq!(add(n, 5), 15);
}

#[test]
fn test_call_string_arg_ok() {
    assert_eq!(greet("world".to_string()), "hello, world".to_string());
}

#[compile_error]
fn test_call_string_for_int_param_rejected() {
    // The original bug: sum expects int but got a String → "stringstring".
    // After the fix this must be a compile-time TypeError.
    let _ = sum("string");
}

#[compile_error]
fn test_call_string_for_int_param_via_let_rejected() {
    let s = "string".to_string();
    let _ = sum(s);
}

#[compile_error]
fn test_call_int_for_string_param_rejected() {
    let _ = greet(42);
}

#[compile_error]
fn test_call_too_few_args_rejected() {
    let _ = add(1);
}

#[compile_error]
fn test_call_too_many_args_rejected() {
    let _ = add(1, 2, 3);
}
