// === Feature: async HTTP client (http::fetch, http::fetch_post) ===
// Tests that async HTTP calls return Future<HttpResponse> and .await unwraps them.

fn main() {}

fn take_int(x: Int) -> Int { x }
fn take_string(s: String) -> String { s }

struct HttpResponse {
    status: Int,
    body: String,
    headers: Map<String, String>,
}

fn expect_response(r: HttpResponse) -> HttpResponse { r }

// --- type-checker: http::fetch returns Future<HttpResponse> ---

#[test]
fn test_fetch_type_flows_through_await() {
    val f = http::fetch("https://example.com".to_string());   // Future<HttpResponse>
    val r = f.await;                                           // HttpResponse
    val _ = expect_response(r);                                // OK: HttpResponse → HttpResponse
}

#[test]
fn test_fetch_post_type_flows_through_await() {
    val f = http::fetch_post("https://example.com".to_string(), "body".to_string());
    val r = f.await;
    val _ = expect_response(r);
}

// --- compile_error: type mismatch across http::fetch .await ---

#[compile_error]
fn fetch_wrong_type_through_await() {
    val f = http::fetch("https://example.com".to_string());   // Future<HttpResponse>
    val r = f.await;                                           // HttpResponse
    val _ = take_int(r);                                       // ERROR: HttpResponse ≠ Int
}

#[compile_error]
fn fetch_post_wrong_type_through_await() {
    val f = http::fetch_post("https://example.com".to_string(), "data".to_string());
    val r = f.await;
    val _ = take_int(r);                                       // ERROR: HttpResponse ≠ Int
}

