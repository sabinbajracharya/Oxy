// === Feature: async closures (`async ||`, `async |p|`) and async blocks (`async {}`) ===

fn main() {}

fn take_int(x: Int) -> Int { x }

// --- async closure: zero params ---

#[test]
fn test_async_closure_basic() {
    val f = async || { 42 };
    val fut = f();
    assert::eq(fut.await, 42);
}

#[test]
fn test_async_closure_single_expr_body() {
    val f = async || 99;
    assert::eq(f().await, 99);
}

// --- async closure: with params ---

#[test]
fn test_async_closure_with_params() {
    val f = async |x: Int, y: Int| { x + y };
    assert::eq(f(10, 20).await, 30);
}

#[test]
fn test_async_closure_single_param() {
    val f = async |x: Int| { x * 3 };
    assert::eq(f(7).await, 21);
}

// --- async closure: captures ---

#[test]
fn test_async_closure_capture() {
    val x = 21;
    val f = async || { x * 2 };
    assert::eq(f().await, 42);
}

#[test]
fn test_async_closure_capture_with_param() {
    val base = 10;
    val f = async |x: Int| { x + base };
    assert::eq(f(32).await, 42);
}

// --- async closure: string return ---

#[test]
fn test_async_closure_string() {
    val f = async || { "hello".to_string() };
    assert::eq(f().await, "hello");
}

// --- async block ---

#[test]
fn test_async_block_basic() {
    val fut = async { 42 };
    assert::eq(fut.await, 42);
}

#[test]
fn test_async_block_multiple_stmts() {
    val fut = async {
        val x = 10;
        val y = 32;
        x + y
    };
    assert::eq(fut.await, 42);
}

// --- async block: captures ---

#[test]
fn test_async_block_capture() {
    val x = 21;
    val fut = async { x * 2 };
    assert::eq(fut.await, 42);
}

// --- type-checker: async closure return type is Future<T> ---

#[test]
fn test_async_closure_type_flows() {
    val f = async || { 42 };
    val fut = f();        // Future<Int>
    val v = fut.await;    // Int
    val _ = take_int(v);  // OK: Int -> Int
}

#[test]
fn test_async_block_type_flows() {
    val fut = async { 42 };  // Future<Int>
    val v = fut.await;       // Int
    val _ = take_int(v);     // OK
}

// --- compile_error: type mismatch across async closure .await ---

#[compile_error]
fn async_closure_wrong_type() {
    val f = async || { "hi".to_string() };
    val fut = f();           // Future<String>
    val v = fut.await;       // String
    val _ = take_int(v);     // ERROR: String != Int
}

#[compile_error]
fn async_block_wrong_type() {
    val fut = async { "hi".to_string() };  // Future<String>
    val v = fut.await;                     // String
    val _ = take_int(v);                   // ERROR
}

