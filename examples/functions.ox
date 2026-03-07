// functions.ox — Functions, recursion, and tail expressions
fn add(a: i64, b: i64) -> i64 {
    a + b
}

fn multiply(a: i64, b: i64) -> i64 {
    a * b
}

fn factorial(n: i64) -> i64 {
    if n <= 1 {
        return 1;
    }
    n * factorial(n - 1)
}

fn greet(name: &str) {
    println!("Hello, {}!", name);
}

fn main() {
    println!("{}", add(3, 4));
    println!("{}", multiply(5, 6));
    println!("5! = {}", factorial(5));
    greet(&"Oxide");
}
