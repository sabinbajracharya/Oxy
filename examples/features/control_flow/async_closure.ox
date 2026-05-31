// === Feature: async closures (`async ||`, `async |p|`) and async blocks (`async {}`) ===

fn main() {}

fn take_int(x: int) -> int { x }

// --- async closure: zero params ---

#[test]
fn test_async_closure_basic() {
    let f = async || { 42 };
    let fut = f();
    assert_eq(fut.await, 42);
}

#[test]
fn test_async_closure_single_expr_body() {
    let f = async || 99;
    assert_eq(f().await, 99);
}

// --- async closure: with params ---

#[test]
fn test_async_closure_with_params() {
    let f = async |x: int, y: int| { x + y };
    assert_eq(f(10, 20).await, 30);
}

#[test]
fn test_async_closure_single_param() {
    let f = async |x: int| { x * 3 };
    assert_eq(f(7).await, 21);
}

// --- async closure: captures ---

#[test]
fn test_async_closure_capture() {
    let x = 21;
    let f = async || { x * 2 };
    assert_eq(f().await, 42);
}

#[test]
fn test_async_closure_capture_with_param() {
    let base = 10;
    let f = async |x: int| { x + base };
    assert_eq(f(32).await, 42);
}

// --- async closure: string return ---

#[test]
fn test_async_closure_string() {
    let f = async || { "hello".to_string() };
    assert_eq(f().await, "hello");
}

// --- async block ---

#[test]
fn test_async_block_basic() {
    let fut = async { 42 };
    assert_eq(fut.await, 42);
}

#[test]
fn test_async_block_multiple_stmts() {
    let fut = async {
        let x = 10;
        let y = 32;
        x + y
    };
    assert_eq(fut.await, 42);
}

// --- async block: captures ---

#[test]
fn test_async_block_capture() {
    let x = 21;
    let fut = async { x * 2 };
    assert_eq(fut.await, 42);
}

// --- type-checker: async closure return type is Future<T> ---

#[test]
fn test_async_closure_type_flows() {
    let f = async || { 42 };
    let fut = f();        // Future<int>
    let v = fut.await;    // int
    let _ = take_int(v);  // OK: int -> int
}

#[test]
fn test_async_block_type_flows() {
    let fut = async { 42 };  // Future<int>
    let v = fut.await;       // int
    let _ = take_int(v);     // OK
}

// --- compile_error: type mismatch across async closure .await ---

#[compile_error]
fn async_closure_wrong_type() {
    let f = async || { "hi".to_string() };
    let fut = f();           // Future<String>
    let v = fut.await;       // String
    let _ = take_int(v);     // ERROR: String != int
}

#[compile_error]
fn async_block_wrong_type() {
    let fut = async { "hi".to_string() };  // Future<String>
    let v = fut.await;                     // String
    let _ = take_int(v);                   // ERROR
}

