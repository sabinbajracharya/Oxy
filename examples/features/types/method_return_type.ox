// === Feature: MethodCall return type resolution ===
// Tests that the type checker infers return types for obj.method() calls.

struct Counter {
    count: int,
}

impl Counter {
    fn new() -> Counter {
        Counter { count: 0 }
    }

    fn get_count(self) -> int {
        self.count
    }

    fn add(self, n: int) -> int {
        self.count + n
    }
}

#[test]
fn test_method_return_type_resolved() {
    let c = Counter::new();
    let n: int = c.get_count();
    assert_eq(n, 0);
}

#[test]
fn test_method_return_type_with_args() {
    let c = Counter::new();
    let n: int = c.add(5);
    assert_eq(n, 5);
}

#[compile_error]
fn test_method_return_type_mismatch() {
    let c = Counter::new();
    let s: String = c.get_count(); // ERROR: int assigned to String
}
