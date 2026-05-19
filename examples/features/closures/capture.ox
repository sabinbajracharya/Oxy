// === Feature: Closures — Capture ===
// Closures capture variables from their enclosing scope. Value capture
// copies the value; mutable capture uses Cell-wrapping so writes are
// visible to the outer scope.

// === Value Capture ===

#[test]
fn test_value_capture() {
    let factor = 10;
    let multiply = |x| x * factor;
    assert_eq!(multiply(5), 50);
}

// === Multiple Captures ===

#[test]
fn test_multiple_captures() {
    let a = 10;
    let b = 20;
    let sum = || a + b;
    assert_eq!(sum(), 30);
}

// === Mutable Capture ===

#[test]
fn test_mutable_capture() {
    let mut count = 0;
    let inc = || {
        count = count + 1;
    };
    inc();
    inc();
    inc();
    assert_eq!(count, 3);
}

// === Capture with Initial Value ===

#[test]
fn test_capture_with_param_and_mut() {
    let mut total = 100;
    let add = |x| total = total + x;
    add(50);
    assert_eq!(total, 150);
}


// === Multiple Closures Capture Same Variable ===

#[test]
fn test_multiple_closures_same_capture() {
    let mut counter = 0;
    let inc = || { counter = counter + 1; };
    let dec = || { counter = counter - 1; };
    inc();
    inc();
    assert_eq!(counter, 2);
    dec();
    assert_eq!(counter, 1);
}

// === Closure Captured in Loop ===

#[test]
fn test_capture_in_loop() {
    let mut captured = 0;
    let mut i = 0;
    while i < 5 {
        let val = i;
        let closure = || val;
        captured = captured + closure();
        i = i + 1;
    }
    assert_eq!(captured, 10);
}

// === Nested Closure Capture ===

#[test]
fn test_nested_closure() {
    let outer_val = 10;
    let inner_closure = || {
        let add = |x| x + outer_val;
        add(5)
    };
    assert_eq!(inner_closure(), 15);
}

// === Capture String ===

#[test]
fn test_capture_string() {
    let prefix = "Hello, ";
    let greet = |name| prefix + name;
    assert_eq!(greet("World"), "Hello, World");
}
