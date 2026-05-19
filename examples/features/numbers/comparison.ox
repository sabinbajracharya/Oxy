// === Feature: Numbers — Comparison ===
// Comparison operators (==, !=, <, >, <=, >=) on all numeric types.
// Cross-width comparison promotes to the wider type. Integer/float
// cross-type comparison works where semantically sound.
//
// === Declaration Styles Used ===
//   let x: i32 = 42;         (type annotation)
//   let x = 42i32;            (literal suffix)

// === Same-Type Equality ===

#[test]
fn test_eq_same_type_i8() {
    assert!(42i8 == 42i8);
    assert!(10i8 != 20i8);
    assert!(!(10i8 == 20i8));
}

#[test]
fn test_eq_same_type_u32() {
    assert!(1000u32 == 1000u32);
    assert!(1000u32 != 2000u32);
}

#[test]
fn test_eq_same_type_f64() {
    assert!(3.14 == 3.14);
    assert!(1.0 != 2.0);
    assert!(!(1.0 == 2.0));
}

#[test]
fn test_eq_zero() {
    assert!(0i8 == 0i8);
    assert!(0u64 == 0u64);
    assert!(0.0f64 == 0.0f64);
}

// === Same-Type Ordering ===

#[test]
fn test_less_than_i32() {
    assert!(5i32 < 10i32);
    assert!(!(10i32 < 5i32));
    assert!(!(10i32 < 10i32));
    assert!((-5i32) < 0i32);
    assert!((-10i32) < -5i32);
}

#[test]
fn test_greater_than_i32() {
    assert!(10i32 > 5i32);
    assert!(!(5i32 > 10i32));
    assert!(!(10i32 > 10i32));
    assert!(0i32 > -5i32);
}

#[test]
fn test_less_equal_i32() {
    assert!(5i32 <= 10i32);
    assert!(10i32 <= 10i32);
    assert!(!(10i32 <= 5i32));
    assert!((-10i32) <= -5i32);
}

#[test]
fn test_greater_equal_i32() {
    assert!(10i32 >= 5i32);
    assert!(10i32 >= 10i32);
    assert!(!(5i32 >= 10i32));
}

// === Ordering on Unsigned ===

#[test]
fn test_ordering_u8() {
    assert!(10u8 < 20u8);
    assert!(200u8 > 100u8);
    assert!(0u8 < 255u8);
    assert!(255u8 >= 255u8);
}

#[test]
fn test_ordering_u32() {
    assert!(1000u32 < 10000u32);
    assert!(50000u32 > 1000u32);
    assert!(0u32 <= 0u32);
}

// === Float Ordering ===

#[test]
fn test_ordering_f64() {
    assert!(1.0 < 2.0);
    assert!(2.0 > 1.0);
    assert!(1.0 <= 1.0);
    assert!(0.0 > -1.0);
    assert!((-3.0) < 0.0);
    assert!((-5.0) < -2.0);
}

// === Cross-Width Comparison ===

#[test]
fn test_cross_width_eq() {
    assert!(42i8 == 42i16);
    assert!(100u8 == 100u32);
    assert!(0i8 == 0i64);
}

#[test]
fn test_cross_width_ordering() {
    assert!(10i8 < 20i64);
    assert!(100u16 > 50u32);
    assert!(0i8 <= 100i64);
}

// === Comparison with Type Annotation ===

#[test]
fn test_annotation_style_comparison() {
    let a: i32 = 100;
    let b: i32 = 200;
    assert!(a < b);
    assert!(b > a);
    assert!(a != b);
    assert!(a == a);
}

// === Edge Cases: Min/Max Comparison ===

#[test]
fn test_min_max_i8_comparison() {
    let min_i8: i8 = -128;
    assert!(min_i8 < 127i8);
    assert!(127i8 > min_i8);
    assert!(min_i8 <= min_i8);
}

#[test]
fn test_min_max_u8_comparison() {
    assert!(0u8 < 255u8);
    assert!(255u8 > 0u8);
    assert!(255u8 >= 255u8);
}

// === Bool result of comparison ===

#[test]
fn test_comparison_returns_bool() {
    let result = 10i32 < 20i32;
    assert!(result);

    let result2 = 10i32 > 20i32;
    assert!(!result2);
}

// === Multiple comparisons in expressions ===

#[test]
fn test_chained_logical_comparisons() {
    let x = 15i32;
    assert!(x > 10i32 && x < 20i32);
    assert!(x >= 15i32 && x <= 15i32);
    assert!(!(x < 10i32 || x > 20i32));
}
