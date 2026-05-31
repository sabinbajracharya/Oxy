// === Bitwise operations on `Byte` and `Int` ===
// Oxy has exactly two integer types: `Int` (= i64) and `Byte` (= u8).
// Bitwise ops on `Byte` wrap modulo 256, just like `u8` in Rust.

#[test]
fn test_bitwise_and_byte() {
    val a: Byte = 0xFF;
    val b: Byte = 0x0F;
    assert_eq(a & b, 0x0F);
    assert_eq(0b1100 as Byte & 0b1010 as Byte, 0b1000);
}

#[test]
fn test_bitwise_or_byte() {
    val a: Byte = 0xF0;
    val b: Byte = 0x0F;
    assert_eq(a | b, 0xFF);
}

#[test]
fn test_bitwise_xor_byte() {
    val a: Byte = 0xFF;
    val b: Byte = 0xFF;
    assert_eq(a ^ b, 0);
    assert_eq(0xAA as Byte ^ 0x55 as Byte, 0xFF);
}

#[test]
fn test_bitwise_not_byte() {
    val a: Byte = 0;
    assert_eq(~a, 0xFF);
    val b: Byte = 0x0F;
    assert_eq(~b, 0xF0);
}

#[test]
fn test_left_shift_byte() {
    val a: Byte = 1;
    assert_eq(a << 4, 16);
    assert_eq(a << 7, 128);
}

#[test]
fn test_right_shift_byte() {
    val a: Byte = 128;
    assert_eq(a >> 1, 64);
    assert_eq(a >> 7, 1);
}

#[test]
fn test_bitwise_int() {
    val a: Int = 0xF0F0;
    val b: Int = 0x0F0F;
    assert_eq(a & b, 0);
    assert_eq(a | b, 0xFFFF);
    assert_eq(a ^ b, 0xFFFF);
    assert_eq(1 << 10, 1024);
    assert_eq(1024 >> 5, 32);
}

#[test]
fn test_bit_clear() {
    // Common idiom: clear bit n with `x & !(1 << n)`.
    val x: Byte = 0b0101;
    assert_eq(x & ~(1 as Byte << 2), 0b0001);
}
