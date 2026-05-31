// === Feature: Closures — As Values ===
// Closures are first-class values: they can be stored in variables,
// passed to functions, and returned from functions.

// === Store in Variable ===

#[test]
fn test_store_in_variable() {
    let add = |x, y| x + y;
    assert_eq(add(10, 20), 30);
}

// === Reassign Variable ===

#[test]
fn test_reassign_variable() {
    let mut op = |x| x + 1;
    let r1 = op(5);
    op = |x| x * 2;
    let r2 = op(5);
    assert_eq(r1, 6);
    assert_eq(r2, 10);
}

// === Pass as Argument to Function ===

fn apply_twice(f: fn(int) -> int, x: int) -> int {
    f(f(x))
}

#[test]
fn test_pass_as_argument() {
    let double = |x: int| x * 2;
    assert_eq(apply_twice(double, 5), 20);
}

// === Return from Function ===

fn make_adder(n: int) -> fn(int) -> int {
    |x| x + n
}

#[test]
fn test_return_from_function() {
    let add_five = make_adder(5);
    assert_eq(add_five(10), 15);
    assert_eq(add_five(100), 105);
}

// === Multiple Closures from Same Factory ===

fn make_multiplier(factor: int) -> fn(int) -> int {
    |x| x * factor
}

#[test]
fn test_closure_factory() {
    let double = make_multiplier(2);
    let triple = make_multiplier(3);
    assert_eq(double(10), 20);
    assert_eq(triple(10), 30);
}

// === Closure Stored in Vec ===

#[test]
fn test_closure_in_vec() {
    let add_one = |x: int| x + 1;
    let double = |x: int| x * 2;
    let ops = vec(add_one, double);
    let result = ops[0](10);
    assert_eq(result, 11);
}
