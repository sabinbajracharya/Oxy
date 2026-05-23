// functions.ox — Functions, recursion, and tail expressions
fn add(a: int, b: int) -> int {
    a + b
}

fn multiply(a: int, b: int) -> int {
    a * b
}

fn factorial(n: int) -> int {
    if n <= 1 {
        return 1;
    }
    n * factorial(n - 1)
}

fn greet(name: String) {
    println!("Hello, {}!", name);
}

fn main() {
    println!("{}", add(3, 4));
    println!("{}", multiply(5, 6));
    println!("5! = {}", factorial(5));
    greet(&"Oxy");
}
