// === Feature: struct field type checking on init and assignment ===

struct Point {
    x: Float,
    y: Float,
}

struct Person {
    name: String,
    age: Int,
}

#[test]
fn test_struct_init_matching_types() {
    val p = Point { x: 1.0, y: 2.0 };
    assert_eq(p.x, 1.0);
    assert_eq(p.y, 2.0);
}

#[test]
fn test_struct_int_for_float_field_ok() {
    // Integer promotes to Float — accepted.
    val p = Point { x: 1, y: 2 };
    assert_eq(p.x, 1.0);
}

#[test]
fn test_struct_field_mut_assign_ok() {
    var p = Point { x: 0.0, y: 0.0 };
    p.x = 5.0;
    assert_eq(p.x, 5.0);
}

#[compile_error]
fn test_struct_init_string_for_float_rejected() {
    val _ = Point { x: "x".to_string(), y: 2.0 };
}

#[compile_error]
fn test_struct_init_float_for_string_rejected() {
    val _ = Person { name: 1.5, age: 30 };
}

#[compile_error]
fn test_struct_field_assign_string_for_float_rejected() {
    var p = Point { x: 0.0, y: 0.0 };
    p.x = "hello".to_string();
}
