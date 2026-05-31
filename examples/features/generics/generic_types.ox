// === Feature: Generics — Generic Types ===
// Generic structs, enums, and functions parameterized by type.
// Type parameters use inference from field values.

// === Generic Struct: Single Type Param ===

struct Box<T> {
    value: T,
}

#[test]
fn test_generic_struct_single() {
    val b = Box { value: 42 };
    assert_eq(b.value, 42);
    val s = Box { value: "hello".to_string() };
    assert_eq(s.value, "hello");
}

// === Generic Struct: Multiple Type Params ===

struct Pair<A, B> {
    first: A,
    second: B,
}

#[test]
fn test_generic_struct_multi() {
    val p = Pair { first: 10, second: "ten".to_string() };
    assert_eq(p.first, 10);
    assert_eq(p.second, "ten");
}

// === Generic Struct: Same Type Inference ===

#[test]
fn test_generic_struct_same_type() {
    val b1 = Box { value: 100 };
    val b2 = Box { value: 200 };
    assert_eq(b1.value, 100);
}

// === Generic Enum ===

enum MyOption<T> {
    Some(T),
    None,
}

#[test]
fn test_generic_enum_some() {
    val x = MyOption::Some(42);
    val is_some = match x {
        MyOption::Some(_) => true,
        MyOption::None => false,
    };
    assert(is_some);
}

// === Generic Function ===

fn identity<T>(x: T) -> T {
    x
}

#[test]
fn test_generic_function() {
    assert_eq(identity(42), 42);
    assert_eq(identity("hello".to_string()), "hello");
}

// === Generic Function with Multiple Params ===

fn make_pair<A, B>(a: A, b: B) -> Pair<A, B> {
    Pair { first: a, second: b }
}

#[test]
fn test_generic_function_multi() {
    val p = make_pair(42, true);
    assert_eq(p.first, 42);
    assert_eq(p.second, true);
}

// === Generic Struct with Method ===

struct Wrapper<T> {
    inner: T,
}

impl Wrapper {
    fn new(value: T) -> Wrapper<T> {
        Wrapper { inner: value }
    }
    fn get(self) -> T {
        self.inner
    }
}

#[test]
fn test_generic_struct_method() {
    val w = Wrapper::new(42);
    assert_eq(w.get(), 42);
}
