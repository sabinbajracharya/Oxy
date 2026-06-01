// === STRESS: function as value (named fn passed to higher-order fn) ===

fn apply(f: fn(Int) -> Int, x: Int) -> Int {
    f(x)
}

fn square(x: Int) -> Int { x * x }
fn neg(x: Int) -> Int { -x }

#[test]
fn test_named_fn_as_arg() {
    assert::eq(apply(square, 5), 25);
}

#[test]
fn test_closure_as_arg() {
    assert::eq(apply(|x| x * 2, 10), 20);
}

#[test]
fn test_pass_different_named_fns() {
    assert::eq(apply(square, 4), 16);
    assert::eq(apply(neg, 7), -7);
}

// --- two-arg fn pointer ---
fn apply2(f: fn(Int, Int) -> Int, a: Int, b: Int) -> Int { f(a, b) }
fn add(a: Int, b: Int) -> Int { a + b }
fn mul(a: Int, b: Int) -> Int { a * b }

#[test]
fn test_two_arg_fn_pointer() {
    assert::eq(apply2(add, 3, 4), 7);
    assert::eq(apply2(mul, 3, 4), 12);
}

// --- fn returning fn (closure as return value) ---
fn make_adder(n: Int) -> fn(Int) -> Int {
    |x| x + n
}

#[test]
fn test_fn_returns_closure() {
    val add5 = make_adder(5);
    assert::eq(add5(3), 8);
}

// --- fn stored in a List ---
#[test]
fn test_fns_in_list() {
    val ops: List<fn(Int) -> Int> = [square, neg];
    val r0 = ops[0](6);
    val r1 = ops[1](6);
    assert::eq(r0, 36);
    assert::eq(r1, -6);
}
