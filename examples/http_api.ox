// Example: HTTP API calls in Oxy
fn main() {
    // Simple GET request
    val resp = http::get("https://jsonplaceholder.typicode.com/todos/1");
    match resp {
        Ok(response) => {
            io::println("Status: {}", response.status);
            if response.status_ok() {
                val data = response.json().unwrap();
                io::println("Title: {}", data.get("title").unwrap());
            }
        }
        Err(e) => io::println("Error: {}", e),
    }

    // POST with JSON
    var user = Map::new();
    user.insert("name", "Alice");
    user.insert("email", "alice@example.com");

    val resp = http::post_json("https://jsonplaceholder.typicode.com/posts", user);
    match resp {
        Ok(response) => io::println("Created with status: {}", response.status),
        Err(e) => io::println("Error: {}", e),
    }

    // Request builder
    val resp = http::request("GET", "https://jsonplaceholder.typicode.com/posts/1")
        .header("Accept", "application/json")
        .send();
    match resp {
        Ok(response) => io::println("Builder status: {}", response.status),
        Err(e) => io::println("Error: {}", e),
    }
}
