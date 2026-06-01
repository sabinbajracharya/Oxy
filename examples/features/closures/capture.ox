// === Feature: Closures — Capture ===
// Closures capture variables from their enclosing scope. Value capture
// copies the value; mutable capture uses Cell-wrapping so writes are
// visible to the outer scope.

// === Value Capture ===

#[test]
fn test_value_capture() {
    val factor = 10;
    val multiply = |x| x * factor;
    assert::eq(multiply(5), 50);
}

// === Multiple Captures ===

#[test]
fn test_multiple_captures() {
    val a = 10;
    val b = 20;
    val sum = || a + b;
    assert::eq(sum(), 30);
}

// === Mutable Capture ===

#[test]
fn test_mutable_capture() {
    var count = 0;
    val inc = || {
        count = count + 1;
    };
    inc();
    inc();
    inc();
    assert::eq(count, 3);
}

// === Capture with Initial Value ===

#[test]
fn test_capture_with_param_and_mut() {
    var total = 100;
    val add = |x| total = total + x;
    add(50);
    assert::eq(total, 150);
}


// === Multiple Closures Capture Same Variable ===

#[test]
fn test_multiple_closures_same_capture() {
    var counter = 0;
    val inc = || { counter = counter + 1; };
    val dec = || { counter = counter - 1; };
    inc();
    inc();
    assert::eq(counter, 2);
    dec();
    assert::eq(counter, 1);
}

// === Closure Captured in Loop ===

#[test]
fn test_capture_in_loop() {
    var captured = 0;
    var i = 0;
    while i < 5 {
        val v = i;
        val closure = || v;
        captured = captured + closure();
        i = i + 1;
    }
    assert::eq(captured, 10);
}

// === Nested Closure Capture ===

#[test]
fn test_nested_closure() {
    val outer_val = 10;
    val inner_closure = || {
        val add = |x| x + outer_val;
        add(5)
    };
    assert::eq(inner_closure(), 15);
}

// === Capture String ===

#[test]
fn test_capture_string() {
    val prefix = "Hello, ";
    val greet = |name| prefix + name;
    assert::eq(greet("World"), "Hello, World");
}
