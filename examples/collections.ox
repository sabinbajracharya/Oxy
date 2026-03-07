// Collections & Strings example for Oxide

fn main() {
    // === Vectors ===
    let mut numbers = vec![1, 2, 3, 4, 5];
    println!("Numbers: {:?}", numbers);
    println!("Length: {}", numbers.len());

    numbers.push(6);
    println!("After push: {:?}", numbers);

    let popped = numbers.pop();
    println!("Popped: {}", popped);

    // Index access and assignment
    numbers[0] = 10;
    println!("After numbers[0] = 10: {:?}", numbers);
    println!("First: {}, Last: {}", numbers.first(), numbers.last());

    // Iteration
    let mut sum = 0;
    for n in numbers {
        sum += n;
    }
    println!("Sum: {}", sum);

    // Array literal
    let arr = [100, 200, 300];
    println!("Array: {:?}", arr);
    println!("arr[1] = {}", arr[1]);

    // Nested vectors
    let matrix = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
    println!("Matrix[1][2] = {}", matrix[1][2]);

    // === Tuples ===
    let point = (3, 4);
    println!("Point: {:?}", point);
    println!("x = {}, y = {}", point.0, point.1);

    let mixed = (42, "hello", true, 3.14);
    println!("Mixed tuple: {:?}", mixed);

    // === Strings ===
    let greeting = "Hello, Oxide!";
    println!("Greeting: {}", greeting);
    println!("Length: {}", greeting.len());
    println!("Uppercase: {}", greeting.to_uppercase());
    println!("Contains 'Oxide': {}", greeting.contains("Oxide"));
    println!("Starts with 'Hello': {}", greeting.starts_with("Hello"));

    let csv = "apple,banana,cherry";
    let fruits = csv.split(",");
    println!("Fruits: {:?}", fruits);
    println!("Joined: {}", fruits.join(" | "));

    let padded = "  trim me  ";
    println!("Trimmed: '{}'", padded.trim());

    let repeated = "ha".repeat(3);
    println!("Repeated: {}", repeated);

    let replaced = "hello world".replace("world", "oxide");
    println!("Replaced: {}", replaced);

    // String iteration
    print!("Chars: ");
    for c in "Oxide" {
        print!("{} ", c);
    }
    println!("");
}
