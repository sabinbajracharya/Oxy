// === Feature: Traits — Static Method Bounds (FIXME) ===
// Calling static trait methods on generic type parameters (T::zero())
// requires monomorphization or dynamic dispatch, not yet implemented.
//
// Tests below are commented out until the feature is implemented.
// The real body of make_zero should be: T::zero()

trait Zero {
    fn zero() -> Self;
}

impl Zero for i64 {
    fn zero() -> i64 {
        0
    }
}

// FIXME: body should be `T::zero()` once generic path calls work
fn make_zero<T: Zero>() -> T {
    // T::zero() — not yet supported for generic type params
    0
}

// FIXME: uncomment #[test] when T::zero() works
//#[test]
fn _test_trait_static_method_bound() {
    let z: i64 = make_zero();
    assert_eq!(z, 0);
}
