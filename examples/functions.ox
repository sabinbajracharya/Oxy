// functions.ox — Functions, recursion, and tail expressions
fn add(a: Int, b: Int) -> Int {
    a + b
}

fn multiply(a: Int, b: Int) -> Int {
    a * b
}

fn factorial(n: Int) -> Int {
    if n <= 1 {
        return 1;
    }
    n * factorial(n - 1)
}

fn greet(name: String) {
    io::println("Hello, {}!", name);
}

fn main() {
    io::println("{}", add(3, 4));
    io::println("{}", multiply(5, 6));
    io::println("5! = {}", factorial(5));
    greet("Oxy");
}
