// === Feature: Numbers — Overflow & Wrapping ===
// Integer arithmetic in Oxy uses two's complement wrapping, matching Rust's
// behaviour in release mode. When a value exceeds its type's range:
//   - Signed: wraps via two's complement (e.g. 127i8 + 1 = -128)
//   - Unsigned: wraps modulo 2^N (e.g. 255u8 + 1 = 0)
//
// Compile-time errors: literal out of range for declared type is rejected
// at compile time (e.g. let x: i8 = 234234 is an error).
//
// Note: `-128i8` as a suffixed literal is rejected because 128 > i8::MAX.
// Use type annotation `let x: i8 = -128;` to get the minimum value.
//
// === Declaration Styles Used ===
//   let x: i8 = 100;        (type annotation)
//   let x = 100i8;           (literal suffix)

// === Signed Addition Wrapping ===

#[test]
fn test_i8_add_overflow_wraps() {
    let a: i8 = 127;
    let min_i8: i8 = -128;
    assert_eq!(a + 1i8, min_i8);  // i8::MAX + 1 wraps to i8::MIN
}

#[test]
fn test_i8_add_overflow_with_suffix() {
    let min_i8: i8 = -128;
    assert_eq!(127i8 + 1i8, min_i8);
    assert_eq!(127i8 + 2i8, -127i8);
}

#[test]
fn test_i16_add_overflow() {
    let min_i16: i16 = -32768;
    assert_eq!(32767i16 + 1i16, min_i16);
}

// === Signed Subtraction Wrapping ===

#[test]
fn test_i8_sub_overflow_wraps() {
    let min_i8: i8 = -128;
    assert_eq!(min_i8 - 1i8, 127i8);     // i8::MIN - 1 wraps to i8::MAX
}

#[test]
fn test_i8_sub_overflow_with_suffix() {
    let min_i8: i8 = -128;
    assert_eq!(min_i8 - 1i8, 127i8);
    assert_eq!(min_i8 - 2i8, 126i8);
}

#[test]
fn test_i16_sub_overflow() {
    let min_i16: i16 = -32768;
    assert_eq!(min_i16 - 1i16, 32767i16);
}

// === Signed Multiplication Wrapping ===

#[test]
fn test_i8_mul_overflow_wraps() {
    let min_i8: i8 = -128;
    assert_eq!(64i8 * 2i8, min_i8);  // 64*2=128, wraps to -128 in i8
    assert_eq!(16i8 * 8i8, min_i8);  // 16*8=128, wraps to -128 in i8
}

#[test]
fn test_i8_mul_double_overflow() {
    // 100 * 2 = 200, which as i8 = 200 - 256 = -56
    assert_eq!(100i8 * 2i8, -56i8);
}

// === Unsigned Addition Wrapping ===

#[test]
fn test_u8_add_overflow_wraps() {
    let a: u8 = 255;
    assert_eq!(a + 1u8, 0u8);        // u8::MAX + 1 wraps to 0
    assert_eq!(a + 2u8, 1u8);
}

#[test]
fn test_u8_add_overflow_with_suffix() {
    assert_eq!(255u8 + 1u8, 0u8);
    assert_eq!(250u8 + 10u8, 4u8);
}

#[test]
fn test_u16_add_overflow() {
    assert_eq!(65535u16 + 1u16, 0u16);
}

// === Unsigned Subtraction Wrapping ===

#[test]
fn test_u8_sub_overflow_wraps() {
    assert_eq!(0u8 - 1u8, 255u8);      // 0u8 - 1 wraps to u8::MAX
    assert_eq!(0u8 - 2u8, 254u8);
}

#[test]
fn test_u8_sub_overflow_with_suffix() {
    assert_eq!(0u8 - 1u8, 255u8);
    assert_eq!(0u8 - 5u8, 251u8);
}

#[test]
fn test_u16_sub_overflow() {
    assert_eq!(0u16 - 1u16, 65535u16);
}

// === Unsigned Multiplication Wrapping ===

#[test]
fn test_u8_mul_overflow_wraps() {
    assert_eq!(16u8 * 16u8, 0u8);    // 256 wraps to 0
    assert_eq!(16u8 * 17u8, 16u8);   // 272 wraps to 16
    assert_eq!(100u8 * 3u8, 44u8);   // 300 wraps to 44
}

// === Negation of MIN Value (two's complement edge case) ===

#[test]
fn test_negate_i8_min() {
    // -i8::MIN = i8::MIN in two's complement
    let min_i8: i8 = -128;
    assert_eq!(-min_i8, min_i8);
}

#[test]
fn test_negate_i16_min() {
    let min_i16: i16 = -32768;
    assert_eq!(-min_i16, min_i16);
}

#[test]
fn test_negate_i32_min() {
    let min_i32: i32 = -2147483648;
    assert_eq!(-min_i32, min_i32);
}

// === Zero Boundary ===

#[test]
fn test_zero_boundary_signed() {
    assert_eq!((-1i8) + 1i8, 0i8);
    assert_eq!(0i8 - 1i8, -1i8);
}

#[test]
fn test_zero_boundary_unsigned() {
    assert_eq!(0u8 - 1u8, 255u8);
    assert_eq!(1u8 - 1u8, 0u8);
}

// === Compound Assignment Overflow ===

#[test]
fn test_compound_add_overflow() {
    let mut x: i8 = 127;
    let min_i8: i8 = -128;
    x += 1i8;
    assert_eq!(x, min_i8);
}

#[test]
fn test_compound_sub_overflow() {
    let mut x: i8 = -128;
    x -= 1i8;
    assert_eq!(x, 127i8);
}

#[test]
fn test_compound_mul_overflow_u8() {
    let mut x: u8 = 16;
    x *= 16u8;
    assert_eq!(x, 0u8);
}

// === Overflow Across Different Ways of Declaring ===

#[test]
fn test_overflow_type_annotation() {
    let a: u8 = 255;
    assert_eq!(a + 1u8, 0u8);
}

#[test]
fn test_overflow_literal_suffix() {
    assert_eq!(255u8 + 1u8, 0u8);
}

// === Max Value Preservation ===

#[test]
fn test_values_within_range_do_not_overflow() {
    assert_eq!(100i8 + 20i8, 120i8);
    assert_eq!(200u8 + 50u8, 250u8);
    assert_eq!((-50i8) + 30i8, -20i8);
}
