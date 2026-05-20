// Supporting module for file_modules.ox — loaded via `mod mymath;`
// This file must be valid standalone (no external dependencies).

pub fn add(a: i64, b: i64) -> i64 {
    a + b
}

pub fn sub(a: i64, b: i64) -> i64 {
    a - b
}

pub fn fib(n: i64) -> i64 {
    if n <= 1 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub enum Operation {
    Add(i64, i64),
    Mul(i64, i64),
}

pub fn execute(op: Operation) -> i64 {
    match op {
        Operation::Add(a, b) => a + b,
        Operation::Mul(a, b) => a * b,
    }
}
