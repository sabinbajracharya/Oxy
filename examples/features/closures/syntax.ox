// === Feature: Closures — Syntax ===
// Closures are anonymous functions defined with |params| body syntax.
// Supports: single/multi params, type annotations, return types,
// block bodies, empty closures, move closures.

// === Single Parameter ===

#[test]
fn test_single_param() {
    let double = |x| x * 2;
    assert_eq!(double(5), 10);
}

// === Multiple Parameters ===

#[test]
fn test_multiple_params() {
    let add = |x, y| x + y;
    assert_eq!(add(3, 4), 7);
}

// === Type Annotations ===

#[test]
fn test_type_annotations() {
    let multiply = |x: int, y: int| x * y;
    assert_eq!(multiply(6, 7), 42);
}

// === Return Type Annotation ===

#[test]
fn test_return_type_annotation() {
    let identity = |x: int| -> int { x };
    assert_eq!(identity(99), 99);
}

// === Empty Closure (no params) ===

#[test]
fn test_empty_closure() {
    let answer = || 42;
    assert_eq!(answer(), 42);
}

// === Block Body ===

#[test]
fn test_block_body() {
    let compute = |x: int| -> int {
        let y = x * 2;
        y + 1
    };
    assert_eq!(compute(10), 21);
}

// === Closure Called Immediately ===

#[test]
fn test_immediately_called() {
    let result = (|x, y| x + y)(3, 4);
    assert_eq!(result, 7);
}

// === Move Closure ===

#[test]
fn test_closure() {
    let name = "world";
    let greet = || "hello " + name;
    assert_eq!(greet(), "hello world");
}

// === Closure as Method Argument (inline) ===

#[test]
fn test_closure_inline() {
    let v = vec![1, 2, 3];
    let doubled = v.map(|x| x * 2);
    assert_eq!(doubled.len(), 3);
}
