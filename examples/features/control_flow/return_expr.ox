// === Feature: Control Flow — Return ===
// `return` exits a function early, optionally with a value. The last
// expression in a function (without semicolon) is an implicit return.

fn add_one(x: Int) -> Int {
    return x + 1;
}

#[test]
fn test_explicit_return() {
    assert_eq(add_one(41), 42);
}

fn early_return(x: Int) -> Int {
    if x < 0 {
        return 0;
    }
    x
}

#[test]
fn test_early_return_guard() {
    assert_eq(early_return(-5), 0);
    assert_eq(early_return(10), 10);
}

fn return_unit(x: Int) {
    if x == 0 {
        return;
    }
    let unused = x;
}

#[test]
fn test_return_unit() {
    return_unit(0);
    return_unit(1);
    assert(true);
}

// Implicit return (last expression, no semicolon)

fn implicit_return(x: Int) -> Int {
    x * 2
}

#[test]
fn test_implicit_return() {
    assert_eq(implicit_return(21), 42);
}

// Return from inside if/else

fn max(a: Int, b: Int) -> Int {
    if a > b {
        return a;
    }
    b
}

#[test]
fn test_return_from_if_else() {
    assert_eq(max(10, 5), 10);
    assert_eq(max(3, 7), 7);
}

// Return from inside loop

fn find_first_even(nums: List<Int>) -> Int {
    for n in nums {
        if n % 2 == 0 {
            return n;
        }
    }
    -1
}

#[test]
fn test_return_from_loop() {
    let nums = list(1, 3, 5, 8, 9);
    assert_eq(find_first_even(nums), 8);
}

// Return from inside match

fn match_return(x: Int) -> String {
    match x {
        0 => return "zero",
        _ => "non-zero",
    }
}

#[test]
fn test_return_from_match() {
    assert_eq(match_return(0), "zero");
    assert_eq(match_return(42), "non-zero");
}

// Return with no value from nested scope

fn bail_if_negative(x: Int) -> Int {
    if x < 0 {
        if x < -100 {
            return -100;
        }
        return -x;
    }
    x
}

#[test]
fn test_nested_returns() {
    assert_eq(bail_if_negative(5), 5);
    assert_eq(bail_if_negative(-5), 5);
    assert_eq(bail_if_negative(-200), -100);
}

// Return from while loop

fn sum_until_limit(limit: Int) -> Int {
    let mut sum = 0;
    let mut i = 0;
    while i < 100 {
        i = i + 1;
        sum = sum + i;
        if sum >= limit {
            return sum;
        }
    }
    sum
}

#[test]
fn test_return_from_while() {
    assert_eq(sum_until_limit(10), 10);
    assert_eq(sum_until_limit(10000), 5050);
}
