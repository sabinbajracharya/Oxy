// === Feature: Numbers — Arithmetic ===
// Arithmetic operations (+, -, *, /, %) on all integer and float widths.
// Includes compound assignment, negation, operator precedence, integer
// division truncation, and wrapping behaviour at type boundaries.
//
// === Declaration Styles Used ===
//   let x: i8 = 100;        (type annotation)
//   let x = 100i8;           (literal suffix)
//   let x = 100;             (bare literal, i64/f64 by default)

// === Basic Arithmetic on i8 ===

#[test]
fn test_add_i8() {
    assert_eq!(10i8 + 20i8, 30i8);
    assert_eq!((-5i8) + 3i8, -2i8);
    assert_eq!((-5i8) + (-5i8), -10i8);
    assert_eq!(0i8 + 0i8, 0i8);
}

#[test]
fn test_sub_i8() {
    assert_eq!(30i8 - 10i8, 20i8);
    assert_eq!(10i8 - 30i8, -20i8);
    assert_eq!(0i8 - 5i8, -5i8);
}

#[test]
fn test_mul_i8() {
    assert_eq!(6i8 * 7i8, 42i8);
    assert_eq!((-3i8) * 4i8, -12i8);
    assert_eq!((-3i8) * (-4i8), 12i8);
    assert_eq!(0i8 * 127i8, 0i8);
}

#[test]
fn test_div_i8() {
    assert_eq!(42i8 / 6i8, 7i8);
    assert_eq!(10i8 / 3i8, 3i8);  // integer division truncates toward zero
    assert_eq!((-10i8) / 3i8, -3i8);
    assert_eq!(0i8 / 5i8, 0i8);
}

#[test]
fn test_rem_i8() {
    assert_eq!(10i8 % 3i8, 1i8);
    assert_eq!(10i8 % 5i8, 0i8);
    assert_eq!((-10i8) % 3i8, -1i8);
    assert_eq!(0i8 % 5i8, 0i8);
}

// === Basic Arithmetic on Unsigned Types ===

#[test]
fn test_arithmetic_u8() {
    assert_eq!(100u8 + 50u8, 150u8);
    assert_eq!(100u8 - 30u8, 70u8);
    assert_eq!(10u8 * 20u8, 200u8);
    assert_eq!(100u8 / 7u8, 14u8);
    assert_eq!(100u8 % 7u8, 2u8);
}

#[test]
fn test_arithmetic_u32() {
    assert_eq!(100000u32 + 50000u32, 150000u32);
    assert_eq!(1000u32 * 1000u32, 1000000u32);
    assert_eq!(100000u32 / 3u32, 33333u32);
}

#[test]
fn test_arithmetic_u64() {
    assert_eq!(1000000u64 + 1u64, 1000001u64);
    assert_eq!(1000000u64 * 2u64, 2000000u64);
    assert_eq!(999999u64 % 1000u64, 999u64);
}

// === Signed Integer Arithmetic (wider types) ===

#[test]
fn test_arithmetic_i16() {
    assert_eq!(1000i16 + 2000i16, 3000i16);
    assert_eq!(1000i16 * 10i16, 10000i16);
    assert_eq!(20000i16 / 3i16, 6666i16);
}

#[test]
fn test_arithmetic_i32() {
    assert_eq!(100000i32 + 200000i32, 300000i32);
    assert_eq!((-50000i32) + 10000i32, -40000i32);
    assert_eq!(10000i32 * 1000i32, 10000000i32);
}

#[test]
fn test_arithmetic_i64() {
    assert_eq!(1000000000i64 + 1i64, 1000000001i64);
    assert_eq!(1000000i64 * 1000i64, 1000000000i64);
    assert_eq!((-500000i64) / 2i64, -250000i64);
}

// === Float Arithmetic ===

#[test]
fn test_add_f32() {
    assert_eq!(1.5f32 + 2.5f32, 4.0f32);
    assert_eq!(1.0f32 + 2.0f32, 3.0f32);
    assert_eq!(0.5f32 + 0.5f32, 1.0f32);
}

#[test]
fn test_add_f64() {
    assert_eq!(1.5 + 2.5, 4.0);
    assert_eq!((-1.0) + 1.0, 0.0);
}

#[test]
fn test_sub_f64() {
    assert_eq!(5.0 - 3.0, 2.0);
    assert_eq!(0.0 - 1.0, -1.0);
    assert_eq!(10.5 - 0.5, 10.0);
}

#[test]
fn test_mul_f64() {
    assert_eq!(3.0 * 4.0, 12.0);
    assert_eq!((-2.0) * 3.0, -6.0);
    assert_eq!(0.5 * 2.0, 1.0);
    assert_eq!(0.0 * 100.0, 0.0);
}

#[test]
fn test_div_f64() {
    assert_eq!(10.0 / 4.0, 2.5);
    assert_eq!(1.0 / 2.0, 0.5);
    assert_eq!((-6.0) / 2.0, -3.0);
    assert_eq!(0.0 / 5.0, 0.0);
}

// Float remainder

#[test]
fn test_rem_f64() {
    assert_eq!(10.0 % 3.0, 1.0);
    assert_eq!(10.5 % 3.0, 1.5);
    assert_eq!(7.0 % 3.5, 0.0);
}

// === Negation ===

#[test]
fn test_negation_signed() {
    assert_eq!(-(42i8), -42i8);
    assert_eq!(-(-42i8), 42i8);
    assert_eq!(-(0i8), 0i8);
    assert_eq!(-(100i16), -100i16);
    assert_eq!(-(1i32), -1i32);
    assert_eq!(-(500i64), -500i64);
}

#[test]
fn test_negation_float() {
    assert_eq!(-(3.14f32), -3.14f32);
    assert_eq!(-(-3.14f32), 3.14f32);
    assert_eq!(-(1.0), -1.0f64);
    assert_eq!(-(0.0f64), 0.0f64);
}

// === Compound Assignment (+=, -=, *=, /=, %=) ===

#[test]
fn test_compound_add() {
    let mut x = 10i32;
    x += 5i32;
    assert_eq!(x, 15i32);
    x += 0i32;
    assert_eq!(x, 15i32);
}

#[test]
fn test_compound_sub() {
    let mut x = 20i32;
    x -= 5i32;
    assert_eq!(x, 15i32);
    x -= 20i32;
    assert_eq!(x, -5i32);
}

#[test]
fn test_compound_mul() {
    let mut x = 3i32;
    x *= 4i32;
    assert_eq!(x, 12i32);
    x *= 0i32;
    assert_eq!(x, 0i32);
}

#[test]
fn test_compound_div() {
    let mut x = 20i32;
    x /= 4i32;
    assert_eq!(x, 5i32);
    x /= 2i32;
    assert_eq!(x, 2i32);
}

#[test]
fn test_compound_rem() {
    let mut x = 10i32;
    x %= 3i32;
    assert_eq!(x, 1i32);
}

#[test]
fn test_compound_float() {
    let mut x: f64 = 10.0;
    x += 5.0;
    assert_eq!(x, 15.0);
    x -= 3.0;
    assert_eq!(x, 12.0);
    x *= 2.0;
    assert_eq!(x, 24.0);
    x /= 4.0;
    assert_eq!(x, 6.0);
}

// === Operator Precedence ===

#[test]
fn test_precedence_mul_before_add() {
    assert_eq!(2i32 + 3i32 * 4i32, 14i32);
    assert_eq!((2i32 + 3i32) * 4i32, 20i32);
}

#[test]
fn test_precedence_nested() {
    let result = (10i32 + 5i32) * (20i32 - 10i32) / 5i32;
    assert_eq!(result, 30i32);
}

#[test]
fn test_precedence_negation() {
    assert_eq!(-(3i8) + 5i8, 2i8);
    assert_eq!(-(3i8 + 5i8), -8i8);
}

// === Mixed-Width Arithmetic (promotion to wider type) ===

#[test]
fn test_mixed_width_add() {
    // i8 + i16 should promote to i16
    let a: i16 = 10i8 + 20i16;
    assert_eq!(a, 30i16);
}

#[test]
fn test_mixed_width_with_annotation() {
    let a: i32 = 100i8;
    let b: i32 = a + 200i32;
    assert_eq!(b, 300i32);
}

// === Arithmetic Using Type Annotation ===

#[test]
fn test_annotation_style_arithmetic() {
    let a: u32 = 1000;
    let b: u32 = 2000;
    assert_eq!(a + b, 3000u32);
    assert_eq!(b - a, 1000u32);
    assert_eq!(a * 2u32, 2000u32);
}

// === Large Value Arithmetic (near type boundaries) ===

#[test]
fn test_large_i64_arithmetic() {
    let big = 9223372036854775800i64;
    assert_eq!(big + 7i64, 9223372036854775807i64);
    assert_eq!(big - 1i64, 9223372036854775799i64);
}

#[test]
fn test_large_u64_arithmetic() {
    let big = 18446744073709551610u64;
    assert_eq!(big + 5u64, 18446744073709551615u64);
}
