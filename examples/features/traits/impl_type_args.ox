// === Feature: Traits — Impl Type Arguments ===
// Tests that impl headers can specify concrete type arguments,
// e.g. `impl MyStruct<Int>` (inherent) and `impl From<Int> for MyType` (trait).

// === Inherent impl with concrete type args ===

struct Pair<A, B> {
    first: A,
    second: B,
}

impl Pair<Int, Int> {
    fn sum(self) -> Int {
        self.first + self.second
    }

    fn make(a: Int, b: Int) -> Pair<Int, Int> {
        Pair { first: a, second: b }
    }
}

#[test]
fn test_inherent_impl_with_type_args() {
    val p = Pair::<Int, Int>::make(10, 20);
    assert::eq(p.sum(), 30);
}

// === Inherent impl on non-generic struct (no type args) still works ===

struct Point {
    x: Float,
    y: Float,
}

impl Point {
    fn origin() -> Point {
        Point { x: 0.0, y: 0.0 }
    }
}

#[test]
fn test_regular_impl_still_works() {
    val p = Point::origin();
    assert::eq(p.x, 0.0);
    assert::eq(p.y, 0.0);
}
