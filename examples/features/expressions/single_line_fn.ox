// Single-line function syntax — `fn name(params) [-> Type] = expr`
// Desugars to `fn name(params) [-> Type] { expr }` at parse time.
// Verified via Rust-side type checker tests.

fn double(x: int) -> int = x * 2
fn add(a: int, b: int) -> int = a + b
fn multiply(a: int, b: int) -> int = a * b
fn square(x: int) -> int = x * x

// No return type annotation — inferred from body
fn greet(name: String) = "Hello, " + name

fn main() {
    println!("double(21) = {}", double(21));
    println!("add(10, 32) = {}", add(10, 32));
    println!("square(9) = {}", square(9));
}
