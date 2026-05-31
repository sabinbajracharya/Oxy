// === Feature: Traits ===
// Traits define shared behavior. Types implement traits via
// `impl Trait for Type { ... }`. Traits can have default method
// implementations that implementors can override.

// === Basic Trait ===

trait Greet {
    fn greet(self) -> String;
}

struct Person {
    name: String,
}

impl Greet for Person {
    fn greet(self) -> String {
        "Hello, " + self.name
    }
}

#[test]
fn test_trait_basic() {
    let p = Person { name: "Alice" };
    assert_eq(p.greet(), "Hello, Alice");
}

// === Trait with Multiple Methods ===

trait Shape2D {
    fn area(self) -> Float;
    fn perimeter(self) -> Float;
}

struct Square {
    side: Float,
}

impl Shape2D for Square {
    fn area(self) -> Float {
        self.side * self.side
    }

    fn perimeter(self) -> Float {
        4.0 * self.side
    }
}

#[test]
fn test_trait_multiple_methods() {
    let s = Square { side: 3.0 };
    assert_eq(s.area(), 9.0);
    assert_eq(s.perimeter(), 12.0);
}

// === Trait with Default Method ===

trait Describable {
    fn describe(self) -> String {
        "no description".to_string()
    }

    fn type_name(self) -> String;
}

struct Widget {
    id: Int,
}

impl Describable for Widget {
    fn type_name(self) -> String {
        "Widget".to_string()
    }
    // describe() uses the default implementation
}

#[test]
fn test_trait_default_method() {
    let w = Widget { id: 1 };
    assert_eq(w.type_name(), "Widget");
    assert_eq(w.describe(), "no description");
}

// === Multiple Impls for Same Type ===

trait Identifiable {
    fn id(self) -> Int;
}

impl Identifiable for Widget {
    fn id(self) -> Int {
        self.id
    }
}

#[test]
fn test_multiple_trait_impls() {
    let w = Widget { id: 99 };
    assert_eq(w.id(), 99);
    assert_eq(w.type_name(), "Widget");
}
