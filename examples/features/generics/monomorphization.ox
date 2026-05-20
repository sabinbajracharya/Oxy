// === Feature: Generics — Monomorphization ===
// Generic functions with turbofish compile a separate copy for each
// concrete type, ensuring correct static method dispatch.

// === Monomorphization: Single Type Arg, Single Impl ===

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
fn test_mono_single_impl() {
    let z = make_zero::<i64>();
    assert_eq!(z, 0);
}

// === Monomorphization: Single Type Arg, Multiple Impls ===

impl Zero for f64 {
    fn zero() -> f64 {
        0.0
    }
}

#[test]
fn test_mono_multi_impl_different_types() {
    let i: i64 = make_zero::<i64>();
    let f: f64 = make_zero::<f64>();
    assert_eq!(i, 0);
    assert_eq!(f, 0.0);
}

// === Monomorphization: Deduplication ===
// Same turbofish called twice should use the same monomorphized copy.

#[test]
fn test_mono_dedup() {
    let a = make_zero::<i64>();
    let b = make_zero::<i64>();
    assert_eq!(a, 0);
    assert_eq!(b, 0);
}

// === Monomorphization: Multiple Type Args ===

trait DefaultValue {
    fn default_val() -> Self;
}

impl DefaultValue for i64 {
    fn default_val() -> i64 {
        42
    }
}

impl DefaultValue for String {
    fn default_val() -> String {
        "hello".to_string()
    }
}

struct Pair<A, B> {
    first: A,
    second: B,
}

fn make_pair<A, B>() -> Pair<A, B>
where
    A: DefaultValue,
    B: DefaultValue,
{
    Pair {
        first: A::default_val(),
        second: B::default_val(),
    }
}

#[test]
fn test_mono_multi_type_args() {
    let p = make_pair::<i64, String>();
    assert_eq!(p.first, 42);
    assert_eq!(p.second, "hello");
}

// === Monomorphization: Mixed Turbofish and Inference ===

fn identity<T>(x: T) -> T {
    x
}

#[test]
fn test_mono_with_inference() {
    let a = identity::<i64>(10);
    let b = identity("hello".to_string());
    assert_eq!(a, 10);
    assert_eq!(b, "hello");
}
