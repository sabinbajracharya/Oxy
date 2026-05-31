// === Feature: Closures — Higher-Order Functions ===
// Custom functions that accept or return closures. Demonstrates
// callbacks, combinators, and closure-based patterns.

// === Custom map function ===

fn my_map(v: List<Int>, f: fn(Int) -> Int) -> List<Int> {
    var result = [];
    var i = 0;
    while i < v.len() {
        result.push(f(v[i]));
        i = i + 1;
    }
    result
}

#[test]
fn test_custom_map() {
    val v = [1, 2, 3];
    val doubled = my_map(v, |x| x * 2);
    assert_eq(doubled.len(), 3);
}

// === Custom filter ===

fn my_filter(v: List<Int>, pred: fn(Int) -> bool) -> List<Int> {
    var result = [];
    var i = 0;
    while i < v.len() {
        if pred(v[i]) {
            result.push(v[i]);
        }
        i = i + 1;
    }
    result
}

#[test]
fn test_custom_filter() {
    val v = [1, 2, 3, 4, 5, 6];
    val evens = my_filter(v, |x| x % 2 == 0);
    assert_eq(evens.len(), 3);
}

// === Custom fold ===

fn my_fold(v: List<Int>, init: Int, f: fn(Int, Int) -> Int) -> Int {
    var acc = init;
    var i = 0;
    while i < v.len() {
        acc = f(acc, v[i]);
        i = i + 1;
    }
    acc
}

#[test]
fn test_custom_fold() {
    val v = [1, 2, 3, 4, 5];
    val sum = my_fold(v, 0, |acc, x| acc + x);
    assert_eq(sum, 15);
}

// === Closure That Returns a Closure ===

fn compose(f: fn(Int) -> Int, g: fn(Int) -> Int) -> fn(Int) -> Int {
    |x| f(g(x))
}

#[test]
fn test_compose() {
    val double = |x: Int| x * 2;
    val add_one = |x: Int| x + 1;
    val double_then_add = compose(add_one, double);
    assert_eq(double_then_add(5), 11); // (5*2)+1
}

// === Callback Pattern ===

fn with_callback(cb: fn(Int) -> String) -> String {
    cb(42)
}

#[test]
fn test_callback() {
    val result = with_callback(|x| "value: " + x);
    assert(result.contains("42"));
}
