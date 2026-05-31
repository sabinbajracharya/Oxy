// === Feature: Closures — Higher-Order Functions ===
// Custom functions that accept or return closures. Demonstrates
// callbacks, combinators, and closure-based patterns.

// === Custom map function ===

fn my_map(v: List<Int>, f: fn(Int) -> Int) -> List<Int> {
    let mut result = list();
    let mut i = 0;
    while i < v.len() {
        result.push(f(v[i]));
        i = i + 1;
    }
    result
}

#[test]
fn test_custom_map() {
    let v = list(1, 2, 3);
    let doubled = my_map(v, |x| x * 2);
    assert_eq(doubled.len(), 3);
}

// === Custom filter ===

fn my_filter(v: List<Int>, pred: fn(Int) -> bool) -> List<Int> {
    let mut result = list();
    let mut i = 0;
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
    let v = list(1, 2, 3, 4, 5, 6);
    let evens = my_filter(v, |x| x % 2 == 0);
    assert_eq(evens.len(), 3);
}

// === Custom fold ===

fn my_fold(v: List<Int>, init: Int, f: fn(Int, Int) -> Int) -> Int {
    let mut acc = init;
    let mut i = 0;
    while i < v.len() {
        acc = f(acc, v[i]);
        i = i + 1;
    }
    acc
}

#[test]
fn test_custom_fold() {
    let v = list(1, 2, 3, 4, 5);
    let sum = my_fold(v, 0, |acc, x| acc + x);
    assert_eq(sum, 15);
}

// === Closure That Returns a Closure ===

fn compose(f: fn(Int) -> Int, g: fn(Int) -> Int) -> fn(Int) -> Int {
    |x| f(g(x))
}

#[test]
fn test_compose() {
    let double = |x: Int| x * 2;
    let add_one = |x: Int| x + 1;
    let double_then_add = compose(add_one, double);
    assert_eq(double_then_add(5), 11); // (5*2)+1
}

// === Callback Pattern ===

fn with_callback(cb: fn(Int) -> String) -> String {
    cb(42)
}

#[test]
fn test_callback() {
    let result = with_callback(|x| "value: " + x);
    assert(result.contains("42"));
}
