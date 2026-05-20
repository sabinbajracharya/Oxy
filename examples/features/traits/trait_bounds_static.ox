// === Feature: Traits — Static Method Bounds ===
// Static trait methods on generic type parameters (T::zero())
// resolve through trait bound resolution at compile time.

trait Zero {
    fn zero() -> Self;
}

impl Zero for i64 {
    fn zero() -> i64 {
        0
    }
}

fn make_zero<T: Zero>() -> T {
    T::zero()
}

#[test]
fn test_trait_static_method_bound() {
    let z = make_zero::<i64>();
    assert_eq!(z, 0);
}
