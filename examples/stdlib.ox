// Oxy Standard Library Examples
// Demonstrates: std::fs, std::env, std::process, std::regex

fn main() {
    // === File System ===
    io::println("=== File System ===");
    
    // Write and read a file
    val result = std::fs::write("test_stdlib.txt", "Hello from Oxy!");
    match result {
        Ok(_) => io::println("File written successfully"),
        Err(e) => io::println("Write error: {}", e),
    }
    
    val content = std::fs::read_to_string("test_stdlib.txt");
    match content {
        Ok(text) => io::println("Read: {}", text),
        Err(e) => io::println("Read error: {}", e),
    }
    
    // Check file properties
    io::println("Exists: {}", std::fs::exists("test_stdlib.txt"));
    io::println("Is file: {}", std::fs::is_file("test_stdlib.txt"));
    io::println("Is dir: {}", std::fs::is_dir("test_stdlib.txt"));
    
    // Get metadata
    val meta = std::fs::metadata("test_stdlib.txt");
    match meta {
        Ok(m) => io::println("Size: {} bytes", m.size),
        Err(e) => io::println("Metadata error: {}", e),
    }
    
    // Clean up
    val _ = std::fs::remove_file("test_stdlib.txt");
    
    // Directory operations
    val _ = std::fs::create_dir_all("test_dir/nested");
    val _ = std::fs::write("test_dir/file1.txt", "one");
    val _ = std::fs::write("test_dir/file2.txt", "two");
    
    val entries = std::fs::read_dir("test_dir");
    match entries {
        Ok(list) => io::println("Directory contents: {:?}", list),
        Err(e) => io::println("Read dir error: {}", e),
    }
    
    // Clean up
    val _ = std::fs::remove_file("test_dir/file1.txt");
    val _ = std::fs::remove_file("test_dir/file2.txt");
    val _ = std::fs::remove_dir("test_dir/nested");
    val _ = std::fs::remove_dir("test_dir");
    
    // === Environment ===
    io::println("\n=== Environment ===");
    
    val path = std::env::get("PATH");
    match path {
        Some(p) => io::println("PATH starts with: {}...", p),
        None => io::println("PATH not set"),
    }
    
    val missing = std::env::get("THIS_DOES_NOT_EXIST_12345");
    io::println("Missing var: {:?}", missing);
    
    val cwd = std::env::current_dir();
    match cwd {
        Ok(dir) => io::println("Current dir: {}", dir),
        Err(e) => io::println("Error: {}", e),
    }
    
    // === Process ===
    io::println("\n=== Process ===");
    
    val result = std::process::command_with_args("echo", ["Hello", "World"]);
    match result {
        Ok(output) => {
            io::println("stdout: {}", output.stdout);
            io::println("status: {}", output.status);
            io::println("success: {}", output.success);
        }
        Err(e) => io::println("Command error: {}", e),
    }
    
    // === Regex ===
    io::println("\n=== Regex ===");
    
    val text = "The price is $42.50 and $18.99";
    
    // Check if pattern matches
    val has_price = std::regex::is_match(r"\$\d+\.\d+", text);
    io::println("Has price: {}", has_price);
    
    // Find first match
    val first = std::regex::find(r"\$(\d+\.\d+)", text);
    match first {
        Some(m) => io::println("First price: {} at position {}", m.text, m.start),
        None => io::println("No price found"),
    }
    
    // Find all matches
    val all = std::regex::find_all(r"\$\d+\.\d+", text);
    io::println("All prices: {:?}", all);
    
    // Named captures
    val caps = std::regex::captures(r"(?P<currency>\$)(?P<amount>\d+\.\d+)", text);
    match caps {
        Some(c) => io::println("Currency: {}, Amount: {}", c["currency"], c["amount"]),
        None => io::println("No captures"),
    }
    
    // Replace
    val censored = std::regex::replace_all(r"\$\d+\.\d+", text, "[REDACTED]");
    io::println("Censored: {}", censored);
    
    // Split
    val parts = std::regex::split(r"\s+", "hello   world   foo");
    io::println("Split: {:?}", parts);
}
