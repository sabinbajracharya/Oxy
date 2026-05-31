// === Feature: unknown type names in annotations are rejected ===

struct Foo {
    x: int,
}

#[test]
fn test_known_type_ok() {
    let _f: Foo = Foo { x: 1 };
}

#[test]
fn test_vec_of_known_type_ok() {
    let _v: Vec<Foo> = vec(Foo { x: 1 });
}

fn identity(x: int) -> int {
    x
}

#[test]
fn test_known_param_type_ok() {
    assert_eq(identity(5), 5);
}

#[compile_error]
fn test_unknown_bare_type_rejected() {
    let _x: BogusType = 0;
}

#[compile_error]
fn test_unknown_type_in_vec_generic_rejected() {
    // The original ask: `Vec<can_type_anythin_here>` should error.
    let _v: Vec<can_type_anythin_here> = vec(1, 2, 3);
}

#[compile_error]
fn test_unknown_type_in_option_generic_rejected() {
    let _o: Option<NotAType> = None;
}

#[compile_error]
fn test_unknown_type_in_hashmap_generic_rejected() {
    let _m: HashMap<String, NotARealType> = HashMap::new();
}

#[compile_error]
fn test_unknown_return_type_rejected() -> WeirdType {
    0
}
