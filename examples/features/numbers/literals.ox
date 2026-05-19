// === Feature: Numbers — Literals ===
// Integer and float literal syntax in Oxy. Covers all widths (i8–i64,
// u8–u64, f32, f64), radix prefixes (0x, 0o, 0b), underscore separators,
// and scientific notation.
//
// === Declaration Styles ===
// There are three ways to give a number a specific type:
//   1. Type annotation on the let binding:  let x: i8 = 42;
//   2. Suffix on the literal itself:        let x = 42i8;
//   3. Bare literal (inferred as i64/f64):  let x = 42;
//
// These styles are demonstrated throughout all test functions.

// === Signed Integer Suffixes ===

#[test]
fn test_signed_suffix_i8() {
    assert_eq!(42i8, 42i8);
    assert_eq!((-5i8), -5i8);
    assert_eq!(0i8, 0i8);
}

#[test]
fn test_signed_suffix_i16() {
    assert_eq!(42i16, 42i16);
    assert_eq!(0i16, 0i16);
}

#[test]
fn test_signed_suffix_i32() {
    assert_eq!(42i32, 42i32);
    assert_eq!(0i32, 0i32);
}

#[test]
fn test_signed_suffix_i64() {
    assert_eq!(42i64, 42i64);
    assert_eq!(0i64, 0i64);
}

// === Unsigned Integer Suffixes ===

#[test]
fn test_unsigned_suffix_u8() {
    assert_eq!(42u8, 42u8);
    assert_eq!(0u8, 0u8);
    assert_eq!(255u8, 255u8);
}

#[test]
fn test_unsigned_suffix_u16() {
    assert_eq!(42u16, 42u16);
    assert_eq!(0u16, 0u16);
}

#[test]
fn test_unsigned_suffix_u32() {
    assert_eq!(42u32, 42u32);
    assert_eq!(0u32, 0u32);
}

#[test]
fn test_unsigned_suffix_u64() {
    assert_eq!(42u64, 42u64);
    assert_eq!(0u64, 0u64);
}

// === Float Suffixes ===

#[test]
fn test_float_suffix_f32() {
    assert_eq!(3.14f32, 3.14f32);
    assert_eq!(0.0f32, 0.0f32);
    assert_eq!((-1.5f32), -1.5f32);
}

#[test]
fn test_float_suffix_f64() {
    assert_eq!(3.14f64, 3.14f64);
    assert_eq!(0.0f64, 0.0f64);
    assert_eq!((-1.5f64), -1.5f64);
}

// === Type Annotation on Let Binding ===

#[test]
fn test_type_annotation_let() {
    let a: i8 = 42;
    assert_eq!(a, 42i8);

    let b: u32 = 100;
    assert_eq!(b, 100u32);

    let c: f64 = 3.14;
    assert_eq!(c, 3.14);
}

// === Bare Literals (Default Types) ===

#[test]
fn test_bare_integer_defaults_to_i64() {
    let a = 42;
    assert_eq!(a, 42i64);

    let big = 1000000;
    assert_eq!(big, 1000000i64);
}

#[test]
fn test_bare_float_defaults_to_f64() {
    let a = 3.14;
    assert_eq!(a, 3.14f64);

    let b = 0.0;
    assert_eq!(b, 0.0f64);
}

// === Hex Literals (0x prefix) ===

#[test]
fn test_hex_literals() {
    assert_eq!(0x7Fi8, 127i8);
    assert_eq!(0x0Au8, 10u8);
    assert_eq!(0xFFFFu16, 65535u16);
    assert_eq!(0xDEADBEEFu32, 3735928559u32);
}

// === Octal Literals (0o prefix) ===

#[test]
fn test_octal_literals() {
    assert_eq!(0o77u8, 63u8);
    assert_eq!(0o10i32, 8i32);
    assert_eq!(0o0u16, 0u16);
}

// === Binary Literals (0b prefix) ===

#[test]
fn test_binary_literals() {
    assert_eq!(0b1010u8, 10u8);
    assert_eq!(0b11111111u8, 255u8);
    assert_eq!(0b0i64, 0i64);
}

// === Underscore Separators ===

#[test]
fn test_underscore_separators() {
    let a = 1_000_000i64;
    assert_eq!(a, 1000000i64);

    let b = 0xDEAD_BEEFu32;
    assert_eq!(b, 3735928559u32);

    let c = 0b1111_0000u8;
    assert_eq!(c, 240u8);

    let d = 0o77_77u16;
    assert_eq!(d, 4095u16);
}

// === Negative Literals ===

#[test]
fn test_negative_literals() {
    assert_eq!((-42i8), -42i8);
    assert_eq!((-1i16), -1i16);
    assert_eq!((-100i32), -100i32);
    assert_eq!((-1i64), -1i64);
    assert_eq!((-3.5f32), -3.5f32);
    assert_eq!((-0.5f64), -0.5f64);
}

// === Zero Values ===

#[test]
fn test_zero_values_all_widths() {
    assert_eq!(0i8, 0i8);
    assert_eq!(0i16, 0i16);
    assert_eq!(0i32, 0i32);
    assert_eq!(0i64, 0i64);
    assert_eq!(0u8, 0u8);
    assert_eq!(0u16, 0u16);
    assert_eq!(0u32, 0u32);
    assert_eq!(0u64, 0u64);
    assert_eq!(0.0f32, 0.0f32);
    assert_eq!(0.0f64, 0.0f64);
}

// === Scientific Notation (Floats) ===

#[test]
fn test_scientific_notation() {
    let a = 1.5e3f64;
    assert_eq!(a, 1500.0f64);

    let b = 1.0e-3f64;
    assert_eq!(b, 0.001f64);

    let c = 3.14e0f32;
    assert_eq!(c, 3.14f32);
}

// === Maximum / Minimum Boundary Values ===

#[test]
fn test_i8_boundaries() {
    assert_eq!(127i8, 127i8);
    let min_i8: i8 = -128;
    assert_eq!(min_i8, min_i8);
}

#[test]
fn test_i16_boundaries() {
    assert_eq!(32767i16, 32767i16);
    let min_i16: i16 = -32768;
    assert_eq!(min_i16, min_i16);
}

#[test]
fn test_u8_boundaries() {
    assert_eq!(255u8, 255u8);
    assert_eq!(0u8, 0u8);
}

#[test]
fn test_u16_boundaries() {
    assert_eq!(65535u16, 65535u16);
    assert_eq!(0u16, 0u16);
}

// === Large Values for Wider Types ===

#[test]
fn test_large_u64_literal() {
    let a = 18446744073709551615u64;
    assert_eq!(a, 18446744073709551615u64);
}

#[test]
fn test_large_i64_literal() {
    let a = 9223372036854775807i64;
    assert_eq!(a, 9223372036854775807i64);
}

// === Annotation + Suffix Combination ===

#[test]
fn test_annotation_with_suffixed_literal() {
    let a: i32 = 42i16;
    assert_eq!(a, 42i32);

    let b: i64 = 100u8;
    assert_eq!(b, 100i64);

    let c: f64 = 3.0f32;
    assert_eq!(c, 3.0f64);
}
