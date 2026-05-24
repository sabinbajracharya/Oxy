// === Feature: Struct Update Syntax `..base` ===
// `Foo { field: val, ..other }` creates a new Foo with the explicit fields
// overriding those from `other`. All remaining fields are copied from `other`.

fn main() {}

struct Point {
    x: int,
    y: int,
}

// === Basic update — change one field ===

#[test]
fn test_update_one_field() {
    let a = Point { x: 1, y: 2 };
    let b = Point { x: 10, ..a };
    assert_eq!(b.x, 10);
    assert_eq!(b.y, 2);
}

// === Update second field ===

#[test]
fn test_update_second_field() {
    let a = Point { x: 1, y: 2 };
    let b = Point { y: 99, ..a };
    assert_eq!(b.x, 1);
    assert_eq!(b.y, 99);
}

// === Original is not mutated ===

#[test]
fn test_original_unchanged() {
    let a = Point { x: 5, y: 6 };
    let b = Point { x: 100, ..a };
    assert_eq!(a.x, 5);
    assert_eq!(b.x, 100);
}

// === No explicit fields — full copy ===

#[test]
fn test_full_copy() {
    let a = Point { x: 3, y: 7 };
    let b = Point { ..a };
    assert_eq!(b.x, 3);
    assert_eq!(b.y, 7);
}

// === Three-field struct ===

struct Config {
    width: int,
    height: int,
    depth: int,
}

#[test]
fn test_three_field_update() {
    let base = Config { width: 1920, height: 1080, depth: 24 };
    let hd = Config { height: 720, ..base };
    assert_eq!(hd.width, 1920);
    assert_eq!(hd.height, 720);
    assert_eq!(hd.depth, 24);
}

// === Chained updates ===

#[test]
fn test_chained_updates() {
    let a = Config { width: 640, height: 480, depth: 8 };
    let b = Config { width: 1280, ..a };
    let c = Config { depth: 32, ..b };
    assert_eq!(c.width, 1280);
    assert_eq!(c.height, 480);
    assert_eq!(c.depth, 32);
}

// === Base is an expression (function return) ===

fn make_point(x: int, y: int) -> Point {
    Point { x, y }
}

#[test]
fn test_update_from_function_return() {
    let p = Point { x: 42, ..make_point(0, 10) };
    assert_eq!(p.x, 42);
    assert_eq!(p.y, 10);
}

// === Both fields overridden (explicit wins over base for all fields) ===

#[test]
fn test_all_fields_overridden() {
    let a = Point { x: 1, y: 2 };
    let b = Point { x: 10, y: 20, ..a };
    assert_eq!(b.x, 10);
    assert_eq!(b.y, 20);
}
