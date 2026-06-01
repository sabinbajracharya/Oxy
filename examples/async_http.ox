// Example: Async/Await in Oxy
async fn fetch_data(url: String) -> Result<String, String> {
    val resp = http::get(url)?;
    Ok(resp.body)
}

fn main() {
    // Async function call returns a future
    val future = fetch_data("https://jsonplaceholder.typicode.com/todos/1".to_string());

    // .await resolves the future
    val result = future.await;
    match result {
        Ok(body) => io::println("Got: {}", body),
        Err(e) => io::println("Error: {}", e),
    }

    // spawn runs a task eagerly
    val handle = spawn(|| {
        var sum = 0;
        for i in 0..100 {
            sum += i;
        }
        sum
    });
    io::println("Sum: {}", handle.await);

    // sleep pauses execution
    sleep(10);
    io::println("Done!");
}
