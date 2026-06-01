// Collections & Strings example for Oxy

fn main() {
    // === Vectors ===
    var numbers = [1, 2, 3, 4, 5];
    io::println("Numbers: {:?}", numbers);
    io::println("Length: {}", numbers.len());

    numbers.push(6);
    io::println("After push: {:?}", numbers);

    val popped = numbers.pop();
    io::println("Popped: {}", popped);

    // Index access and assignment
    numbers[0] = 10;
    io::println("After numbers[0] = 10: {:?}", numbers);
    io::println("First: {}, Last: {}", numbers.first(), numbers.last());

    // Iteration
    var sum = 0;
    for n in numbers {
        sum += n;
    }
    io::println("Sum: {}", sum);

    // Array literal
    val arr = [100, 200, 300];
    io::println("Array: {:?}", arr);
    io::println("arr[1] = {}", arr[1]);

    // Nested vectors
    val matrix = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];
    io::println("Matrix[1][2] = {}", matrix[1][2]);

    // === Tuples ===
    val point = (3, 4);
    io::println("Point: {:?}", point);
    io::println("x = {}, y = {}", point.0, point.1);

    val mixed = (42, "hello", true, 3.14);
    io::println("Mixed tuple: {:?}", mixed);

    // === Strings ===
    val greeting = "Hello, Oxy!";
    io::println("Greeting: {}", greeting);
    io::println("Length: {}", greeting.len());
    io::println("Uppercase: {}", greeting.to_uppercase());
    io::println("Contains 'Oxy': {}", greeting.contains("Oxy"));
    io::println("Starts with 'Hello': {}", greeting.starts_with("Hello"));

    val csv = "apple,banana,cherry";
    val fruits = csv.split(",");
    io::println("Fruits: {:?}", fruits);
    io::println("Joined: {}", fruits.join(" | "));

    val padded = "  trim me  ";
    io::println("Trimmed: '{}'", padded.trim());

    val repeated = "ha".repeat(3);
    io::println("Repeated: {}", repeated);

    val replaced = "hello world".replace("world", "oxide");
    io::println("Replaced: {}", replaced);

    // String iteration
    io::print("Chars: ");
    for c in "Oxy" {
        io::print("{} ", c);
    }
    io::println("");
}
