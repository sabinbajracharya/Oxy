// Oxide Error Handling Example
// Demonstrates Option, Result, ? operator, if let, while let, and panic!

fn divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        Err(String::from("division by zero"))
    } else {
        Ok(a / b)
    }
}

fn find_item(items: Vec<String>, target: String) -> Option<i64> {
    let mut i = 0;
    for item in items {
        if item == target {
            return Some(i);
        }
        i = i + 1;
    }
    None
}

fn safe_divide(a: f64, b: f64) -> Result<f64, String> {
    let result = divide(a, b)?;
    Ok(result * 2.0)
}

fn main() {
    // Result usage
    println!("=== Result ===");
    match divide(10.0, 3.0) {
        Ok(result) => println!("10 / 3 = {}", result),
        Err(e) => println!("Error: {}", e),
    }

    match divide(10.0, 0.0) {
        Ok(result) => println!("10 / 0 = {}", result),
        Err(e) => println!("Error: {}", e),
    }

    // Result methods
    let ok_val: Result<i64, String> = Ok(42);
    println!("is_ok: {}", ok_val.is_ok());
    println!("unwrap: {}", ok_val.unwrap());

    // Option usage
    println!("\n=== Option ===");
    let items = vec!["apple", "banana", "cherry"];
    let found = find_item(items, "banana");

    if let Some(idx) = found {
        println!("Found banana at index {}", idx);
    } else {
        println!("Not found");
    }

    // Option methods
    let some_val = Some(10);
    let none_val: Option<i64> = None;
    println!("some is_some: {}", some_val.is_some());
    println!("none is_none: {}", none_val.is_none());
    println!("unwrap_or: {}", none_val.unwrap_or(99));

    // while let with Vec::pop()
    println!("\n=== while let ===");
    let mut stack = vec![1, 2, 3];
    while let Some(top) = stack.pop() {
        println!("popped: {}", top);
    }

    // ? operator
    println!("\n=== ? operator ===");
    match safe_divide(10.0, 2.0) {
        Ok(result) => println!("safe_divide(10, 2) = {}", result),
        Err(e) => println!("Error: {}", e),
    }

    match safe_divide(10.0, 0.0) {
        Ok(result) => println!("safe_divide(10, 0) = {}", result),
        Err(e) => println!("Error: {}", e),
    }

    // dbg! macro
    println!("\n=== dbg! ===");
    let x = 42;
    dbg!(x);

    println!("\nDone!");
}
