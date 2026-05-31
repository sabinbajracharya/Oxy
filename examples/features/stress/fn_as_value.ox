// === STRESS: function as value (named fn passed to higher-order fn) ===

fn apply(f: fn(int) -> int, x: int) -> int {
    f(x)
}

fn square(x: int) -> int { x * x }
fn neg(x: int) -> int { -x }

#[test]
fn test_named_fn_as_arg() {
    assert_eq!(apply(square, 5), 25);
}

#[test]
fn test_closure_as_arg() {
    assert_eq!(apply(|x| x * 2, 10), 20);
}

#[test]
fn test_pass_different_named_fns() {
    assert_eq!(apply(square, 4), 16);
    assert_eq!(apply(neg, 7), -7);
}

// --- two-arg fn pointer ---
fn apply2(f: fn(int, int) -> int, a: int, b: int) -> int { f(a, b) }
fn add(a: int, b: int) -> int { a + b }
fn mul(a: int, b: int) -> int { a * b }

#[test]
fn test_two_arg_fn_pointer() {
    assert_eq!(apply2(add, 3, 4), 7);
    assert_eq!(apply2(mul, 3, 4), 12);
}

// --- fn returning fn (closure as return value) ---
fn make_adder(n: int) -> fn(int) -> int {
    |x| x + n
}

#[test]
fn test_fn_returns_closure() {
    let add5 = make_adder(5);
    assert_eq!(add5(3), 8);
}

// --- fn stored in a Vec ---
#[test]
fn test_fns_in_vec() {
    let ops: Vec<fn(int) -> int> = vec![square, neg];
    let r0 = ops[0](6);
    let r1 = ops[1](6);
    assert_eq!(r0, 36);
    assert_eq!(r1, -6);
}
