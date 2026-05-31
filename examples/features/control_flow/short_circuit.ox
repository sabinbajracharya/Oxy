// === Feature: Control Flow — Short-Circuit Evaluation ===
// `&&` and `||` evaluate lazily: the right operand is only evaluated if
// the left operand doesn't determine the result. This is verified by using
// expressions that would panic if evaluated.

// === && (AND) Short-Circuit ===

#[test]
fn test_and_both_true() {
    assert(true && true);
}

#[test]
fn test_and_left_false() {
    assert(!(false && true));
}

#[test]
fn test_and_both_false() {
    assert(!(false && false));
}

#[test]
fn test_and_truthy_values() {
    assert(1 == 1 && 2 == 2);
}

// === || (OR) Short-Circuit ===

#[test]
fn test_or_both_false() {
    assert(!(false || false));
}

#[test]
fn test_or_left_true() {
    assert(true || false);
}

#[test]
fn test_or_right_true() {
    assert(false || true);
}

#[test]
fn test_or_both_true() {
    assert(true || true);
}

// === Combined && and || ===

#[test]
fn test_combined_short_circuit() {
    assert(true || false && false);
    assert(!(false && true || false));
}

// === && with Comparisons ===

#[test]
fn test_and_with_comparisons() {
    let x = 10;
    assert(x > 5 && x < 20);
    assert(!(x > 5 && x < 8));
}

// === || with Comparisons ===

#[test]
fn test_or_with_comparisons() {
    let x = 10;
    assert(x < 5 || x > 8);
    assert(!(x < 5 || x < 0));
}

// === Nested Short-Circuit ===

#[test]
fn test_nested_and_or() {
    let a = true;
    let b = false;
    let c = true;
    assert((a && c) || b);
    assert(!(a && b && c));
}

// === Short-Circuit with Function Calls ===

#[test]
fn test_short_circuit_skips_function() {
    // When left side is false for &&, right side never executes
    let mut called = false;
    let result = false && {
        called = true;
        true
    };
    assert(!result);
    assert(!called);
}

#[test]
fn test_or_short_circuit_skips_function() {
    // When left side is true for ||, right side never executes
    let mut called = false;
    let result = true || {
        called = true;
        false
    };
    assert(result);
    assert(!called);
}

// === Multiple Short-Circuits in Sequence ===

#[test]
fn test_chain_of_ands() {
    let x = 15;
    assert(x > 0 && x < 100 && x != 50 && x > 10);
    assert(!(x > 0 && x < 100 && x == 15 && x > 20));
}

#[test]
fn test_chain_of_ors() {
    let x = 5;
    assert(x == 0 || x == 5 || x == 10);
    assert(!(x == 1 || x == 2 || x == 3));
}
