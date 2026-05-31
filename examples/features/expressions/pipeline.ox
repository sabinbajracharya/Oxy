// Pipeline operator `|>` examples — desugars `x |> f(args)` to `f(x, args)`.
// Verified via Rust-side type checker tests and manual CLI runs.

fn double(x: int) -> int { x * 2 }
fn add(a: int, b: int) -> int { a + b }
fn multiply(a: int, b: int) -> int { a * b }

fn main() {
    let r1 = 5 |> double();
    println("5 |> double() = {}", r1);

    let r2 = 5 |> add(3);
    println("5 |> add(3) = {}", r2);

    let r3 = 5 |> double() |> add(3);
    println("5 |> double() |> add(3) = {}", r3);

    let r4 = 21 |> double;
    println("21 |> double = {}", r4);

    let r5 = 1 |> add(2) |> multiply(3) |> add(4);
    println("1 |> add(2) |> multiply(3) |> add(4) = {}", r5);
}
