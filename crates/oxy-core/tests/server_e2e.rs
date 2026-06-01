//! End-to-end test for the `std::server` module.
//!
//! Spins up a server in a worker thread, makes real TCP requests against
//! `127.0.0.1`, and asserts on the wire-level HTTP responses. Because the
//! `listen` call blocks indefinitely, the worker thread is leaked — the
//! test runner exits when the test function returns, which tears it down.

#![cfg(feature = "server")]

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use oxy_core::vm::run_compiled_capturing;

/// Find a TCP port the OS reports as currently free. There's an inherent
/// race between this and the server binding it, but in practice it's
/// reliable enough for a single test.
fn pick_port() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let p = l.local_addr().unwrap().port();
    drop(l);
    p
}

/// Wait until the server is accepting connections (or time out).
fn wait_ready(port: u16) {
    for _ in 0..50 {
        if TcpStream::connect_timeout(
            &format!("127.0.0.1:{port}").parse().unwrap(),
            Duration::from_millis(50),
        )
        .is_ok()
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("server did not become ready on port {port}");
}

/// Send a raw HTTP request, return the full response text.
fn http_request(port: u16, request: &str) -> String {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}

#[test]
fn server_serves_text_and_json_routes() {
    let port = pick_port();
    let src = format!(
        r#"
fn main() {{
    val app = std::server::new();
    std::server::get(app, "/hello", |req| std::server::text("hello world"));
    std::server::get(app, "/users/:id", |req| {{
        val id = req.params.get("id").unwrap();
        std::server::json(string::format("{{{{\"id\":\"{{}}\"}}}}", id))
    }});
    std::server::post(app, "/echo", |req| std::server::text(req.body));
    std::server::listen(app, {port});
}}
"#,
    );

    // The VM's listen() blocks; run it in a thread that we deliberately leak.
    std::thread::spawn(move || {
        let _ = run_compiled_capturing(&src);
    });
    wait_ready(port);

    // GET /hello — basic text route.
    let resp = http_request(port, "GET /hello HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(resp.contains("200 OK"), "got: {resp}");
    assert!(resp.contains("hello world"), "got: {resp}");
    assert!(
        resp.to_lowercase().contains("content-type: text/plain"),
        "got: {resp}"
    );

    // GET /users/42 — path params.
    let resp = http_request(port, "GET /users/42 HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(resp.contains("200 OK"), "got: {resp}");
    assert!(resp.contains(r#"{"id":"42"}"#), "got: {resp}");
    assert!(
        resp.to_lowercase().contains("application/json"),
        "got: {resp}"
    );

    // POST /echo — body.
    let body = "ping";
    let req = format!(
        "POST /echo HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let resp = http_request(port, &req);
    assert!(resp.contains("200 OK"), "got: {resp}");
    assert!(
        resp.ends_with(body),
        "expected body to be {body:?}; got: {resp}"
    );

    // GET /missing — 404.
    let resp = http_request(port, "GET /missing HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(resp.contains("404 Not Found"), "got: {resp}");

    // POST /hello — method mismatch should also 404 (no matching route).
    let resp = http_request(
        port,
        "POST /hello HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\n\r\n",
    );
    assert!(resp.contains("404 Not Found"), "got: {resp}");
}

#[test]
fn server_status_helper_sets_arbitrary_code() {
    let port = pick_port();
    let src = format!(
        r#"
fn main() {{
    val app = std::server::new();
    std::server::get(app, "/teapot", |req| std::server::status(418, "i'm a teapot"));
    std::server::listen(app, {port});
}}
"#,
    );
    std::thread::spawn(move || {
        let _ = run_compiled_capturing(&src);
    });
    wait_ready(port);

    let resp = http_request(port, "GET /teapot HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(resp.contains("418"), "got: {resp}");
    assert!(resp.contains("i'm a teapot"), "got: {resp}");
}
