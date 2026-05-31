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
    val c = Color::Red;
    val result = match c {
        Color::Red => "red",
        Color::Green => "green",
        Color::Blue => "blue",
    };
    assert_eq(result, "red");
}

// === Enum with Tuple Variants ===

enum Shape {
    Circle(Float),
    Rectangle(Float, Float),
}

#[test]
fn test_enum_tuple_variant() {
    val s = Shape::Circle(5.0);
    val area = match s {
        Shape::Circle(r) => 3.14 * r * r,
        Shape::Rectangle(w, h) => w * h,
    };
    assert(area > 0.0);
}

#[test]
fn test_enum_match_rectangle() {
    val s = Shape::Rectangle(4.0, 5.0);
    val area = match s {
        Shape::Circle(r) => 3.14 * r * r,
        Shape::Rectangle(w, h) => w * h,
    };
    assert_eq(area, 20.0);
}

// === Enum with Struct Variants ===

enum Message {
    Quit,
    Move { x: Int, y: Int },
}

#[test]
fn test_enum_struct_variant() {
    val msg = Message::Move { x: 10, y: 20 };
    val result = match msg {
        Message::Quit => "quit",
        Message::Move { x, y } => "move",
    };
    assert_eq(result, "move");
}

// === Enum Multiple Variants ===

enum Status {
    Pending,
    Active,
    Done,
}

#[test]
fn test_enum_exhaustive_match() {
    val s = Status::Active;
    val code = match s {
        Status::Pending => 0,
        Status::Active => 1,
        Status::Done => 2,
    };
    assert_eq(code, 1);
}

// === Enum with Wildcard Match ===

#[test]
fn test_enum_wildcard_match() {
    val c = Color::Green;
    val is_red = match c {
        Color::Red => true,
        _ => false,
    };
    assert(!is_red);
}
