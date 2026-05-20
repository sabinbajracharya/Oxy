// === Feature: Generics — Turbofish Syntax ===
// Explicit type arguments via `::<Type>` syntax on structs, functions,
// and method calls.

// === Turbofish on Struct Init ===

struct Box<T> {
    value: T,
}

#[test]
fn test_turbofish_struct_init() {
    let b = Box::<i64> { value: 42 };
    assert_eq!(b.value, 42);
}

// === Turbofish on Generic Function ===

fn identity<T>(x: T) -> T {
    x
}

#[test]
fn test_turbofish_function() {
    let x = identity::<i64>(42);
    assert_eq!(x, 42);
}

// === Turbofish on Generic Enum Variant ===

enum MyOption<T> {
    Some(T),
    None,
}

#[test]
fn test_turbofish_enum() {
    let x: MyOption<i64> = MyOption::<i64>::Some(42);
    match x {
        MyOption::Some(_) => {},
        MyOption::None => panic!("expected Some"),
    }
}

// === Turbofish with Multiple Type Args ===

struct Pair<A, B> {
    first: A,
    second: B,
}

#[test]
fn test_turbofish_multi_struct() {
    let p = Pair::<i64, String> { first: 10, second: "ten".to_string() };
    assert_eq!(p.first, 10);
}
