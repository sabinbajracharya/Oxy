// === Feature: Traits — Default Methods and Inheritance ===
// Traits can provide default method implementations that impl blocks
// can inherit or override.

// === Basic Default Method ===

trait Greet {
    fn greeting(self) -> String {
        "Hello!".to_string()
    }
}

struct Person {
    name: String,
}

impl Greet for Person {}

#[test]
fn test_default_method_inherited() {
    let p = Person { name: "Alice" };
    assert_eq!(p.greeting(), "Hello!");
}

// === Override Default Method ===

trait Descriptor {
    fn describe(self) -> String {
        "a thing".to_string()
    }
    fn label(self) -> String {
        "Label: ".to_string() + self.describe()
    }
}

struct Widget {
    kind: String,
}

impl Descriptor for Widget {
    fn describe(self) -> String {
        "a " + self.kind
    }
}

#[test]
fn test_override_default() {
    let w = Widget { kind: "gadget" };
    assert_eq!(w.describe(), "a gadget");
    // label() is NOT overridden — uses default, which calls overridden describe()
    assert_eq!(w.label(), "Label: a gadget");
}

// === Default Method Calls Another Default ===

trait Calculator {
    fn add(self, other: i64) -> i64 {
        self.value() + other
    }
    fn value(self) -> i64 {
        0
    }
}

struct Counter {
    count: i64,
}

impl Calculator for Counter {
    fn value(self) -> i64 {
        self.count
    }
}

#[test]
fn test_default_calls_other_method() {
    let c = Counter { count: 10 };
    assert_eq!(c.value(), 10);
    assert_eq!(c.add(5), 15);
}

// === Multiple Traits with Default Methods ===

trait A {
    fn a(self) -> i64 {
        10
    }
}

trait B {
    fn b(self) -> i64 {
        20
    }
}

struct Thing;

impl A for Thing {}
impl B for Thing {}

#[test]
fn test_multiple_defaults() {
    let t = Thing;
    assert_eq!(t.a(), 10);
    assert_eq!(t.b(), 20);
}
