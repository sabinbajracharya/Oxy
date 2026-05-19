// === Feature: Numbers — Type Casts ===
// The `as` operator converts between numeric types. Widening casts are
// lossless. Narrowing casts truncate/wrap. Float→int truncates toward zero.
// Int→float may lose precision for very large values.
//
// === Declaration Styles Used ===
//   let x: i8 = 42;          (type annotation)
//   let x = 42i8;             (literal suffix)
//   let x = 42;               (bare literal)

// === Widening Casts (always safe) ===

#[test]
fn test_widen_i8_to_i16() {
    assert_eq!(42i8 as i16, 42i16);
    assert_eq!((-5i8) as i16, -5i16);
}

#[test]
fn test_widen_i8_to_i32() {
    assert_eq!(100i8 as i32, 100i32);
    assert_eq!((-1i8) as i32, -1i32);
}

#[test]
fn test_widen_i8_to_i64() {
    assert_eq!(127i8 as i64, 127i64);
    let min_i8: i8 = -128;
    assert_eq!(min_i8 as i64, -128i64);
}

#[test]
fn test_widen_i16_to_i32() {
    assert_eq!(1000i16 as i32, 1000i32);
}

#[test]
fn test_widen_i16_to_i64() {
    assert_eq!(32767i16 as i64, 32767i64);
}

#[test]
fn test_widen_i32_to_i64() {
    assert_eq!(100000i32 as i64, 100000i64);
}

#[test]
fn test_widen_unsigned() {
    assert_eq!(100u8 as u16, 100u16);
    assert_eq!(255u8 as u32, 255u32);
    assert_eq!(1000u16 as u32, 1000u32);
    assert_eq!(65535u16 as u64, 65535u64);
    assert_eq!(1000000u32 as u64, 1000000u64);
}

#[test]
fn test_widen_f32_to_f64() {
    assert_eq!(1.5f32 as f64, 1.5);
    assert_eq!((-2.0f32) as f64, -2.0);
    assert_eq!(0.0f32 as f64, 0.0);
    assert_eq!(0.5f32 as f64, 0.5);
}

// === Narrowing Casts (truncate / wrap) ===

#[test]
fn test_narrow_i64_to_i32() {
    assert_eq!(100000i64 as i32, 100000i32);
}

#[test]
fn test_narrow_i32_to_i16() {
    assert_eq!(1000i32 as i16, 1000i16);
}

#[test]
fn test_narrow_i16_to_i8() {
    assert_eq!(100i16 as i8, 100i8);
}

#[test]
fn test_narrow_i64_to_i8_wraps() {
    // 300 as i8 = 300 % 256 = 44 (two's complement)
    assert_eq!(300i64 as i8, 44i8);
    // 256 as i8 = 0 (wraps)
    assert_eq!(256i64 as i8, 0i8);
    // 128 as i8 = -128 (two's complement)
    let min_i8: i8 = -128;
    assert_eq!(128i64 as i8, min_i8);
    // 255 as i8 = -1
    assert_eq!(255i64 as i8, -1i8);
}

#[test]
fn test_narrow_u64_to_u8_wraps() {
    assert_eq!(300u64 as u8, 44u8);
    assert_eq!(256u64 as u8, 0u8);
    assert_eq!(255u64 as u8, 255u8);
    assert_eq!(0u64 as u8, 0u8);
}

#[test]
fn test_narrow_u16_to_u8() {
    assert_eq!(100u16 as u8, 100u8);
    assert_eq!(256u16 as u8, 0u8);
    assert_eq!(257u16 as u8, 1u8);
}

#[test]
fn test_narrow_u32_to_u16() {
    assert_eq!(65535u32 as u16, 65535u16);
    assert_eq!(65536u32 as u16, 0u16);
}

// === Signed / Unsigned Same-Width Casts ===

#[test]
fn test_signed_to_unsigned_same_width() {
    assert_eq!(42i8 as u8, 42u8);
    assert_eq!((-1i8) as u8, 255u8);
    let min_i8: i8 = -128;
    assert_eq!(min_i8 as u8, 128u8);
    assert_eq!(0i8 as u8, 0u8);
}

#[test]
fn test_unsigned_to_signed_same_width() {
    assert_eq!(42u8 as i8, 42i8);
    let min_i8: i8 = -128;
    assert_eq!(128u8 as i8, min_i8);
    assert_eq!(255u8 as i8, -1i8);
    assert_eq!(0u8 as i8, 0i8);
}

#[test]
fn test_i32_to_u32_cast() {
    assert_eq!(100i32 as u32, 100u32);
    assert_eq!((-1i32) as u32, 4294967295u32);
    assert_eq!(0i32 as u32, 0u32);
}

#[test]
fn test_u64_to_i64_cast() {
    assert_eq!(100u64 as i64, 100i64);
    assert_eq!(0u64 as i64, 0i64);
    // u64::MAX as i64 = -1 (two's complement wraparound)
    assert_eq!(18446744073709551615u64 as i64, (-1i64));
}

// === Float / Integer Casts ===

#[test]
fn test_float_to_int_truncates_toward_zero() {
    assert_eq!(3.7f64 as i64, 3i64);
    assert_eq!((-3.7f64) as i64, -3i64);
    assert_eq!(0.9f64 as i64, 0i64);
    assert_eq!(3.0f64 as i64, 3i64);
}

#[test]
fn test_float_to_int_narrow() {
    assert_eq!(3.14f64 as i32, 3i32);
    assert_eq!(3.7f32 as i8, 3i8);
}

#[test]
fn test_int_to_float() {
    assert_eq!(42i64 as f64, 42.0f64);
    assert_eq!((-10i32) as f64, -10.0);
    assert_eq!(0i8 as f64, 0.0);
}

#[test]
fn test_int_to_float_then_back() {
    // Small ints survive float round-trip
    let x = 42i64 as f64 as i64;
    assert_eq!(x, 42i64);
}

// === Using Type Annotations with Casts ===

#[test]
fn test_annotation_style_casts() {
    let a: i32 = 1000;
    let b: i16 = a as i16;
    assert_eq!(b, 1000i16);

    let c: u8 = 200;
    let d: i8 = c as i8;
    assert_eq!(d, (-56i8));
}

// === Chained Casts ===

#[test]
fn test_chained_casts() {
    assert_eq!((300u16 as u8) as i8, 44i8);
    assert_eq!(((-1i8) as u8) as i16, 255i16);
}
