// === STRESS: modules — nesting, visibility, use globs / aliases ===

mod math {
    pub fn add(a: Int, b: Int) -> Int { a + b }
    pub fn sub(a: Int, b: Int) -> Int { a - b }
    fn _private_helper(n: Int) -> Int { n * 2 }

    pub mod advanced {
        pub fn square(n: Int) -> Int { n * n }
        pub fn cube(n: Int) -> Int { n * n * n }
    }
}

#[test]
fn test_module_pub_fn() {
    assert_eq(math::add(2, 3), 5);
    assert_eq(math::sub(10, 4), 6);
}

#[test]
fn test_nested_module() {
    assert_eq(math::advanced::square(5), 25);
    assert_eq(math::advanced::cube(3), 27);
}

// --- use for shorter name ---
use math::add;
use math::advanced::square;

#[test]
fn test_use_simple() {
    assert_eq(add(7, 8), 15);
    assert_eq(square(4), 16);
}

// --- use as alias ---
use math::sub as subtract;

#[test]
fn test_use_alias() {
    assert_eq(subtract(10, 3), 7);
}

// --- use group ---
mod ops {
    pub fn one() -> Int { 1 }
    pub fn two() -> Int { 2 }
    pub fn three() -> Int { 3 }
}

use ops::{one, two, three};

#[test]
fn test_use_group() {
    assert_eq(one() + two() + three(), 6);
}

// --- module-private struct ---
mod data {
    pub struct PubBox {
        pub value: Int,
    }

    pub fn new(v: Int) -> PubBox { PubBox { value: v } }
}

#[test]
fn test_pub_struct_pub_field() {
    val b = data::new(42);
    assert_eq(b.value, 42);
}

// --- field visibility within module ---
mod hidden_fields {
    pub struct Counter {
        pub visible: Int,
        count: Int,
    }
    pub fn new() -> Counter { Counter { visible: 0, count: 0 } }
    pub fn bump(c: Counter) -> Counter {
        c.count = c.count + 1;
        c.visible = c.visible + 1;
        c
    }
    pub fn count(c: Counter) -> Int { c.count }
}

#[test]
fn test_field_visibility_pub() {
    val c = hidden_fields::new();
    assert_eq(c.visible, 0);
    val c2 = hidden_fields::bump(c);
    assert_eq(c2.visible, 1);
    assert_eq(hidden_fields::count(c2), 1);
}

// --- pub ---
mod crate_only {
    pub fn shared() -> Int { 99 }
    pub fn calls_shared() -> Int { shared() }
}

#[test]
fn test_pub_crate() {
    assert_eq(crate_only::shared(), 99);
    assert_eq(crate_only::calls_shared(), 99);
}

// --- enum in module — `mod::Enum::Variant` resolves to the qualified enum ---
mod shapes {
    pub enum Color { Red, Green, Blue }
    pub fn name(c: Color) -> String {
        match c {
            Color::Red => "red".to_string(),
            Color::Green => "green".to_string(),
            Color::Blue => "blue".to_string(),
        }
    }
}

#[test]
fn test_enum_in_module() {
    val c = shapes::Color::Green;
    assert_eq(shapes::name(c), "green");
}

// --- enum via use ---
use shapes::Color;

#[test]
fn test_enum_via_use() {
    val c = Color::Red;
    assert_eq(shapes::name(c), "red");
}
