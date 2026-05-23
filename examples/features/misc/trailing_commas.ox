// === Feature: Trailing Commas ===
// Like Rust, Oxy accepts a trailing comma in argument and field lists.

fn add3(a: int, b: int, c: int) -> int { a + b + c }

struct Point { x: int, y: int }

enum E { Tri(int, int, int) }

#[test]
fn test_trailing_comma_in_function_call() {
    assert_eq!(add3(1, 2, 3,), 6);
}

#[test]
fn test_trailing_comma_in_enum_variant_constructor() {
    let t = E::Tri(10, 20, 30,);
    match t {
        E::Tri(a, b, c) => {
            assert_eq!(a, 10);
            assert_eq!(b, 20);
            assert_eq!(c, 30);
        }
    }
}

#[test]
fn test_trailing_comma_in_struct_init() {
    let p = Point { x: 1, y: 2, };
    assert_eq!(p.x, 1);
    assert_eq!(p.y, 2);
}

#[test]
fn test_trailing_comma_in_vec_macro() {
    let v = vec![1, 2, 3,];
    assert_eq!(v.len(), 3);
}

#[test]
fn test_trailing_comma_in_array_literal() {
    let arr = [4, 5, 6,];
    assert_eq!(arr.len(), 3);
}
