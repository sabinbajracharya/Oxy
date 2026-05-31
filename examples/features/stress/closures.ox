// === STRESS: closures — capture, args, higher-order, recursion ===

// --- nullary closure ---
#[test]
fn test_closure_no_args() {
    val f = || 42;
    assert_eq(f(), 42);
}

// --- closure capturing local by value ---
#[test]
fn test_closure_capture_int() {
    val n = 10;
    val f = || n * 2;
    assert_eq(f(), 20);
}

// --- closure capturing String ---
#[test]
fn test_closure_capture_string() {
    val greeting = "hello".to_string();
    val f = || format("{}, world", greeting);
    assert_eq(f(), "hello, world");
}

// --- closure with one arg ---
#[test]
fn test_closure_one_arg() {
    val inc = |x: Int| x + 1;
    assert_eq(inc(4), 5);
}

// --- closure with two args ---
#[test]
fn test_closure_two_args() {
    val add = |a: Int, b: Int| a + b;
    assert_eq(add(3, 4), 7);
}

// --- closure capturing & taking arg ---
#[test]
fn test_closure_capture_plus_arg() {
    val base = 100;
    val add_base = |x: Int| x + base;
    assert_eq(add_base(5), 105);
}

// --- closure with multi-statement body ---
#[test]
fn test_closure_block_body() {
    val f = |x: Int| {
        val doubled = x * 2;
        val plus_one = doubled + 1;
        plus_one
    };
    assert_eq(f(7), 15);
}

// --- closure (explicit) ---
#[test]
fn test_closure() {
    val v = [1, 2, 3];
    val f = || v.len();
    assert_eq(f(), 3);
}

// --- closure as fn arg ---
fn apply_int(f: fn(Int) -> Int, x: Int) -> Int { f(x) }

#[test]
fn test_closure_passed_to_fn() {
    val r = apply_int(|x| x * x, 5);
    assert_eq(r, 25);
}

// --- closure returned from fn (no captures) ---
fn make_adder() -> fn(Int, Int) -> Int { |a, b| a + b }

#[test]
fn test_closure_returned_no_captures() {
    val add = make_adder();
    assert_eq(add(2, 3), 5);
}

// --- closure with explicit return type ---
#[test]
fn test_closure_explicit_return_type() {
    val f = |x: Int| -> Int { x + 10 };
    assert_eq(f(5), 15);
}

// --- closure inside an if branch ---
#[test]
fn test_closure_in_if_branch() {
    val n = 5;
    val f = if n > 0 { |x: Int| x + 1 } else { |x: Int| x - 1 };
    assert_eq(f(10), 11);
}

// --- closure in vec.map ---
#[test]
fn test_closure_in_vec_map() {
    val r: List<Int> = [1, 2, 3].iter().map(|x| x * x).collect();
    assert_eq(r, [1, 4, 9]);
}

// --- closure in vec.filter ---
#[test]
fn test_closure_in_vec_filter() {
    val r: List<Int> = [1, 2, 3, 4, 5].iter().filter(|x| x % 2 == 0).collect();
    assert_eq(r, [2, 4]);
}

// --- closure in vec.fold ---
#[test]
fn test_closure_in_vec_fold() {
    val r = [1, 2, 3, 4].iter().fold(0, |acc, x| acc + x);
    assert_eq(r, 10);
}

// --- chained map+filter+collect ---
#[test]
fn test_closure_chained_iter() {
    val r: List<Int> = [1, 2, 3, 4, 5, 6]
        .iter()
        .map(|x| x * 2)
        .filter(|x| x > 4)
        .collect();
    assert_eq(r, [6, 8, 10, 12]);
}

// --- closure capturing mutable ---
#[test]
fn test_closure_modifies_capture() {
    var count = 0;
    var inc = || { count = count + 1; };
    inc();
    inc();
    inc();
    assert_eq(count, 3);
}

// --- closure returning Option ---
#[test]
fn test_closure_returning_option() {
    val safe_div = |a: Int, b: Int| -> Option<Int> {
        if b == 0 { None } else { Some(a / b) }
    };
    assert_eq(safe_div(10, 2), Some(5));
    assert_eq(safe_div(10, 0), None);
}

// --- closure inside closure ---
#[test]
fn test_nested_closure() {
    val outer = |x: Int| {
        val inner = |y: Int| y + 1;
        inner(x) * 2
    };
    assert_eq(outer(3), 8);  // (3+1)*2
}

// --- closure used twice with different args ---
#[test]
fn test_closure_called_twice() {
    val sq = |x: Int| x * x;
    assert_eq(sq(3), 9);
    assert_eq(sq(7), 49);
}

// --- closure capturing struct field ---
struct Counter { count: Int }

#[test]
fn test_closure_capturing_struct() {
    val c = Counter { count: 42 };
    val f = || c.count + 1;
    assert_eq(f(), 43);
}
