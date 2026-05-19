// === Feature: Numbers — Bitwise Operations ===
// Bitwise operators (&, |, ^, <<, >>, ~) on integer types.
// Works on all integer widths including signed and unsigned.
// Shift behavior: left shift fills with zeros; right shift on signed types
// is arithmetic (sign-extending), on unsigned is logical (zero-fill).
//
// === Declaration Styles Used ===
//   let x: u8 = 0x0F;       (type annotation with hex)
//   let x = 0xFFu8;          (literal suffix with hex)
//   let x = 0b1010u8;        (literal suffix with binary)

// === Bitwise AND ===

#[test]
fn test_bitwise_and_u8() {
    assert_eq!(0xFFu8 & 0x0Fu8, 0x0Fu8);
    assert_eq!(0b1100u8 & 0b1010u8, 0b1000u8);
    assert_eq!(0xFFu8 & 0x00u8, 0u8);
    assert_eq!(0xFFu8 & 0xFFu8, 0xFFu8);
}

#[test]
fn test_bitwise_and_i32() {
    assert_eq!(0xFFFFi32 & 0x0000i32, 0i32);
    assert_eq!(0xFF00i32 & 0x0FF0i32, 0x0F00i32);
    assert_eq!((-1i32) & 0xFFi32, 0xFFi32);
}

// === Bitwise OR ===

#[test]
fn test_bitwise_or_u8() {
    assert_eq!(0xF0u8 | 0x0Fu8, 0xFFu8);
    assert_eq!(0b1100u8 | 0b0011u8, 0b1111u8);
    assert_eq!(0xAAu8 | 0x00u8, 0xAAu8);
    assert_eq!(0x00u8 | 0x00u8, 0u8);
}

#[test]
fn test_bitwise_or_i16() {
    assert_eq!(0x00FFi16 | 0x7F00i16, 0x7FFFi16);
    assert_eq!(0i16 | 42i16, 42i16);
}

// === Bitwise XOR ===

#[test]
fn test_bitwise_xor_u8() {
    assert_eq!(0xFFu8 ^ 0xFFu8, 0u8);
    assert_eq!(0b1100u8 ^ 0b1010u8, 0b0110u8);
    assert_eq!(0xFFu8 ^ 0x00u8, 0xFFu8);
    assert_eq!(0xAAu8 ^ 0x55u8, 0xFFu8);
}

#[test]
fn test_bitwise_xor_property() {
    // XOR is self-inverse: (a ^ b) ^ b == a
    let a = 42u8;
    let b = 0xAAu8;
    assert_eq!((a ^ b) ^ b, a);
}

// === Bitwise NOT ===

#[test]
fn test_bitwise_not_u8() {
    assert_eq!(~0u8, 0xFFu8);
    assert_eq!(~0xFFu8, 0u8);
    assert_eq!(~0x0Fu8, 0xF0u8);
    assert_eq!(~~42u8, 42u8);
}

#[test]
fn test_bitwise_not_u16() {
    assert_eq!(~0u16, 0xFFFFu16);
    assert_eq!(~0xFFFFu16, 0u16);
}

#[test]
fn test_bitwise_not_i32() {
    assert_eq!(~0i32, (-1i32));
    assert_eq!(~(-1i32), 0i32);
}

// === Left Shift ===

#[test]
fn test_left_shift_u8() {
    assert_eq!(1u8 << 0u8, 1u8);
    assert_eq!(1u8 << 1u8, 2u8);
    assert_eq!(1u8 << 4u8, 16u8);
    assert_eq!(1u8 << 7u8, 128u8);
    assert_eq!(3u8 << 2u8, 12u8);
}

#[test]
fn test_left_shift_i32() {
    assert_eq!(1i32 << 10i32, 1024i32);
    assert_eq!(5i32 << 2i32, 20i32);
    assert_eq!(0i32 << 5i32, 0i32);
}

#[test]
fn test_left_shift_wrap_u8() {
    // Shifting beyond the bit width truncates
    // 1u8 << 8 wraps around in u8
    assert_eq!(1u8 << 8i32, 0u8); // or wraps behavior - depends on impl
}

// === Right Shift ===

#[test]
fn test_right_shift_u8() {
    assert_eq!(128u8 >> 1u8, 64u8);
    assert_eq!(128u8 >> 7u8, 1u8);
    assert_eq!(16u8 >> 2u8, 4u8);
    assert_eq!(0u8 >> 4u8, 0u8);
}

#[test]
fn test_right_shift_i32() {
    assert_eq!(1024i32 >> 10i32, 1i32);
    assert_eq!(16i32 >> 2i32, 4i32);
    assert_eq!(0i32 >> 1i32, 0i32);
}

#[test]
fn test_right_shift_signed_negative() {
    // Arithmetic right shift on signed: preserves sign bit
    assert_eq!((-16i32) >> 1i32, -8i32);
    assert_eq!((-1i32) >> 1i32, -1i32);
}

// === Shift by Large Amounts ===

#[test]
fn test_shift_by_large_amount() {
    // Shift by more than bit width: may wrap or produce zero.
    // Verify the operations don't crash.
    let s1 = 1u8 << 20i32;
    let s2 = 128u8 >> 20i32;
    // Both values exist and the VM didn't panic
    assert!(s1 == s1);
    assert!(s2 == s2);
}

// === Bitwise on Different Widths ===

#[test]
fn test_bitwise_u16() {
    assert_eq!(0xFF00u16 & 0x0FF0u16, 0x0F00u16);
    assert_eq!(0xFF00u16 | 0x00FFu16, 0xFFFFu16);
    assert_eq!(0xFFFFu16 ^ 0xAAAAu16, 0x5555u16);
    assert_eq!(1u16 << 15u16, 32768u16);
}

#[test]
fn test_bitwise_u32() {
    assert_eq!(0xFFFF0000u32 | 0x0000FFFFu32, 0xFFFFFFFFu32);
    assert_eq!(1u32 << 16u32, 65536u32);
    assert_eq!(0xFFFFFFFFu32 >> 16u32, 0xFFFFu32);
}

#[test]
fn test_bitwise_u64() {
    assert_eq!(1u64 << 32u64, 4294967296u64);
    assert_eq!(0xFFFFFFFFFFFFFFFFu64 & 0x00000000FFFFFFFFu64, 0xFFFFFFFFu64);
}

// === Bitwise with Type Annotation ===

#[test]
fn test_annotation_style_bitwise() {
    let a: u8 = 0x0F;
    let b: u8 = 0xF0;
    assert_eq!(a | b, 0xFFu8);
    assert_eq!(a & b, 0u8);
    assert_eq!(a ^ b, 0xFFu8);
}

// === Common Bit Patterns ===

#[test]
fn test_common_bit_patterns() {
    // Check if a number is odd
    assert_eq!(5u8 & 1u8, 1u8);
    assert_eq!(4u8 & 1u8, 0u8);

    // Set a bit
    assert_eq!(0b0001u8 | (1u8 << 2u8), 0b0101u8);

    // Toggle a bit
    assert_eq!(0b0101u8 ^ (1u8 << 0u8), 0b0100u8);
}

#[test]
fn test_clear_bit_with_not() {
    // Clear a bit using AND with NOT
    assert_eq!(0b0101u8 & ~(1u8 << 2u8), 0b0001u8);
    assert_eq!(0xFFu8 & ~0x0Fu8, 0xF0u8);
}
