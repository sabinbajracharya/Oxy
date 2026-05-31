// === Feature: async fn, .await, spawn, and sleep ===
// Tests that async functions return Futures, .await resolves them,
// spawn runs closures eagerly returning JoinHandle, and sleep pauses.

fn main() {}

// --- async fn definitions (must be at top level) ---

async fn answer() -> Int { 42 }

async fn add(a: Int, b: Int) -> Int { a + b }

async fn double(x: Int) -> Int { x * 2 }

async fn greet(name: String) -> String { "Hello, ".to_string() + name }

async fn inner(x: Int) -> Int { x + 1 }

async fn outer(x: Int) -> Int {
    val f = inner(x);
    f.await * 2
}

async fn step1(x: Int) -> Int { x + 1 }

async fn step2(x: Int) -> Int { step1(x).await + 1 }

async fn step3(x: Int) -> Int { step2(x).await + 1 }

fn sync_double(x: Int) -> Int { x * 2 }

// --- async fn basics ---

#[test]
fn test_async_fn_returns_future() {
    val f = answer();
    assert_eq(f.await, 42);
}

#[test]
fn test_async_fn_with_params() {
    val f = add(3, 4);
    assert_eq(f.await, 7);
}

#[test]
fn test_async_fn_multiple_calls() {
    val a = double(5);
    val b = double(10);
    assert_eq(a.await, 10);
    assert_eq(b.await, 20);
}

#[test]
fn test_async_fn_string_return() {
    val f = greet("World".to_string());
    assert_eq(f.await, "Hello, World");
}

// --- await passthrough ---

#[test]
fn test_await_on_plain_value_passes_through() {
    val x = 42;
    assert_eq(x.await, 42);
}

#[test]
fn test_await_on_string_passes_through() {
    val s = "hello".to_string();
    assert_eq(s.await, "hello");
}

// --- spawn ---

#[test]
fn test_spawn_returns_join_handle() {
    val h = spawn(|| 42);
    assert_eq(h.await, 42);
}

#[test]
fn test_spawn_with_capture() {
    val x = 10;
    val h = spawn(|| x * 2);
    assert_eq(h.await, 20);
}

#[test]
fn test_spawn_multiple() {
    val a = spawn(|| 100);
    val b = spawn(|| 200);
    assert_eq(a.await, 100);
    assert_eq(b.await, 200);
}

// --- sleep ---

#[test]
fn test_sleep_runs_without_error() {
    val _ = sleep(10);
}

// --- nested async ---

#[test]
fn test_nested_async_calls() {
    val f = outer(5);
    assert_eq(f.await, 12);
}

#[test]
fn test_async_fn_chain() {
    assert_eq(step3(1).await, 4);
}

// --- await on non-Future from fn call ---

#[test]
fn test_await_on_sync_fn_result() {
    val v = sync_double(21);
    assert_eq(v.await, 42);
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

fn take_int(x: Int) -> Int { x }

fn take_string(s: String) -> String { s }

#[test]
fn test_await_future_type_flows_to_callee() {
    val f = answer();          // Future<Int>
    val v = f.await;           // Int (unwrapped by type checker)
    val _ = take_int(v);       // OK: Int → Int
}

#[test]
fn test_await_spawn_type_flows_to_callee() {
    val h = spawn(|| 42);      // JoinHandle<Int>
    val v = h.await;           // Int
    val _ = take_int(v);       // OK
}

#[test]
fn test_await_plain_value_passthrough() {
    val x = 42;
    val v = x.await;           // passthrough: Int
    val _ = take_int(v);       // OK
}

// --- compile_error: type mismatch across .await ---

#[compile_error]
fn await_future_wrong_type() {
    val f = answer();          // Future<Int>
    val v = f.await;           // Int
    val _ = take_string(v);    // ERROR: Int does not match String
}

#[compile_error]
fn await_spawn_wrong_type() {
    val h = spawn(|| 42);      // JoinHandle<Int>
    val v = h.await;           // Int
    val _ = take_string(v);    // ERROR: Int does not match String
}

// --- event-loop spawn: correctness ---

#[test]
fn test_spawn_basic() {
    val h = spawn(|| 42);
    assert_eq(h.await, 42);
}

#[test]
fn test_spawn_with_captured_var() {
    val x = 10;
    val h = spawn(|| x * 3);
    assert_eq(h.await, 30);
}

#[test]
fn test_spawn_multiple_independent() {
    val a = spawn(|| 100);
    val b = spawn(|| 200);
    val c = spawn(|| 300);
    // Results are collected in any order
    assert_eq(a.await, 100);
    assert_eq(b.await, 200);
    assert_eq(c.await, 300);
}

#[test]
fn test_spawn_sequential_await() {
    val a = spawn(|| 1);
    val r1 = a.await;
    val b = spawn(|| r1 + 1);
    val r2 = b.await;
    assert_eq(r2, 2);
}

// --- sleep yields to scheduler ---

#[test]
fn test_sleep_inside_spawn() {
    val h = spawn(|| {
        sleep(0);
        99
    });
    assert_eq(h.await, 99);
}

#[test]
fn test_sleep_multiple_spawns() {
    val a = spawn(|| {
        sleep(0);
        "a"
    });
    val b = spawn(|| {
        sleep(0);
        "b"
    });
    assert_eq(a.await, "a");
    assert_eq(b.await, "b");
}

// --- nested spawn ---

#[test]
fn test_nested_spawn() {
    val outer = spawn(|| {
        val inner = spawn(|| 42);
        inner.await
    });
    assert_eq(outer.await, 42);
}

#[test]
fn test_spawn_chain() {
    val h = spawn(|| {
        val inner = spawn(|| 7);
        inner.await * 6
    });
    assert_eq(h.await, 42);
}

// --- await on plain values still pass-through ---

#[test]
fn test_await_passthrough_inside_spawn() {
    val h = spawn(|| {
        val x = 42;
        x.await
    });
    assert_eq(h.await, 42);
}

// --- async fn inside spawn ---

#[test]
fn test_async_fn_inside_spawn() {
    val h = spawn(|| {
        val f = answer();
        f.await
    });
    assert_eq(h.await, 42);
}

// --- spawn with string return ---

#[test]
fn test_spawn_string_result() {
    val h = spawn(|| "hello".to_string());
    assert_eq(h.await, "hello");
}

// --- async methods on structs ---

struct Calculator {
    value: Int,
}

impl Calculator {
    fn new(v: Int) -> Calculator { Calculator { value: v } }

    async fn compute(self) -> Int { self.value * 2 }

    async fn add(self, other: Int) -> Int { self.value + other }

    fn sync_get(self) -> Int { self.value }
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
    val c = Calculator::new(21);
    val f = c.compute();
    assert_eq(f.await, 42);
}

#[test]
fn test_async_method_with_param() {
    val c = Calculator::new(40);
    val f = c.add(2);
    assert_eq(f.await, 42);
}

#[test]
fn test_async_method_string_return() {
    val g = Greeter { name: "World".to_string() };
    val f = g.greet();
    assert_eq(f.await, "Hello, World");
}

#[test]
fn test_async_method_multiple_calls() {
    val c1 = Calculator::new(10);
    val c2 = Calculator::new(20);
    val a = c1.compute();
    val b = c2.compute();
    assert_eq(a.await, 20);
    assert_eq(b.await, 40);
}

#[test]
fn test_async_method_with_formal_param() {
    val g = Greeter { name: "Smith".to_string() };
    val f = g.greet_formal("Dr.".to_string());
    assert_eq(f.await, "Dr. Smith");
}

#[test]
fn test_async_method_chain_with_sync() {
    val c = Calculator::new(21);
    assert_eq(c.sync_get(), 21);
    val f = c.compute();
    assert_eq(f.await, 42);
}

// --- type-checker: async method .await resolves to correct type ---

#[test]
fn test_async_method_type_flows_to_callee() {
    val c = Calculator::new(42);
    val f = c.compute();       // Future<Int>
    val v = f.await;           // Int
    val _ = take_int(v);       // OK: Int → Int
}

// --- compile_error: type mismatch across async method .await ---

#[compile_error]
fn async_method_wrong_type() {
    val g = Greeter { name: "Test".to_string() };
    val f = g.greet();         // Future<String>
    val v = f.await;           // String
    val _ = take_int(v);       // ERROR: String does not match Int
}
