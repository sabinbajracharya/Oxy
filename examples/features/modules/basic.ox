// === Feature: Modules — Basic Inline Modules and Use Imports ===
// Tests for: inline mod blocks, use imports (simple, group, glob, as),
// qualified path calls, nested modules.

mod calculator {
    pub fn add(a: int, b: int) -> int {
        a + b
    }

    pub fn sub(a: int, b: int) -> int {
        a - b
    }
}

#[test]
fn test_simple_import() {
    use calculator::add;
    assert_eq(add(3, 4), 7);
}

#[test]
fn test_group_import() {
    use calculator::{add, sub};
    assert_eq(add(10, 5), 15);
    assert_eq(sub(10, 5), 5);
}

#[test]
fn test_glob_import() {
    use calculator::*;
    assert_eq(add(1, 2), 3);
    assert_eq(sub(7, 3), 4);
}

#[test]
fn test_use_as_rename() {
    use calculator::add as plus;
    assert_eq(plus(2, 3), 5);
}

#[test]
fn test_qualified_path_call() {
    assert_eq(calculator::add(3, 4), 7);
}

// === Structs inside modules ===

mod shapes {
    pub struct Point {
        pub x: float,
        pub y: float,
    }

    pub fn make_point(x: float, y: float) -> Point {
        Point { x, y }
    }
}

#[test]
fn test_struct_in_module() {
    use shapes::Point;
    let p = Point { x: 1.0, y: 2.0 };
    assert_eq(p.x, 1.0);
    assert_eq(p.y, 2.0);
}

#[test]
fn test_struct_via_qualified_path() {
    let p = shapes::make_point(3.0, 4.0);
    assert_eq(p.x, 3.0);
    assert_eq(p.y, 4.0);
}

// === Enums inside modules ===

mod colors {
    pub enum Color {
        Red,
        Green,
        Blue,
    }

    pub fn is_red(c: Color) -> bool {
        match c {
            Color::Red => true,
            _ => false,
        }
    }
}

#[test]
fn test_enum_in_module() {
    use colors::Color;
    let c = Color::Red;
    assert_eq(colors::is_red(c), true);
}

// === Nested modules ===

mod outer {
    pub mod inner {
        pub fn greet() -> String {
            "hello from inner".to_string()
        }
    }
}

#[test]
fn test_nested_module() {
    assert_eq(outer::inner::greet(), "hello from inner");
}

#[test]
fn test_use_from_nested() {
    use outer::inner::greet;
    assert_eq(greet(), "hello from inner");
}

// === Module with impl blocks ===

mod counter {
    pub struct Counter {
        pub count: int,
    }

    impl Counter {
        pub fn new() -> Counter {
            Counter { count: 0 }
        }

        pub fn inc(self) -> Counter {
            Counter { count: self.count + 1 }
        }
    }
}

#[test]
fn test_impl_in_module() {
    use counter::Counter;
    let c = Counter::new();
    let c2 = c.inc();
    assert_eq(c2.count, 1);
}
