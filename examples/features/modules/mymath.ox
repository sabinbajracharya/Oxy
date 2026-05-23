// Supporting module for file_modules.ox — loaded via `mod mymath;`
// This file must be valid standalone (no external dependencies).

pub fn add(a: int, b: int) -> int {
    a + b
}

pub fn sub(a: int, b: int) -> int {
    a - b
}

pub fn fib(n: int) -> int {
    if n <= 1 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

pub struct Point {
    pub x: float,
    pub y: float,
}

pub enum Operation {
    Add(int, int),
    Mul(int, int),
}

pub fn execute(op: Operation) -> int {
    match op {
        Operation::Add(a, b) => a + b,
        Operation::Mul(a, b) => a * b,
    }
}
