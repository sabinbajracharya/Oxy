// Oxide Standard Library Examples
// Demonstrates: std::fs, std::env, std::process, std::regex

fn main() {
    // === File System ===
    println!("=== File System ===");
    
    // Write and read a file
    let result = std::fs::write("test_stdlib.txt", "Hello from Oxide!");
    match result {
        Ok(_) => println!("File written successfully"),
        Err(e) => println!("Write error: {}", e),
    }
    
    let content = std::fs::read_to_string("test_stdlib.txt");
    match content {
        Ok(text) => println!("Read: {}", text),
        Err(e) => println!("Read error: {}", e),
    }
    
    // Check file properties
    println!("Exists: {}", std::fs::exists("test_stdlib.txt"));
    println!("Is file: {}", std::fs::is_file("test_stdlib.txt"));
    println!("Is dir: {}", std::fs::is_dir("test_stdlib.txt"));
    
    // Get metadata
    let meta = std::fs::metadata("test_stdlib.txt");
    match meta {
        Ok(m) => println!("Size: {} bytes", m.size),
        Err(e) => println!("Metadata error: {}", e),
    }
    
    // Clean up
    let _ = std::fs::remove_file("test_stdlib.txt");
    
    // Directory operations
    let _ = std::fs::create_dir_all("test_dir/nested");
    let _ = std::fs::write("test_dir/file1.txt", "one");
    let _ = std::fs::write("test_dir/file2.txt", "two");
    
    let entries = std::fs::read_dir("test_dir");
    match entries {
        Ok(list) => println!("Directory contents: {:?}", list),
        Err(e) => println!("Read dir error: {}", e),
    }
    
    // Clean up
    let _ = std::fs::remove_file("test_dir/file1.txt");
    let _ = std::fs::remove_file("test_dir/file2.txt");
    let _ = std::fs::remove_dir("test_dir/nested");
    let _ = std::fs::remove_dir("test_dir");
    
    // === Environment ===
    println!("\n=== Environment ===");
    
    let path = std::env::var("PATH");
    match path {
        Some(p) => println!("PATH starts with: {}...", p),
        None => println!("PATH not set"),
    }
    
    let missing = std::env::var("THIS_DOES_NOT_EXIST_12345");
    println!("Missing var: {:?}", missing);
    
    let cwd = std::env::current_dir();
    match cwd {
        Ok(dir) => println!("Current dir: {}", dir),
        Err(e) => println!("Error: {}", e),
    }
    
    // === Process ===
    println!("\n=== Process ===");
    
    let result = std::process::command_with_args("echo", vec!["Hello", "World"]);
    match result {
        Ok(output) => {
            println!("stdout: {}", output.stdout);
            println!("status: {}", output.status);
            println!("success: {}", output.success);
        }
        Err(e) => println!("Command error: {}", e),
    }
    
    // === Regex ===
    println!("\n=== Regex ===");
    
    let text = "The price is $42.50 and $18.99";
    
    // Check if pattern matches
    let has_price = std::regex::is_match(r"\$\d+\.\d+", text);
    println!("Has price: {}", has_price);
    
    // Find first match
    let first = std::regex::find(r"\$(\d+\.\d+)", text);
    match first {
        Some(m) => println!("First price: {} at position {}", m.text, m.start),
        None => println!("No price found"),
    }
    
    // Find all matches
    let all = std::regex::find_all(r"\$\d+\.\d+", text);
    println!("All prices: {:?}", all);
    
    // Named captures
    let caps = std::regex::captures(r"(?P<currency>\$)(?P<amount>\d+\.\d+)", text);
    match caps {
        Some(c) => println!("Currency: {}, Amount: {}", c["currency"], c["amount"]),
        None => println!("No captures"),
    }
    
    // Replace
    let censored = std::regex::replace_all(r"\$\d+\.\d+", text, "[REDACTED]");
    println!("Censored: {}", censored);
    
    // Split
    let parts = std::regex::split(r"\s+", "hello   world   foo");
    println!("Split: {:?}", parts);
}
