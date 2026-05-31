// === STRESS: generics — types, fns, turbofish, monomorphization ===

// --- generic struct, single param ---
struct Wrap<T> { value: T }

#[test]
fn test_generic_struct_int() {
    let w = Wrap { value: 42 };
    assert_eq(w.value, 42);
}
#[test]
fn test_generic_struct_string() {
    let w = Wrap { value: "hi".to_string() };
    assert_eq(w.value, "hi");
}
#[test]
fn test_generic_struct_bool() {
    let w = Wrap { value: true };
    assert_eq(w.value, true);
}

// --- generic struct, multi param ---
struct Pair<A, B> { first: A, second: B }

#[test]
fn test_generic_multi_param() {
    let p = Pair { first: 1, second: "one".to_string() };
    assert_eq(p.first, 1);
    assert_eq(p.second, "one");
}

// --- generic fn, single param, no bounds ---
fn identity<T>(x: T) -> T { x }

#[test]
fn test_generic_identity_int() { assert_eq(identity(42), 42); }
#[test]
fn test_generic_identity_string() { assert_eq(identity("hi".to_string()), "hi"); }
#[test]
fn test_generic_identity_bool() { assert_eq(identity(true), true); }

// --- generic fn, two args same type ---
fn pick_first<T>(a: T, _b: T) -> T { a }

#[test]
fn test_generic_pick_first() {
    assert_eq(pick_first(10, 20), 10);
}

// --- turbofish on generic fn ---
fn returns_value<T>(x: T) -> T { x }

#[test]
fn test_turbofish_explicit() {
    let n: Int = returns_value::<Int>(42);
    assert_eq(n, 42);
}

// --- generic over List ---
fn first_elem<T>(v: List<T>) -> Option<T> {
    if v.len() == 0 { None } else { Some(v[0]) }
}

#[test]
fn test_generic_over_vec_int() {
    assert_eq(first_elem(list(1, 2, 3)), Some(1));
}
#[test]
fn test_generic_over_vec_empty() {
    let v: List<Int> = list();
    assert_eq(first_elem(v), None);
}

// --- nested generic types ---
#[test]
fn test_vec_of_option_int() {
    let v: List<Option<Int>> = list(Some(1), None, Some(3));
    assert_eq(v.len(), 3);
}
#[test]
fn test_option_of_vec_int() {
    let o: Option<List<Int>> = Some(list(1, 2, 3));
    assert_eq(o.is_some(), true);
}

// --- Map<K, V> ---
use std::collections::Map;

#[test]
fn test_hashmap_string_int() {
    let mut m: Map<String, Int> = Map::new();
    m.insert("a".to_string(), 1);
    m.insert("b".to_string(), 2);
    assert_eq(m.get("a"), Some(1));
    assert_eq(m.get("missing"), None);
}

// --- generic struct method (concrete impl) ---
struct Box1<T> { v: T }
impl Box1<Int> {
    fn get_int(self) -> Int { self.v }
}
#[test]
fn test_generic_struct_concrete_method() {
    let b = Box1 { v: 7 };
    assert_eq(b.get_int(), 7);
}

// --- turbofish on PathCall ---
struct Stash<T> { items: List<T> }
impl Stash<Int> {
    fn new() -> Stash<Int> { Stash { items: list() } }
}
#[test]
fn test_turbofish_on_path_call() {
    let s: Stash<Int> = Stash::<Int>::new();
    assert_eq(s.items.len(), 0);
}
