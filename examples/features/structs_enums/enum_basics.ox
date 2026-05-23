// === Feature: Enums — Basics ===
// Enums define types that can be one of several variants. Variants
// can be unit (no data), tuple (positional data), or struct (named data).
// Use `Enum::Variant` to construct, and `match` to destructure.

// === Unit Variants ===

enum Color {
    Red,
    Green,
    Blue,
}

#[test]
fn test_enum_unit_variants() {
    let c = Color::Red;
    let result = match c {
        Color::Red => "red",
        Color::Green => "green",
        Color::Blue => "blue",
    };
    assert_eq!(result, "red");
}

// === Enum with Tuple Variants ===

enum Shape {
    Circle(float),
    Rectangle(float, float),
}

#[test]
fn test_enum_tuple_variant() {
    let s = Shape::Circle(5.0);
    let area = match s {
        Shape::Circle(r) => 3.14 * r * r,
        Shape::Rectangle(w, h) => w * h,
    };
    assert!(area > 0.0);
}

#[test]
fn test_enum_match_rectangle() {
    let s = Shape::Rectangle(4.0, 5.0);
    let area = match s {
        Shape::Circle(r) => 3.14 * r * r,
        Shape::Rectangle(w, h) => w * h,
    };
    assert_eq!(area, 20.0);
}

// === Enum with Struct Variants ===

enum Message {
    Quit,
    Move { x: int, y: int },
}

#[test]
fn test_enum_struct_variant() {
    let msg = Message::Move { x: 10, y: 20 };
    let result = match msg {
        Message::Quit => "quit",
        Message::Move { x, y } => "move",
    };
    assert_eq!(result, "move");
}

// === Enum Multiple Variants ===

enum Status {
    Pending,
    Active,
    Done,
}

#[test]
fn test_enum_exhaustive_match() {
    let s = Status::Active;
    let code = match s {
        Status::Pending => 0,
        Status::Active => 1,
        Status::Done => 2,
    };
    assert_eq!(code, 1);
}

// === Enum with Wildcard Match ===

#[test]
fn test_enum_wildcard_match() {
    let c = Color::Green;
    let is_red = match c {
        Color::Red => true,
        _ => false,
    };
    assert!(!is_red);
}
