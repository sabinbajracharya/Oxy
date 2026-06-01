// === Feature: unknown type names in annotations are rejected ===

struct Foo {
    x: Int,
}

#[test]
fn test_known_type_ok() {
    val _f: Foo = Foo { x: 1 };
}

#[test]
fn test_vec_of_known_type_ok() {
    val _v: List<Foo> = [Foo { x: 1 }];
}

fn identity(x: Int) -> Int {
    x
}

#[test]
fn test_known_param_type_ok() {
    assert::eq(identity(5), 5);
}

#[compile_error]
fn test_unknown_bare_type_rejected() {
    val _x: BogusType = 0;
}

#[compile_error]
fn test_unknown_type_in_vec_generic_rejected() {
    // The original ask: `List<can_type_anythin_here>` should error.
    val _v: List<can_type_anythin_here> = [1, 2, 3];
}

#[compile_error]
fn test_unknown_type_in_option_generic_rejected() {
    val _o: Option<NotAType> = None;
}

#[compile_error]
fn test_unknown_type_in_hashmap_generic_rejected() {
    val _m: Map<String, NotARealType> = Map::new();
}

#[compile_error]
fn test_unknown_return_type_rejected() -> WeirdType {
    0
}
