// === Feature: Struct Update Syntax `..base` ===
// `Foo { field: val, ..other }` creates a new Foo with the explicit fields
// overriding those from `other`. All remaining fields are copied from `other`.

fn main() {}

struct Point {
    x: Int,
    y: Int,
}

// === Basic update — change one field ===

#[test]
fn test_update_one_field() {
    val a = Point { x: 1, y: 2 };
    val b = Point { x: 10, ..a };
    assert::eq(b.x, 10);
    assert::eq(b.y, 2);
}

// === Update second field ===

#[test]
fn test_update_second_field() {
    val a = Point { x: 1, y: 2 };
    val b = Point { y: 99, ..a };
    assert::eq(b.x, 1);
    assert::eq(b.y, 99);
}

// === Original is not mutated ===

#[test]
fn test_original_unchanged() {
    val a = Point { x: 5, y: 6 };
    val b = Point { x: 100, ..a };
    assert::eq(a.x, 5);
    assert::eq(b.x, 100);
}

// === No explicit fields — full copy ===

#[test]
fn test_full_copy() {
    val a = Point { x: 3, y: 7 };
    val b = Point { ..a };
    assert::eq(b.x, 3);
    assert::eq(b.y, 7);
}

// === Three-field struct ===

struct Config {
    width: Int,
    height: Int,
    depth: Int,
}

#[test]
fn test_three_field_update() {
    val base = Config { width: 1920, height: 1080, depth: 24 };
    val hd = Config { height: 720, ..base };
    assert::eq(hd.width, 1920);
    assert::eq(hd.height, 720);
    assert::eq(hd.depth, 24);
}

// === Chained updates ===

#[test]
fn test_chained_updates() {
    val a = Config { width: 640, height: 480, depth: 8 };
    val b = Config { width: 1280, ..a };
    val c = Config { depth: 32, ..b };
    assert::eq(c.width, 1280);
    assert::eq(c.height, 480);
    assert::eq(c.depth, 32);
}

// === Base is an expression (function return) ===

fn make_point(x: Int, y: Int) -> Point {
    Point { x, y }
}

#[test]
fn test_update_from_function_return() {
    val p = Point { x: 42, ..make_point(0, 10) };
    assert::eq(p.x, 42);
    assert::eq(p.y, 10);
}

// === Both fields overridden (explicit wins over base for all fields) ===

#[test]
fn test_all_fields_overridden() {
    val a = Point { x: 1, y: 2 };
    val b = Point { x: 10, y: 20, ..a };
    assert::eq(b.x, 10);
    assert::eq(b.y, 20);
}
