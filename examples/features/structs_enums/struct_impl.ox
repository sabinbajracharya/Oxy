// === Feature: Structs — Impl Blocks ===
// `impl` blocks add methods to structs. Methods can take `self`
// (immutable reference) for read-only access, or `self` for ownership
// transfer. Associated functions (no `self`) act as constructors.

struct Counter {
    count: int,
}

impl Counter {
    fn new(initial: int) -> Counter {
        Counter { count: initial }
    }

    fn value(self) -> int {
        self.count
    }

    fn increment(self) -> Counter {
        Counter { count: self.count + 1 }
    }

    fn reset(self) {
        // Mutable mutation not supported without mut self,
        // so return a new value instead
        let _ = Counter { count: 0 };
    }
}

#[test]
fn test_impl_constructor() {
    let c = Counter::new(10);
    assert_eq!(c.value(), 10);
}

#[test]
fn test_impl_method_self() {
    let c = Counter::new(5);
    assert_eq!(c.value(), 5);
}

#[test]
fn test_impl_method_chaining() {
    let c = Counter::new(1)
        .increment()
        .increment()
        .increment();
    assert_eq!(c.value(), 4);
}

// === Impl with Multiple Methods ===

struct Rect {
    width: int,
    height: int,
}

impl Rect {
    fn area(self) -> int {
        self.width * self.height
    }

    fn perimeter(self) -> int {
        2 * (self.width + self.height)
    }
}

#[test]
fn test_rect_methods() {
    let r = Rect { width: 10, height: 5 };
    assert_eq!(r.area(), 50);
    assert_eq!(r.perimeter(), 30);
}

// === Self Type in Return ===

struct Wrapper {
    value: int,
}

impl Wrapper {
    fn wrap(v: int) -> Wrapper {
        Wrapper { value: v }
    }

    fn unwrap(self) -> int {
        self.value
    }
}

#[test]
fn test_self_return_type() {
    let w = Wrapper::wrap(42);
    assert_eq!(w.unwrap(), 42);
}
