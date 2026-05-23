// === Feature: Traits — Impl Type Arguments ===
// Tests that impl headers can specify concrete type arguments,
// e.g. `impl MyStruct<int>` (inherent) and `impl From<int> for MyType` (trait).

// === Inherent impl with concrete type args ===

struct Pair<A, B> {
    first: A,
    second: B,
}

impl Pair<int, int> {
    fn sum(self) -> int {
        self.first + self.second
    }

    fn make(a: int, b: int) -> Pair<int, int> {
        Pair { first: a, second: b }
    }
}

#[test]
fn test_inherent_impl_with_type_args() {
    let p = Pair::<int, int>::make(10, 20);
    assert_eq!(p.sum(), 30);
}

// === Inherent impl on non-generic struct (no type args) still works ===

struct Point {
    x: float,
    y: float,
}

impl Point {
    fn origin() -> Point {
        Point { x: 0.0, y: 0.0 }
    }
}

#[test]
fn test_regular_impl_still_works() {
    let p = Point::origin();
    assert_eq!(p.x, 0.0);
    assert_eq!(p.y, 0.0);
}
