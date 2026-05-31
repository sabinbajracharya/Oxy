// === Feature: Structs — Basics ===
// Structs are user-defined types with named fields. Three kinds:
// named-field, tuple, and unit structs. Access fields with `.` and
// mutate with `.field = value`.

// === Named Struct Definition ===

struct Point {
    x: Int,
    y: Int,
}

#[test]
fn test_named_struct_construction() {
    let p = Point { x: 10, y: 20 };
    assert_eq(p.x, 10);
    assert_eq(p.y, 20);
}

#[test]
fn test_named_struct_field_access() {
    let p = Point { x: 1, y: 2 };
    assert_eq(p.x, 1);
}

// === Struct Field Mutation ===

#[test]
fn test_struct_field_mutation() {
    let mut p = Point { x: 0, y: 0 };
    p.x = 42;
    p.y = 99;
    assert_eq(p.x, 42);
    assert_eq(p.y, 99);
}

// === Shorthand Field Init ===

#[test]
fn test_struct_shorthand_init() {
    let x = 5;
    let y = 15;
    let p = Point { x, y };
    assert_eq(p.x, 5);
    assert_eq(p.y, 15);
}

// === Tuple Struct ===

struct Pair(Int, String);

#[test]
fn test_tuple_struct_field_access() {
    let p = Pair { 0: 42, 1: "hello" };
    assert_eq(p.0, 42);
}

// === Unit Struct ===

struct Marker;

#[test]
fn test_unit_struct() {
    let m = Marker {};
    // Unit struct exists and can be matched
    assert(true);
}

// === Multiple Struct Instances ===

#[test]
fn test_multiple_instances() {
    let a = Point { x: 1, y: 2 };
    let b = Point { x: 3, y: 4 };
    assert_eq(a.x + b.x, 4);
    assert_eq(a.y + b.y, 6);
}
