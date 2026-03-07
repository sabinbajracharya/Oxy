// fibonacci.ox — Recursive Fibonacci in Oxide
fn fib(n: i64) -> i64 {
    if n <= 1 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

fn main() {
    println!("Fibonacci sequence:");
    println!("fib(0) = {}", fib(0));
    println!("fib(1) = {}", fib(1));
    println!("fib(2) = {}", fib(2));
    println!("fib(5) = {}", fib(5));
    println!("fib(10) = {}", fib(10));
}
