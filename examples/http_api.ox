// Example: HTTP API calls in Oxide
fn main() {
    // Simple GET request
    let resp = http::get("https://jsonplaceholder.typicode.com/todos/1");
    match resp {
        Ok(response) => {
            println!("Status: {}", response.status);
            if response.status_ok() {
                let data = response.json().unwrap();
                println!("Title: {}", data.get("title").unwrap());
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    // POST with JSON
    let mut user = HashMap::new();
    user.insert("name", "Alice");
    user.insert("email", "alice@example.com");

    let resp = http::post_json("https://jsonplaceholder.typicode.com/posts", user);
    match resp {
        Ok(response) => println!("Created with status: {}", response.status),
        Err(e) => println!("Error: {}", e),
    }

    // Request builder
    let resp = http::request("GET", "https://jsonplaceholder.typicode.com/posts/1")
        .header("Accept", "application/json")
        .send();
    match resp {
        Ok(response) => println!("Builder status: {}", response.status),
        Err(e) => println!("Error: {}", e),
    }
}
