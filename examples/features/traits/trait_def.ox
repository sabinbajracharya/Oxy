// === Feature: Traits — Trait Definition and Implementation ===
// Define traits with method signatures and implement them on structs.
// Trait methods are called via `.method()` syntax like inherent methods.

// === Basic Trait: Single Method ===

trait Speak {
    fn speak(self) -> String;
}

struct Dog {
    name: String,
}

impl Speak for Dog {
    fn speak(self) -> String {
        "Woof! I'm " + self.name
    }
}

#[test]
fn test_basic_trait() {
    val d = Dog { name: "Rex" };
    assert_eq(d.speak(), "Woof! I'm Rex");
}

// === Trait on Enum ===

trait Area {
    fn area(self) -> Float;
}

enum Shape {
    Circle(Float),
    Rectangle(Float, Float),
}

impl Area for Shape {
    fn area(self) -> Float {
        match self {
            Shape::Circle(r) => 3.14 * r * r,
            Shape::Rectangle(w, h) => w * h,
        }
    }
}

#[test]
fn test_trait_on_enum() {
    val c = Shape::Circle(10.0);
    val r = Shape::Rectangle(4.0, 5.0);
    assert_eq(c.area(), 314.0);
    assert_eq(r.area(), 20.0);
}

// === Trait with Multiple Methods ===

trait Calculator {
    fn add(self, other: Self) -> Self;
    fn sub(self, other: Self) -> Self;
}

struct Num(Int);

impl Calculator for Num {
    fn add(self, other: Num) -> Num {
        Num(self.0 + other.0)
    }

    fn sub(self, other: Num) -> Num {
        Num(self.0 - other.0)
    }
}

#[test]
fn test_multiple_trait_methods() {
    val a = Num(10);
    val b = Num(3);
    assert_eq(a.add(b).0, 13);
    val c = Num(10);
    val d = Num(3);
    assert_eq(c.sub(d).0, 7);
}

// === Trait with self Receiver ===

trait Describe {
    fn describe(self) -> String;
    fn tag_line(self) -> String;
}

struct Book {
    title: String,
    year: Int,
}

impl Describe for Book {
    fn describe(self) -> String {
        self.title + " (" + self.year + ")"
    }
    fn tag_line(self) -> String {
        "A great read: " + self.describe()
    }
}

#[test]
fn test_self_receiver() {
    val b = Book { title: "Oxy Guide", year: 2025 };
    assert_eq(b.describe(), "Oxy Guide (2025)");
    assert_eq(b.tag_line(), "A great read: Oxy Guide (2025)");
}

// === Chaining Trait Method Calls ===

trait Chain {
    fn double(self) -> Self;
    fn add_ten(self) -> Self;
}

impl Chain for Int {
    fn double(self) -> Int {
        self * 2
    }
    fn add_ten(self) -> Int {
        self + 10
    }
}

#[test]
fn test_trait_method_chain() {
    val x: Int = 5;
    assert_eq(x.double().add_ten(), 20);
    assert_eq(x.add_ten().double(), 30);
}
