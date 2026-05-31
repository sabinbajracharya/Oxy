// === Feature: Structs — Impl Blocks ===
// `impl` blocks add methods to structs. Methods can take `self`
// (immutable reference) for read-only access, or `self` for ownership
// transfer. Associated functions (no `self`) act as constructors.

struct Counter {
    count: Int,
}

impl Counter {
    fn new(initial: Int) -> Counter {
        Counter { count: initial }
    }

    fn value(self) -> Int {
        self.count
    }

    fn increment(self) -> Counter {
        Counter { count: self.count + 1 }
    }

    fn reset(self) {
        // Mutable mutation not supported without mut self,
        // so return a new value instead
        val _ = Counter { count: 0 };
    }
}

#[test]
fn test_impl_constructor() {
    val c = Counter::new(10);
    assert_eq(c.value(), 10);
}

#[test]
fn test_impl_method_self() {
    val c = Counter::new(5);
    assert_eq(c.value(), 5);
}

#[test]
fn test_impl_method_chaining() {
    val c = Counter::new(1)
        .increment()
        .increment()
        .increment();
    assert_eq(c.value(), 4);
}

// === Impl with Multiple Methods ===

struct Rect {
    width: Int,
    height: Int,
}

impl Rect {
    fn area(self) -> Int {
        self.width * self.height
    }

    fn perimeter(self) -> Int {
        2 * (self.width + self.height)
    }
}

#[test]
fn test_rect_methods() {
    val r = Rect { width: 10, height: 5 };
    assert_eq(r.area(), 50);
    assert_eq(r.perimeter(), 30);
}

// === Self Type in Return ===

struct Wrapper {
    value: Int,
}

impl Wrapper {
    fn wrap(v: Int) -> Wrapper {
        Wrapper { value: v }
    }

    fn unwrap(self) -> Int {
        self.value
    }
}

#[test]
fn test_self_return_type() {
    val w = Wrapper::wrap(42);
    assert_eq(w.unwrap(), 42);
}
