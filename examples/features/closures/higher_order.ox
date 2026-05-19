// === Feature: Closures — Higher-Order Functions ===
// Custom functions that accept or return closures. Demonstrates
// callbacks, combinators, and closure-based patterns.

// === Custom map function ===

fn my_map(v: Vec<i64>, f: fn(i64) -> i64) -> Vec<i64> {
    let mut result = vec![];
    let mut i = 0;
    while i < v.len() {
        result.push(f(v[i]));
        i = i + 1;
    }
    result
}

#[test]
fn test_custom_map() {
    let v = vec![1, 2, 3];
    let doubled = my_map(v, |x| x * 2);
    assert_eq!(doubled.len(), 3);
}

// === Custom filter ===

fn my_filter(v: Vec<i64>, pred: fn(i64) -> bool) -> Vec<i64> {
    let mut result = vec![];
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
    let v = vec![1, 2, 3, 4, 5, 6];
    let evens = my_filter(v, |x| x % 2 == 0);
    assert_eq!(evens.len(), 3);
}

// === Custom fold ===

fn my_fold(v: Vec<i64>, init: i64, f: fn(i64, i64) -> i64) -> i64 {
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
    let v = vec![1, 2, 3, 4, 5];
    let sum = my_fold(v, 0, |acc, x| acc + x);
    assert_eq!(sum, 15);
}

// === Closure That Returns a Closure ===

fn compose(f: fn(i64) -> i64, g: fn(i64) -> i64) -> fn(i64) -> i64 {
    |x| f(g(x))
}

#[test]
fn test_compose() {
    let double = |x: i64| x * 2;
    let add_one = |x: i64| x + 1;
    let double_then_add = compose(add_one, double);
    assert_eq!(double_then_add(5), 11); // (5*2)+1
}

// === Callback Pattern ===

fn with_callback(cb: fn(i64) -> String) -> String {
    cb(42)
}

#[test]
fn test_callback() {
    let result = with_callback(|x| "value: " + x);
    assert!(result.contains("42"));
}
