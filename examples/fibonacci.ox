// fibonacci.ox — Recursive Fibonacci in Oxy
fn fib(n: Int) -> Int {
    if n <= 1 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

fn main() {
    io::println("Fibonacci sequence:");
    io::println("fib(0) = {}", fib(0));
    io::println("fib(1) = {}", fib(1));
    io::println("fib(2) = {}", fib(2));
    io::println("fib(5) = {}", fib(5));
    io::println("fib(10) = {}", fib(10));
}
