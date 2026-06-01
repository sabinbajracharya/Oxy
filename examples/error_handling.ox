// Oxy Error Handling Example
// Demonstrates Option, Result, ? operator, if let, while let, and panic

fn divide(a: Float, b: Float) -> Result<Float, String> {
    if b == 0.0 {
        Err(String::from("division by zero"))
    } else {
        Ok(a / b)
    }
}

fn find_item(items: List<String>, target: String) -> Option<Int> {
    var i = 0;
    for item in items {
        if item == target {
            return Some(i);
        }
        i = i + 1;
    }
    None
}

fn safe_divide(a: Float, b: Float) -> Result<Float, String> {
    val result = divide(a, b)?;
    Ok(result * 2.0)
}

fn main() {
    // Result usage
    io::println("=== Result ===");
    match divide(10.0, 3.0) {
        Ok(result) => io::println("10 / 3 = {}", result),
        Err(e) => io::println("Error: {}", e),
    }

    match divide(10.0, 0.0) {
        Ok(result) => io::println("10 / 0 = {}", result),
        Err(e) => io::println("Error: {}", e),
    }

    // Result methods
    val ok_val: Result<Int, String> = Ok(42);
    io::println("is_ok: {}", ok_val.is_ok());
    io::println("unwrap: {}", ok_val.unwrap());

    // Option usage
    io::println("\n=== Option ===");
    val items = ["apple", "banana", "cherry"];
    val found = find_item(items, "banana");

    if val Some(idx) = found {
        io::println("Found banana at index {}", idx);
    } else {
        io::println("Not found");
    }

    // Option methods
    val some_val = Some(10);
    val none_val: Option<Int> = None;
    io::println("some is_some: {}", some_val.is_some());
    io::println("none is_none: {}", none_val.is_none());
    io::println("unwrap_or: {}", none_val.unwrap_or(99));

    // while val with List::pop()
    io::println("\n=== while val ===");
    var stack = [1, 2, 3];
    while val Some(top) = stack.pop() {
        io::println("popped: {}", top);
    }

    // ? operator
    io::println("\n=== ? operator ===");
    match safe_divide(10.0, 2.0) {
        Ok(result) => io::println("safe_divide(10, 2) = {}", result),
        Err(e) => io::println("Error: {}", e),
    }

    match safe_divide(10.0, 0.0) {
        Ok(result) => io::println("safe_divide(10, 0) = {}", result),
        Err(e) => io::println("Error: {}", e),
    }

    // dbg function
    io::println("\n=== dbg ===");
    val x = 42;
    sys::dbg(x);

    io::println("\nDone!");
}
