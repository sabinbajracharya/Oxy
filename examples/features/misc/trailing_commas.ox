// === Feature: Trailing Commas ===
// Like Rust, Oxy accepts a trailing comma in argument and field lists.

fn add3(a: Int, b: Int, c: Int) -> Int { a + b + c }

struct Point { x: Int, y: Int }

enum E { Tri(Int, Int, Int) }

#[test]
fn test_trailing_comma_in_function_call() {
    assert::eq(add3(1, 2, 3,), 6);
}

#[test]
fn test_trailing_comma_in_enum_variant_constructor() {
    val t = E::Tri(10, 20, 30,);
    match t {
        E::Tri(a, b, c) => {
            assert::eq(a, 10);
            assert::eq(b, 20);
            assert::eq(c, 30);
        }
    }
}

#[test]
fn test_trailing_comma_in_struct_init() {
    val p = Point { x: 1, y: 2, };
    assert::eq(p.x, 1);
    assert::eq(p.y, 2);
}

#[test]
fn test_trailing_comma_in_vec_macro() {
    val v = [1, 2, 3,];
    assert::eq(v.len(), 3);
}

#[test]
fn test_trailing_comma_in_array_literal() {
    val arr = [4, 5, 6,];
    assert::eq(arr.len(), 3);
}
