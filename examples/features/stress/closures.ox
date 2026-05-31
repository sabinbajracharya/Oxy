// === STRESS: closures — capture, args, higher-order, recursion ===

// --- nullary closure ---
#[test]
fn test_closure_no_args() {
    let f = || 42;
    assert_eq(f(), 42);
}

// --- closure capturing local by value ---
#[test]
fn test_closure_capture_int() {
    let n = 10;
    let f = || n * 2;
    assert_eq(f(), 20);
}

// --- closure capturing String ---
#[test]
fn test_closure_capture_string() {
    let greeting = "hello".to_string();
    let f = || format("{}, world", greeting);
    assert_eq(f(), "hello, world");
}

// --- closure with one arg ---
#[test]
fn test_closure_one_arg() {
    let inc = |x: int| x + 1;
    assert_eq(inc(4), 5);
}

// --- closure with two args ---
#[test]
fn test_closure_two_args() {
    let add = |a: int, b: int| a + b;
    assert_eq(add(3, 4), 7);
}

// --- closure capturing & taking arg ---
#[test]
fn test_closure_capture_plus_arg() {
    let base = 100;
    let add_base = |x: int| x + base;
    assert_eq(add_base(5), 105);
}

// --- closure with multi-statement body ---
#[test]
fn test_closure_block_body() {
    let f = |x: int| {
        let doubled = x * 2;
        let plus_one = doubled + 1;
        plus_one
    };
    assert_eq(f(7), 15);
}

// --- closure (explicit) ---
#[test]
fn test_closure() {
    let v = vec(1, 2, 3);
    let f = || v.len();
    assert_eq(f(), 3);
}

// --- closure as fn arg ---
fn apply_int(f: fn(int) -> int, x: int) -> int { f(x) }

#[test]
fn test_closure_passed_to_fn() {
    let r = apply_int(|x| x * x, 5);
    assert_eq(r, 25);
}

// --- closure returned from fn (no captures) ---
fn make_adder() -> fn(int, int) -> int { |a, b| a + b }

#[test]
fn test_closure_returned_no_captures() {
    let add = make_adder();
    assert_eq(add(2, 3), 5);
}

// --- closure with explicit return type ---
#[test]
fn test_closure_explicit_return_type() {
    let f = |x: int| -> int { x + 10 };
    assert_eq(f(5), 15);
}

// --- closure inside an if branch ---
#[test]
fn test_closure_in_if_branch() {
    let n = 5;
    let f = if n > 0 { |x: int| x + 1 } else { |x: int| x - 1 };
    assert_eq(f(10), 11);
}

// --- closure in vec.map ---
#[test]
fn test_closure_in_vec_map() {
    let r: Vec<int> = vec(1, 2, 3).iter().map(|x| x * x).collect();
    assert_eq(r, vec(1, 4, 9));
}

// --- closure in vec.filter ---
#[test]
fn test_closure_in_vec_filter() {
    let r: Vec<int> = vec(1, 2, 3, 4, 5).iter().filter(|x| x % 2 == 0).collect();
    assert_eq(r, vec(2, 4));
}

// --- closure in vec.fold ---
#[test]
fn test_closure_in_vec_fold() {
    let r = vec(1, 2, 3, 4).iter().fold(0, |acc, x| acc + x);
    assert_eq(r, 10);
}

// --- chained map+filter+collect ---
#[test]
fn test_closure_chained_iter() {
    let r: Vec<int> = vec(1, 2, 3, 4, 5, 6)
        .iter()
        .map(|x| x * 2)
        .filter(|x| x > 4)
        .collect();
    assert_eq(r, vec(6, 8, 10, 12));
}

// --- closure capturing mutable ---
#[test]
fn test_closure_modifies_capture() {
    let mut count = 0;
    let mut inc = || { count = count + 1; };
    inc();
    inc();
    inc();
    assert_eq(count, 3);
}

// --- closure returning Option ---
#[test]
fn test_closure_returning_option() {
    let safe_div = |a: int, b: int| -> Option<int> {
        if b == 0 { None } else { Some(a / b) }
    };
    assert_eq(safe_div(10, 2), Some(5));
    assert_eq(safe_div(10, 0), None);
}

// --- closure inside closure ---
#[test]
fn test_nested_closure() {
    let outer = |x: int| {
        let inner = |y: int| y + 1;
        inner(x) * 2
    };
    assert_eq(outer(3), 8);  // (3+1)*2
}

// --- closure used twice with different args ---
#[test]
fn test_closure_called_twice() {
    let sq = |x: int| x * x;
    assert_eq(sq(3), 9);
    assert_eq(sq(7), 49);
}

// --- closure capturing struct field ---
struct Counter { count: int }

#[test]
fn test_closure_capturing_struct() {
    let c = Counter { count: 42 };
    let f = || c.count + 1;
    assert_eq(f(), 43);
}
