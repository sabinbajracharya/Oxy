// arithmetic.ox — Basic arithmetic operations
fn main() {
    val a = 10;
    val b = 3;

    io::println("{} + {} = {}", a, b, a + b);
    io::println("{} - {} = {}", a, b, a - b);
    io::println("{} * {} = {}", a, b, a * b);
    io::println("{} / {} = {}", a, b, a / b);
    io::println("{} % {} = {}", a, b, a % b);

    val x = 2.5;
    val y = 1.5;
    io::println("{} + {} = {}", x, y, x + y);
}
