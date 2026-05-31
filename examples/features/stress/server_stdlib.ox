// === STRESS: std::server route registration & response helpers ===
//
// Network behaviour (listen + actual HTTP) is covered by the Rust integration
// test crates/oxy-core/tests/server_e2e.rs. These tests cover the parts that
// don't block: app creation, route registration, and response-struct builders.

#[test]
fn test_new_returns_increasing_handle() {
    val a = std::server::new();
    val b = std::server::new();
    assert(a > 0);
    assert(b > a);
}

#[test]
fn test_register_routes_all_methods() {
    val app = std::server::new();
    // Each call should succeed without erroring; we don't assert on the
    // return value because Oxy's `()` literal isn't comparable via assert_eq.
    std::server::get(app, "/", |req| "ok");
    std::server::post(app, "/items", |req| "created");
    std::server::put(app, "/items/:id", |req| "updated");
    std::server::delete(app, "/items/:id", |req| "deleted");
    std::server::patch(app, "/items/:id", |req| "patched");
}

#[test]
fn test_text_response_shape() {
    val r = std::server::text("hello");
    assert_eq(r.status, 200);
    assert_eq(r.body, "hello");
    assert_eq(r.content_type, "text/plain; charset=utf-8");
}

#[test]
fn test_json_response_shape() {
    val r = std::server::json("{\"k\":1}");
    assert_eq(r.status, 200);
    assert_eq(r.body, "{\"k\":1}");
    assert_eq(r.content_type, "application/json; charset=utf-8");
}

#[test]
fn test_html_response_shape() {
    val r = std::server::html("<h1>hi</h1>");
    assert_eq(r.status, 200);
    assert_eq(r.content_type, "text/html; charset=utf-8");
}

#[test]
fn test_status_helper_arbitrary_code() {
    val r = std::server::status(404, "missing");
    assert_eq(r.status, 404);
    assert_eq(r.body, "missing");
}

#[test]
fn test_listen_invalid_port_returns_err() {
    val app = std::server::new();
    val r = std::server::listen(app, 99999);
    assert(r.is_err());
}
