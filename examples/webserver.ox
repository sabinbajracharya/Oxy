// Example: Simple web server in Oxide
//
// Run with: oxide run examples/webserver.ox
// Then visit http://127.0.0.1:8080 in your browser

fn main() {
    let app = Server::new();

    // Home page
    app.get("/", |req| {
        Response::html("<h1>Welcome to Oxide!</h1><p>A web server written in Oxide.</p>")
    });

    // Plain text endpoint
    app.get("/hello", |req| {
        Response::text("Hello, World!")
    });

    // Path parameters
    app.get("/users/:id", |req| {
        let id = req.params.get("id").unwrap_or("unknown");
        Response::json(format!("{{\"id\": \"{}\"}}", id))
    });

    // POST endpoint — echo the request body
    app.post("/echo", |req| {
        Response::text(req.body)
    });

    // Custom status code
    app.get("/teapot", |req| {
        Response::status(418, "I'm a teapot")
    });

    // Serve static files from ./public directory
    // app.static_files("./public");

    // Start the server
    println!("Starting server...");
    app.listen("127.0.0.1:8080");
}
