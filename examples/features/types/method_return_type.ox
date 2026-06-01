// === Feature: MethodCall return type resolution ===
// Tests that the type checker infers return types for obj.method() calls.

struct Counter {
    count: Int,
}

impl Counter {
    fn new() -> Counter {
        Counter { count: 0 }
    }

    fn get_count(self) -> Int {
        self.count
    }

    fn add(self, n: Int) -> Int {
        self.count + n
    }
}

#[test]
fn test_method_return_type_resolved() {
    val c = Counter::new();
    val n: Int = c.get_count();
    assert::eq(n, 0);
}

#[test]
fn test_method_return_type_with_args() {
    val c = Counter::new();
    val n: Int = c.add(5);
    assert::eq(n, 5);
}

#[compile_error]
fn test_method_return_type_mismatch() {
    val c = Counter::new();
    val s: String = c.get_count(); // ERROR: Int assigned to String
}
