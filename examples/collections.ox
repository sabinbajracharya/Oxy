// Collections & Strings example for Oxy

fn main() {
    // === Vectors ===
    var numbers = [1, 2, 3, 4, 5];
    println("Numbers: {:?}", numbers);
    println("Length: {}", numbers.len());

    numbers.push(6);
    println("After push: {:?}", numbers);

    val popped = numbers.pop();
    println("Popped: {}", popped);

    // Index access and assignment
    numbers[0] = 10;
    println("After numbers[0] = 10: {:?}", numbers);
    println("First: {}, Last: {}", numbers.first(), numbers.last());

    // Iteration
    var sum = 0;
    for n in numbers {
        sum += n;
    }
    println("Sum: {}", sum);

    // Array literal
    val arr = [100, 200, 300];
    println("Array: {:?}", arr);
    println("arr[1] = {}", arr[1]);

    // Nested vectors
    val matrix = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];
    println("Matrix[1][2] = {}", matrix[1][2]);

    // === Tuples ===
    val point = (3, 4);
    println("Point: {:?}", point);
    println("x = {}, y = {}", point.0, point.1);

    val mixed = (42, "hello", true, 3.14);
    println("Mixed tuple: {:?}", mixed);

    // === Strings ===
    val greeting = "Hello, Oxy!";
    println("Greeting: {}", greeting);
    println("Length: {}", greeting.len());
    println("Uppercase: {}", greeting.to_uppercase());
    println("Contains 'Oxy': {}", greeting.contains("Oxy"));
    println("Starts with 'Hello': {}", greeting.starts_with("Hello"));

    val csv = "apple,banana,cherry";
    val fruits = csv.split(",");
    println("Fruits: {:?}", fruits);
    println("Joined: {}", fruits.join(" | "));

    val padded = "  trim me  ";
    println("Trimmed: '{}'", padded.trim());

    val repeated = "ha".repeat(3);
    println("Repeated: {}", repeated);

    val replaced = "hello world".replace("world", "oxide");
    println("Replaced: {}", replaced);

    // String iteration
    print("Chars: ");
    for c in "Oxy" {
        print("{} ", c);
    }
    println("");
}
