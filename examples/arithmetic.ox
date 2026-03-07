// arithmetic.ox — Basic arithmetic operations
fn main() {
    let a = 10;
    let b = 3;

    println!("{} + {} = {}", a, b, a + b);
    println!("{} - {} = {}", a, b, a - b);
    println!("{} * {} = {}", a, b, a * b);
    println!("{} / {} = {}", a, b, a / b);
    println!("{} % {} = {}", a, b, a % b);

    let x = 2.5;
    let y = 1.5;
    println!("{} + {} = {}", x, y, x + y);
}
