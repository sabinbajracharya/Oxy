// === Feature: Generics — Type Mismatch Detection ===
// The type checker must reject calls to generic functions when the same
// generic type parameter is bound to incompatible concrete types.

// === Helper: generic identity function ===

fn identity<T>(x: T) -> T {
    x
}

#[test]
fn test_identity_same_type_int() {
    assert_eq(identity(42), 42);
}

#[test]
fn test_identity_same_type_string() {
    assert_eq(identity("hello".to_string()), "hello");
}

// === Single generic param, mixed arg types ===

fn sum_both<T>(a: T, b: T) -> T {
    a
}

#[test]
fn test_same_generic_param_same_type_ok() {
    // Both args Int → should compile
    let _r = sum_both(1, 2);
}

#[test]
fn test_same_generic_param_string_ok() {
    // Both args String → should compile
    let _r = sum_both("a".to_string(), "b".to_string());
}

#[compile_error]
fn test_same_generic_param_int_and_string_rejected() {
    // T bound to Int for first arg, String for second → type mismatch
    let _r = sum_both(5, "SABOM".to_string());
}

#[compile_error]
fn test_same_generic_param_string_and_int_rejected() {
    let _r = sum_both("hello".to_string(), 42);
}

#[compile_error]
fn test_same_generic_param_bool_and_int_rejected() {
    let _r = sum_both(true, 42);
}

// === Single generic param, variable args ===

#[compile_error]
fn test_same_generic_param_vars_mixed_types_rejected() {
    let a: Int = 5;
    let b: String = "world".to_string();
    let _r = sum_both(a, b);
}

#[test]
fn test_same_generic_param_vars_same_type_ok() {
    let a: Int = 5;
    let b: Int = 10;
    let _r = sum_both(a, b);
}

// === Three generic params, mixed violations ===

fn triple<T>(a: T, b: T, c: T) -> T {
    a
}

#[test]
fn test_triple_same_type_ok() {
    let _r = triple(1, 2, 3);
}

#[compile_error]
fn test_triple_first_two_ok_third_mismatch_rejected() {
    let _r = triple(1, 2, "bad".to_string());
}

#[compile_error]
fn test_triple_middle_mismatch_rejected() {
    let _r = triple(1, "bad".to_string(), 3);
}

// === Multiple generic params, independent types are fine ===

fn make_pair<A, B>(first: A, second: B) -> Pair<A, B> {
    Pair { first, second }
}

struct Pair<A, B> {
    first: A,
    second: B,
}

#[test]
fn test_different_generic_params_different_types_ok() {
    // A=Int, B=String — different generic params, should compile
    let p = make_pair(42, "hello".to_string());
    assert_eq(p.first, 42);
    assert_eq(p.second, "hello");
}

#[test]
fn test_different_generic_params_same_type_ok() {
    // A=Int, B=Int — different params can be same concrete type
    let p = make_pair(42, 100);
    assert_eq(p.first, 42);
    assert_eq(p.second, 100);
}

// === Multiple generic params where same param reused ===

fn first_of_three<A, B>(a: A, b: B, c: A) -> A {
    a
}

#[test]
fn test_reused_generic_param_same_type_ok() {
    let _r = first_of_three(1, true, 2);
}

#[compile_error]
fn test_reused_generic_param_mismatch_rejected() {
    let _r = first_of_three(1, true, "bad".to_string());
}

// === Generic function returning generic type ===

fn choose<T>(a: T, b: T) -> T {
    a
}

#[test]
fn test_choose_same_type_ok() {
    assert_eq(choose(10, 20), 10);
}

#[compile_error]
fn test_choose_mixed_type_rejected() {
    let _r = choose(10, "not Int".to_string());
}

// === Generic with concrete trailing param ===

fn tag_value<T>(label: String, value: T) -> T {
    value
}

#[test]
fn test_mixed_generic_and_concrete_ok() {
    assert_eq(tag_value("age".to_string(), 42), 42);
}

#[test]
fn test_mixed_generic_and_concrete_different_types_ok() {
    // Different calls with different T are fine
    assert_eq(tag_value("name".to_string(), "Sabin".to_string()), "Sabin");
}

// === Struct with same-type constraint via single generic param ===

struct SameType<T> {
    a: T,
    b: T,
}

#[test]
fn test_same_type_struct_same_types_ok() {
    let s = SameType { a: 10, b: 20 };
    assert_eq(s.a, 10);
    assert_eq(s.b, 20);
}

#[compile_error]
fn test_same_type_struct_mixed_types_rejected() {
    let _s = SameType { a: 10, b: "hello".to_string() };
}

// === Enum with generic type mismatch ===

enum MyResult<T, E> {
    Ok(T),
    Err(E),
}

#[test]
fn test_result_ok_ok() {
    let _r: MyResult<Int, String> = MyResult::Ok(42);
}

#[test]
fn test_result_err_ok() {
    let _r: MyResult<Int, String> = MyResult::Err("oops".to_string());
}

// === Impl block method with generics, type mismatch ===

struct Cell<T> {
    value: T,
}

impl Cell {
    fn new(val: T) -> Cell<T> {
        Cell { value: val }
    }

    fn replace(self, new_val: T) -> Cell<T> {
        Cell { value: new_val }
    }
}

#[test]
fn test_cell_new_ok() {
    let c = Cell::new(42);
    assert_eq(c.value, 42);
}

#[test]
fn test_cell_replace_same_type_ok() {
    let c = Cell::new(10);
    let c2 = c.replace(20);
    assert_eq(c2.value, 20);
}

#[compile_error]
fn test_cell_replace_wrong_type_rejected() {
    let c = Cell::new(10);
    let _c2 = c.replace("wrong".to_string());
}

// === Generic function with turbofish, type mismatch ===

fn id_t<T>(x: T) -> T {
    x
}

#[test]
fn test_turbofish_correct_type_ok() {
    let x = id_t::<Int>(42);
    assert_eq(x, 42);
}

// NOTE: Generic function bodies with operators like `a + b` cannot be
// checked for type-class validity without trait bounds (not yet in Oxy).
// Those will fail at runtime for unsupported types.
