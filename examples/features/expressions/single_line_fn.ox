// Single-line function syntax — `fn name(params) [-> Type] = expr`
// Desugars to `fn name(params) [-> Type] { expr }` at parse time.
// Verified via Rust-side type checker tests.

fn double(x: Int) -> Int = x * 2
fn add(a: Int, b: Int) -> Int = a + b
fn multiply(a: Int, b: Int) -> Int = a * b
fn square(x: Int) -> Int = x * x

// No return type annotation — inferred from body
fn greet(name: String) = "Hello, " + name

fn main() {
    io::println("double(21) = {}", double(21));
    io::println("add(10, 32) = {}", add(10, 32));
    io::println("square(9) = {}", square(9));
}
