// === Feature: Generics — Monomorphization ===
// Generic functions with turbofish compile a separate copy for each
// concrete type, ensuring correct static method dispatch.

// === Monomorphization: Single Type Arg, Single Impl ===

trait Zero {
    fn zero() -> Self;
}

impl Zero for Int {
    fn zero() -> Int {
        0
    }
}

fn make_zero<T: Zero>() -> T {
    T::zero()
}

#[test]
fn test_mono_single_impl() {
    let z = make_zero::<Int>();
    assert_eq(z, 0);
}

// === Monomorphization: Single Type Arg, Multiple Impls ===

impl Zero for Float {
    fn zero() -> Float {
        0.0
    }
}

#[test]
fn test_mono_multi_impl_different_types() {
    let i: Int = make_zero::<Int>();
    let f: Float = make_zero::<Float>();
    assert_eq(i, 0);
    assert_eq(f, 0.0);
}

// === Monomorphization: Deduplication ===
// Same turbofish called twice should use the same monomorphized copy.

#[test]
fn test_mono_dedup() {
    let a = make_zero::<Int>();
    let b = make_zero::<Int>();
    assert_eq(a, 0);
    assert_eq(b, 0);
}

// === Monomorphization: Multiple Type Args ===

trait DefaultValue {
    fn default_val() -> Self;
}

impl DefaultValue for Int {
    fn default_val() -> Int {
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

fn make_pair<A: DefaultValue, B: DefaultValue>() -> Pair<A, B>
{
    Pair {
        first: A::default_val(),
        second: B::default_val(),
    }
}

#[test]
fn test_mono_multi_type_args() {
    let p = make_pair::<Int, String>();
    assert_eq(p.first, 42);
    assert_eq(p.second, "hello");
}

// === Monomorphization: Mixed Turbofish and Inference ===

fn identity<T>(x: T) -> T {
    x
}

#[test]
fn test_mono_with_inference() {
    let a = identity::<Int>(10);
    let b = identity("hello".to_string());
    assert_eq(a, 10);
    assert_eq(b, "hello");
}
