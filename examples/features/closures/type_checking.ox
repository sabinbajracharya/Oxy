// === Feature: Closure Type Inference ===
// Tests that the type checker infers closure body types and validates
// return type annotations.

// === Basic Closure, No Annotations ===

#[test]
fn test_closure_no_annotations() {
    let add = |x, y| x + y;
    assert_eq(add(10, 20), 30);
}

// === Void Closure (empty body / no return) ===

#[test]
fn test_closure_empty_body() {
    let mut called = false;
    let set = || { called = true; };
    set();
    assert(called);
}

// === Closure with Return Type Annotation (matching) ===

fn make_increment() -> fn(int) -> int {
    |x| x + 1
}

#[test]
fn test_closure_with_matching_return_annotation() {
    let inc = make_increment();
    assert_eq(inc(5), 6);
}

// === Closure with Param Type Annotations ===

#[test]
fn test_closure_with_param_type_annotations() {
    let mul = |x: int, y: int| x * y;
    assert_eq(mul(3, 4), 12);
}

// === Closure with Both Param and Return Type Annotations ===

fn make_doubler() -> fn(int) -> int {
    |x: int| -> int { x * 2 }
}

#[test]
fn test_closure_with_both_annotations() {
    let double = make_doubler();
    assert_eq(double(7), 14);
}

// === Nested Closure ===

#[test]
fn test_nested_closure() {
    let outer = |x: int| {
        let inner = |y: int| x + y;
        inner(x)
    };
    assert_eq(outer(5), 10);
}

// === Store in Variable with Fn Type Annotation ===

#[test]
fn test_store_in_variable_with_fn_annotation() {
    let op: Fn = |x: int| -> int { x + 10 };
    assert_eq(op(3), 13);
}

// === Return Type Mismatch — should be a compile error ===

#[compile_error]
fn test_closure_return_type_mismatch_on_variable() {
    // Variable declared as fn(int) -> bool, but closure returns int
    let f: fn(int) -> bool = |x: int| x + 1;
}

#[compile_error]
fn test_closure_own_return_type_mismatch() {
    // Closure declares -> bool but body returns int
    let _ = || -> bool { 42 };
}
