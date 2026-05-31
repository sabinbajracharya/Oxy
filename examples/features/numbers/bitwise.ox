// === Bitwise operations on `byte` and `int` ===
// Oxy has exactly two integer types: `int` (= i64) and `byte` (= u8).
// Bitwise ops on `byte` wrap modulo 256, just like `u8` in Rust.

#[test]
fn test_bitwise_and_byte() {
    let a: byte = 0xFF;
    let b: byte = 0x0F;
    assert_eq(a & b, 0x0F);
    assert_eq(0b1100 as byte & 0b1010 as byte, 0b1000);
}

#[test]
fn test_bitwise_or_byte() {
    let a: byte = 0xF0;
    let b: byte = 0x0F;
    assert_eq(a | b, 0xFF);
}

#[test]
fn test_bitwise_xor_byte() {
    let a: byte = 0xFF;
    let b: byte = 0xFF;
    assert_eq(a ^ b, 0);
    assert_eq(0xAA as byte ^ 0x55 as byte, 0xFF);
}

#[test]
fn test_bitwise_not_byte() {
    let a: byte = 0;
    assert_eq(~a, 0xFF);
    let b: byte = 0x0F;
    assert_eq(~b, 0xF0);
}

#[test]
fn test_left_shift_byte() {
    let a: byte = 1;
    assert_eq(a << 4, 16);
    assert_eq(a << 7, 128);
}

#[test]
fn test_right_shift_byte() {
    let a: byte = 128;
    assert_eq(a >> 1, 64);
    assert_eq(a >> 7, 1);
}

#[test]
fn test_bitwise_int() {
    let a: int = 0xF0F0;
    let b: int = 0x0F0F;
    assert_eq(a & b, 0);
    assert_eq(a | b, 0xFFFF);
    assert_eq(a ^ b, 0xFFFF);
    assert_eq(1 << 10, 1024);
    assert_eq(1024 >> 5, 32);
}

#[test]
fn test_bit_clear() {
    // Common idiom: clear bit n with `x & !(1 << n)`.
    let x: byte = 0b0101;
    assert_eq(x & ~(1 as byte << 2), 0b0001);
}
