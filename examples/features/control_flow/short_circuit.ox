// === Feature: Control Flow — Short-Circuit Evaluation ===
// `&&` and `||` evaluate lazily: the right operand is only evaluated if
// the left operand doesn't determine the result. This is verified by using
// expressions that would panic if evaluated.

// === && (AND) Short-Circuit ===

#[test]
fn test_and_both_true() {
    assert::true(true && true);
}

#[test]
fn test_and_left_false() {
    assert::true(!(false && true));
}

#[test]
fn test_and_both_false() {
    assert::true(!(false && false));
}

#[test]
fn test_and_truthy_values() {
    assert::true(1 == 1 && 2 == 2);
}

// === || (OR) Short-Circuit ===

#[test]
fn test_or_both_false() {
    assert::true(!(false || false));
}

#[test]
fn test_or_left_true() {
    assert::true(true || false);
}

#[test]
fn test_or_right_true() {
    assert::true(false || true);
}

#[test]
fn test_or_both_true() {
    assert::true(true || true);
}

// === Combined && and || ===

#[test]
fn test_combined_short_circuit() {
    assert::true(true || false && false);
    assert::true(!(false && true || false));
}

// === && with Comparisons ===

#[test]
fn test_and_with_comparisons() {
    val x = 10;
    assert::true(x > 5 && x < 20);
    assert::true(!(x > 5 && x < 8));
}

// === || with Comparisons ===

#[test]
fn test_or_with_comparisons() {
    val x = 10;
    assert::true(x < 5 || x > 8);
    assert::true(!(x < 5 || x < 0));
}

// === Nested Short-Circuit ===

#[test]
fn test_nested_and_or() {
    val a = true;
    val b = false;
    val c = true;
    assert::true((a && c) || b);
    assert::true(!(a && b && c));
}

// === Short-Circuit with Function Calls ===

#[test]
fn test_short_circuit_skips_function() {
    // When left side is false for &&, right side never executes
    var called = false;
    val result = false && {
        called = true;
        true
    };
    assert::true(!result);
    assert::true(!called);
}

#[test]
fn test_or_short_circuit_skips_function() {
    // When left side is true for ||, right side never executes
    var called = false;
    val result = true || {
        called = true;
        false
    };
    assert::true(result);
    assert::true(!called);
}

// === Multiple Short-Circuits in Sequence ===

#[test]
fn test_chain_of_ands() {
    val x = 15;
    assert::true(x > 0 && x < 100 && x != 50 && x > 10);
    assert::true(!(x > 0 && x < 100 && x == 15 && x > 20));
}

#[test]
fn test_chain_of_ors() {
    val x = 5;
    assert::true(x == 0 || x == 5 || x == 10);
    assert::true(!(x == 1 || x == 2 || x == 3));
}
