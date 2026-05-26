// === Feature: async fn, .await, spawn, and sleep ===
// Tests that async functions return Futures, .await resolves them,
// spawn runs closures eagerly returning JoinHandle, and sleep pauses.

fn main() {}

// --- async fn definitions (must be at top level) ---

async fn answer() -> int { 42 }

async fn add(a: int, b: int) -> int { a + b }

async fn double(x: int) -> int { x * 2 }

async fn greet(name: String) -> String { "Hello, ".to_string() + name }

async fn inner(x: int) -> int { x + 1 }

async fn outer(x: int) -> int {
    let f = inner(x);
    f.await * 2
}

async fn step1(x: int) -> int { x + 1 }

async fn step2(x: int) -> int { step1(x).await + 1 }

async fn step3(x: int) -> int { step2(x).await + 1 }

fn sync_double(x: int) -> int { x * 2 }

// --- async fn basics ---

#[test]
fn test_async_fn_returns_future() {
    let f = answer();
    assert_eq!(f.await, 42);
}

#[test]
fn test_async_fn_with_params() {
    let f = add(3, 4);
    assert_eq!(f.await, 7);
}

#[test]
fn test_async_fn_multiple_calls() {
    let a = double(5);
    let b = double(10);
    assert_eq!(a.await, 10);
    assert_eq!(b.await, 20);
}

#[test]
fn test_async_fn_string_return() {
    let f = greet("World".to_string());
    assert_eq!(f.await, "Hello, World");
}

// --- await passthrough ---

#[test]
fn test_await_on_plain_value_passes_through() {
    let x = 42;
    assert_eq!(x.await, 42);
}

#[test]
fn test_await_on_string_passes_through() {
    let s = "hello".to_string();
    assert_eq!(s.await, "hello");
}

// --- spawn ---

#[test]
fn test_spawn_returns_join_handle() {
    let h = spawn(|| 42);
    assert_eq!(h.await, 42);
}

#[test]
fn test_spawn_with_capture() {
    let x = 10;
    let h = spawn(|| x * 2);
    assert_eq!(h.await, 20);
}

#[test]
fn test_spawn_multiple() {
    let a = spawn(|| 100);
    let b = spawn(|| 200);
    assert_eq!(a.await, 100);
    assert_eq!(b.await, 200);
}

// --- sleep ---

#[test]
fn test_sleep_runs_without_error() {
    let _ = sleep(10);
}

// --- nested async ---

#[test]
fn test_nested_async_calls() {
    let f = outer(5);
    assert_eq!(f.await, 12);
}

#[test]
fn test_async_fn_chain() {
    assert_eq!(step3(1).await, 4);
}

// --- await on non-Future from fn call ---

#[test]
fn test_await_on_sync_fn_result() {
    let v = sync_double(21);
    assert_eq!(v.await, 42);
}
