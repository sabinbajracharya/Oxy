// Pipeline operator `|>` examples — desugars `x |> f(args)` to `f(x, args)`.
// Verified via Rust-side type checker tests and manual CLI runs.

fn double(x: Int) -> Int { x * 2 }
fn add(a: Int, b: Int) -> Int { a + b }
fn multiply(a: Int, b: Int) -> Int { a * b }

fn main() {
    val r1 = 5 |> double();
    println("5 |> double() = {}", r1);

    val r2 = 5 |> add(3);
    println("5 |> add(3) = {}", r2);

    val r3 = 5 |> double() |> add(3);
    println("5 |> double() |> add(3) = {}", r3);

    val r4 = 21 |> double;
    println("21 |> double = {}", r4);

    val r5 = 1 |> add(2) |> multiply(3) |> add(4);
    println("1 |> add(2) |> multiply(3) |> add(4) = {}", r5);
}
