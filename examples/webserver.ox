// Example: Simple web server in Oxy
//
// Run with: oxy run examples/webserver.ox
// Then visit http://127.0.0.1:8080 in your browser

fn main() {
    val app = std::server::new();

    // Home page
    std::server::get(app, "/", |req| {
        std::server::html("<h1>Welcome to Oxy!</h1><p>A web server written in Oxy.</p>")
    });

    // Plain text endpoint
    std::server::get(app, "/hello", |req| {
        std::server::text("Hello, World!")
    });

    // Path parameters
    std::server::get(app, "/users/:id", |req| {
        val id = req.params.get("id").unwrap_or("unknown");
        std::server::json(string::format("{{\"id\": \"{}\"}}", id))
    });

    // POST endpoint — echo the request body
    std::server::post(app, "/echo", |req| {
        std::server::text(req.body)
    });

    // Custom status code
    std::server::get(app, "/teapot", |req| {
        std::server::status(418, "I'm a teapot")
    });

    // Start the server
    io::println("Starting server on http://127.0.0.1:8080 ...");
    std::server::listen(app, 8080);
}
