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

// --- compile_error: spawn with wrong arg count ---

#[compile_error]
fn spawn_zero_args() {
    spawn();
}

#[compile_error]
fn spawn_two_args() {
    spawn(|| 1, || 2);
}

// --- compile_error: sleep with wrong arg count ---

#[compile_error]
fn sleep_zero_args() {
    sleep();
}

#[compile_error]
fn sleep_two_args() {
    sleep(10, 20);
}

// --- compile_error: spawn with non-closure arg ---

#[compile_error]
fn spawn_non_closure() {
    spawn(42);
}

// --- type-checker: .await resolves to the correct type ---

fn take_int(x: int) -> int { x }

fn take_string(s: String) -> String { s }

#[test]
fn test_await_future_type_flows_to_callee() {
    let f = answer();          // Future<int>
    let v = f.await;           // int (unwrapped by type checker)
    let _ = take_int(v);       // OK: int → int
}

#[test]
fn test_await_spawn_type_flows_to_callee() {
    let h = spawn(|| 42);      // JoinHandle<int>
    let v = h.await;           // int
    let _ = take_int(v);       // OK
}

#[test]
fn test_await_plain_value_passthrough() {
    let x = 42;
    let v = x.await;           // passthrough: int
    let _ = take_int(v);       // OK
}

// --- compile_error: type mismatch across .await ---

#[compile_error]
fn await_future_wrong_type() {
    let f = answer();          // Future<int>
    let v = f.await;           // int
    let _ = take_string(v);    // ERROR: int does not match String
}

#[compile_error]
fn await_spawn_wrong_type() {
    let h = spawn(|| 42);      // JoinHandle<int>
    let v = h.await;           // int
    let _ = take_string(v);    // ERROR: int does not match String
}

// --- event-loop spawn: correctness ---

#[test]
fn test_spawn_basic() {
    let h = spawn(|| 42);
    assert_eq!(h.await, 42);
}

#[test]
fn test_spawn_with_captured_var() {
    let x = 10;
    let h = spawn(|| x * 3);
    assert_eq!(h.await, 30);
}

#[test]
fn test_spawn_multiple_independent() {
    let a = spawn(|| 100);
    let b = spawn(|| 200);
    let c = spawn(|| 300);
    // Results are collected in any order
    assert_eq!(a.await, 100);
    assert_eq!(b.await, 200);
    assert_eq!(c.await, 300);
}

#[test]
fn test_spawn_sequential_await() {
    let a = spawn(|| 1);
    let r1 = a.await;
    let b = spawn(|| r1 + 1);
    let r2 = b.await;
    assert_eq!(r2, 2);
}

// --- sleep yields to scheduler ---

#[test]
fn test_sleep_inside_spawn() {
    let h = spawn(|| {
        sleep(0);
        99
    });
    assert_eq!(h.await, 99);
}

#[test]
fn test_sleep_multiple_spawns() {
    let a = spawn(|| {
        sleep(0);
        "a"
    });
    let b = spawn(|| {
        sleep(0);
        "b"
    });
    assert_eq!(a.await, "a");
    assert_eq!(b.await, "b");
}

// --- nested spawn ---

#[test]
fn test_nested_spawn() {
    let outer = spawn(|| {
        let inner = spawn(|| 42);
        inner.await
    });
    assert_eq!(outer.await, 42);
}

#[test]
fn test_spawn_chain() {
    let h = spawn(|| {
        let inner = spawn(|| 7);
        inner.await * 6
    });
    assert_eq!(h.await, 42);
}

// --- await on plain values still pass-through ---

#[test]
fn test_await_passthrough_inside_spawn() {
    let h = spawn(|| {
        let x = 42;
        x.await
    });
    assert_eq!(h.await, 42);
}

// --- async fn inside spawn ---

#[test]
fn test_async_fn_inside_spawn() {
    let h = spawn(|| {
        let f = answer();
        f.await
    });
    assert_eq!(h.await, 42);
}

// --- spawn with string return ---

#[test]
fn test_spawn_string_result() {
    let h = spawn(|| "hello".to_string());
    assert_eq!(h.await, "hello");
}

// --- async methods on structs ---

struct Calculator {
    value: int,
}

impl Calculator {
    fn new(v: int) -> Calculator { Calculator { value: v } }

    async fn compute(self) -> int { self.value * 2 }

    async fn add(self, other: int) -> int { self.value + other }

    fn sync_get(self) -> int { self.value }
}

struct Greeter {
    name: String,
}

impl Greeter {
    async fn greet(self) -> String { "Hello, ".to_string() + self.name }

    async fn greet_formal(self, title: String) -> String {
        title + " " + self.name
    }
}

#[test]
fn test_async_method_basic() {
    let c = Calculator::new(21);
    let f = c.compute();
    assert_eq!(f.await, 42);
}

#[test]
fn test_async_method_with_param() {
    let c = Calculator::new(40);
    let f = c.add(2);
    assert_eq!(f.await, 42);
}

#[test]
fn test_async_method_string_return() {
    let g = Greeter { name: "World".to_string() };
    let f = g.greet();
    assert_eq!(f.await, "Hello, World");
}

#[test]
fn test_async_method_multiple_calls() {
    let c1 = Calculator::new(10);
    let c2 = Calculator::new(20);
    let a = c1.compute();
    let b = c2.compute();
    assert_eq!(a.await, 20);
    assert_eq!(b.await, 40);
}

#[test]
fn test_async_method_with_formal_param() {
    let g = Greeter { name: "Smith".to_string() };
    let f = g.greet_formal("Dr.".to_string());
    assert_eq!(f.await, "Dr. Smith");
}

#[test]
fn test_async_method_chain_with_sync() {
    let c = Calculator::new(21);
    assert_eq!(c.sync_get(), 21);
    let f = c.compute();
    assert_eq!(f.await, 42);
}

// --- type-checker: async method .await resolves to correct type ---

#[test]
fn test_async_method_type_flows_to_callee() {
    let c = Calculator::new(42);
    let f = c.compute();       // Future<int>
    let v = f.await;           // int
    let _ = take_int(v);       // OK: int → int
}

// --- compile_error: type mismatch across async method .await ---

#[compile_error]
fn async_method_wrong_type() {
    let g = Greeter { name: "Test".to_string() };
    let f = g.greet();         // Future<String>
    let v = f.await;           // String
    let _ = take_int(v);       // ERROR: String does not match int
}
