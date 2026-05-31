// === STRESS: modules — nesting, visibility, use globs / aliases ===

mod math {
    pub fn add(a: int, b: int) -> int { a + b }
    pub fn sub(a: int, b: int) -> int { a - b }
    fn _private_helper(n: int) -> int { n * 2 }

    pub mod advanced {
        pub fn square(n: int) -> int { n * n }
        pub fn cube(n: int) -> int { n * n * n }
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
    pub fn one() -> int { 1 }
    pub fn two() -> int { 2 }
    pub fn three() -> int { 3 }
}

use ops::{one, two, three};

#[test]
fn test_use_group() {
    assert_eq(one() + two() + three(), 6);
}

// --- module-private struct ---
mod data {
    pub struct PubBox {
        pub value: int,
    }

    pub fn new(v: int) -> PubBox { PubBox { value: v } }
}

#[test]
fn test_pub_struct_pub_field() {
    let b = data::new(42);
    assert_eq(b.value, 42);
}

// --- field visibility within module ---
mod hidden_fields {
    pub struct Counter {
        pub visible: int,
        count: int,
    }
    pub fn new() -> Counter { Counter { visible: 0, count: 0 } }
    pub fn bump(c: Counter) -> Counter {
        c.count = c.count + 1;
        c.visible = c.visible + 1;
        c
    }
    pub fn count(c: Counter) -> int { c.count }
}

#[test]
fn test_field_visibility_pub() {
    let c = hidden_fields::new();
    assert_eq(c.visible, 0);
    let c2 = hidden_fields::bump(c);
    assert_eq(c2.visible, 1);
    assert_eq(hidden_fields::count(c2), 1);
}

// --- pub ---
mod crate_only {
    pub fn shared() -> int { 99 }
    pub fn calls_shared() -> int { shared() }
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
    let c = shapes::Color::Green;
    assert_eq(shapes::name(c), "green");
}

// --- enum via use ---
use shapes::Color;

#[test]
fn test_enum_via_use() {
    let c = Color::Red;
    assert_eq(shapes::name(c), "red");
}
