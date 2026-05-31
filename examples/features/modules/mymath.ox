// Supporting module for file_modules.ox — loaded via `mod mymath;`
// This file must be valid standalone (no external dependencies).

pub fn add(a: Int, b: Int) -> Int {
    a + b
}

pub fn sub(a: Int, b: Int) -> Int {
    a - b
}

pub fn fib(n: Int) -> Int {
    if n <= 1 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

pub struct Point {
    pub x: Float,
    pub y: Float,
}

pub enum Operation {
    Add(Int, Int),
    Mul(Int, Int),
}

pub fn execute(op: Operation) -> Int {
    match op {
        Operation::Add(a, b) => a + b,
        Operation::Mul(a, b) => a * b,
    }
}
