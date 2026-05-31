// === Feature: stack discipline of `println` and `match` in recursive fns ===
// Both `println` (and friends) and a `match` used as the tail expression of
// a function used to under-pop the operand stack, which is harmless at the
// top level (Pop on empty is a no-op) but corrupts the *caller's* frame in
// recursive calls. Each call to a function with `println` would silently
// consume a value from its caller's stack, eventually leading to bizarre
// runtime errors like "cannot add () and int" or a VM-level subtract overflow.

fn go(n: int) -> int {
    if n <= 0 { return 0; }
    println("step {}", n);
    go(n - 1) + 1
}

fn pick(n: int) -> int {
    if n <= 0 { return 0; }
    match n {
        1 => 1,
        _ => pick(n - 1) + pick(n - 2),
    }
}

fn classify(n: int) -> int {
    match n {
        0 => 10,
        1 => 20,
        _ => 99,
    }
}

fn fib_match(n: int) -> int {
    if n <= 1 { return n; }
    println("fib({})", n);
    match n {
        2 => 1,
        _ => fib_match(n - 1) + fib_match(n - 2),
    }
}

fn label(n: int) -> String {
    if n <= 0 { return "done".to_string(); }
    let _ = format("{}", n);
    label(n - 1)
}

#[test]
fn test_println_in_recursive_fn() {
    assert_eq(go(3), 3);
}

#[test]
fn test_match_as_tail_in_recursive_fn() {
    assert_eq(pick(5), 5);
}

#[test]
fn test_match_only_no_leading_if() {
    assert_eq(classify(0), 10);
    assert_eq(classify(1), 20);
    assert_eq(classify(7), 99);
}

#[test]
fn test_println_and_match_combined_recursive() {
    assert_eq(fib_match(6), 8);
}

#[test]
fn test_format_arg_in_recursive_fn() {
    assert_eq(label(4), "done");
}
