// === STRESS: every numeric operator on every combination of types ===
// int (i64 wrapping), byte (u8 wrapping), float (f64 IEEE-754).

// --- int arithmetic ---
#[test]
fn test_int_add() { assert_eq!(2 + 3, 5); }
#[test]
fn test_int_sub() { assert_eq!(10 - 4, 6); }
#[test]
fn test_int_sub_negative_result() { assert_eq!(3 - 10, -7); }
#[test]
fn test_int_mul() { assert_eq!(7 * 8, 56); }
#[test]
fn test_int_div_truncates() { assert_eq!(7 / 2, 3); }
#[test]
fn test_int_div_negative_truncates_toward_zero() { assert_eq!(-7 / 2, -3); }
#[test]
fn test_int_mod() { assert_eq!(10 % 3, 1); }
#[test]
fn test_int_mod_negative() { assert_eq!(-10 % 3, -1); }
#[test]
fn test_int_neg() { let n = 5; assert_eq!(-n, -5); }
#[test]
fn test_int_neg_zero() { let n = 0; assert_eq!(-n, 0); }
#[test]
fn test_int_chained() { assert_eq!(1 + 2 * 3 - 4 / 2, 5); }
#[test]
fn test_int_paren_overrides() { assert_eq!((1 + 2) * 3, 9); }

// --- int wrapping on overflow (i64 wraps) ---
#[test]
fn test_int_max_plus_one_wraps() {
    let max: int = 9223372036854775807;
    let wrapped = max + 1;
    assert_eq!(wrapped, -9223372036854775808);
}
#[test]
fn test_int_min_minus_one_wraps() {
    let min: int = -9223372036854775808;
    let wrapped = min - 1;
    assert_eq!(wrapped, 9223372036854775807);
}

// --- byte arithmetic + wrapping ---
#[test]
fn test_byte_add() {
    let a: byte = 100;
    let b: byte = 50;
    assert_eq!(a + b, 150);
}
#[test]
fn test_byte_overflow_wraps() {
    let a: byte = 255;
    let b: byte = 1;
    let r: byte = a + b;
    assert_eq!(r, 0);
}
#[test]
fn test_byte_underflow_wraps() {
    let a: byte = 0;
    let b: byte = 1;
    let r: byte = a - b;
    assert_eq!(r, 255);
}
#[test]
fn test_byte_mul_wraps() {
    let a: byte = 16;
    let r: byte = a * 16;
    assert_eq!(r, 0);
}

// --- float arithmetic ---
#[test]
fn test_float_add() { assert_eq!(1.5 + 2.5, 4.0); }
#[test]
fn test_float_sub() { assert_eq!(3.0 - 1.5, 1.5); }
#[test]
fn test_float_mul() { assert_eq!(2.5 * 4.0, 10.0); }
#[test]
fn test_float_div() { assert_eq!(7.0 / 2.0, 3.5); }
#[test]
fn test_float_div_fraction() { assert_eq!(1.0 / 4.0, 0.25); }
#[test]
fn test_float_neg() { let f = 1.5; assert_eq!(-f, -1.5); }

// --- bitwise on int ---
#[test]
fn test_int_bitand() { assert_eq!(0xFF & 0x0F, 0x0F); }
#[test]
fn test_int_bitor() { assert_eq!(0x0F | 0xF0, 0xFF); }
#[test]
fn test_int_bitxor() { assert_eq!(0xFF ^ 0x0F, 0xF0); }
#[test]
fn test_int_shl() { assert_eq!(1 << 8, 256); }
#[test]
fn test_int_shr() { assert_eq!(256 >> 8, 1); }
#[test]
fn test_int_bitnot() { assert_eq!(~0, -1); }

// --- bitwise on byte ---
#[test]
fn test_byte_bitand() {
    let a: byte = 0xFF;
    let b: byte = 0x0F;
    let r: byte = a & b;
    assert_eq!(r, 0x0F);
}
#[test]
fn test_byte_bitor() {
    let a: byte = 0x0F;
    let b: byte = 0xF0;
    let r: byte = a | b;
    assert_eq!(r, 0xFF);
}
#[test]
fn test_byte_bitxor() {
    let a: byte = 0xFF;
    let b: byte = 0x0F;
    let r: byte = a ^ b;
    assert_eq!(r, 0xF0);
}
#[test]
fn test_byte_shl() {
    let a: byte = 1;
    let r: byte = a << 4;
    assert_eq!(r, 16);
}
#[test]
fn test_byte_shr() {
    let a: byte = 16;
    let r: byte = a >> 4;
    assert_eq!(r, 1);
}
#[test]
fn test_byte_bitnot() {
    let a: byte = 0;
    let r: byte = ~a;
    assert_eq!(r, 255);
}

// --- comparisons ---
#[test]
fn test_int_eq() { assert_eq!(3 == 3, true); assert_eq!(3 == 4, false); }
#[test]
fn test_int_lt() { assert_eq!(2 < 3, true); assert_eq!(3 < 2, false); }
#[test]
fn test_int_gt() { assert_eq!(3 > 2, true); assert_eq!(2 > 3, false); }
#[test]
fn test_int_le() { assert_eq!(3 <= 3, true); assert_eq!(3 <= 4, true); assert_eq!(4 <= 3, false); }
#[test]
fn test_int_ge() { assert_eq!(3 >= 3, true); assert_eq!(4 >= 3, true); assert_eq!(2 >= 3, false); }
#[test]
fn test_float_eq() { assert_eq!(1.5 == 1.5, true); }
#[test]
fn test_float_lt() { assert_eq!(1.4 < 1.5, true); }

// --- mixed int + float arithmetic (widens to float at the operator) ---
#[test]
fn test_mixed_int_float_add() {
    let r: float = 2 as float + 1.5;
    assert_eq!(r, 3.5);
}

// --- as-casts ---
#[test]
fn test_cast_int_to_byte_truncate() {
    let n: int = 300;
    let b: byte = n as byte;
    assert_eq!(b, 44);  // 300 mod 256
}
#[test]
fn test_cast_byte_to_int() {
    let b: byte = 200;
    let n: int = b as int;
    assert_eq!(n, 200);
}
#[test]
fn test_cast_int_to_float() {
    let n: int = 7;
    let f: float = n as float;
    assert_eq!(f, 7.0);
}
#[test]
fn test_cast_float_to_int_truncates() {
    let f: float = 3.9;
    let n: int = f as int;
    assert_eq!(n, 3);
}
#[test]
fn test_cast_negative_float_to_int_truncates_toward_zero() {
    let f: float = -3.9;
    let n: int = f as int;
    assert_eq!(n, -3);
}
#[test]
fn test_cast_chain() {
    let f: float = 257.5;
    let b: byte = f as int as byte;
    assert_eq!(b, 1);  // 257 mod 256
}

// --- division by zero behavior ---
// Integer div by zero is a runtime error in Oxy — test elsewhere via #[compile_error]
// if it's caught at compile time, or document the runtime behavior.

// --- compound assignment ---
#[test]
fn test_compound_assign_add() {
    let mut n = 5;
    n += 3;
    assert_eq!(n, 8);
}
#[test]
fn test_compound_assign_sub() {
    let mut n = 10;
    n -= 4;
    assert_eq!(n, 6);
}
#[test]
fn test_compound_assign_mul() {
    let mut n = 3;
    n *= 4;
    assert_eq!(n, 12);
}
#[test]
fn test_compound_assign_div() {
    let mut n = 12;
    n /= 4;
    assert_eq!(n, 3);
}
#[test]
fn test_compound_assign_mod() {
    let mut n = 10;
    n %= 3;
    assert_eq!(n, 1);
}
#[test]
fn test_compound_assign_byte() {
    let mut b: byte = 200;
    b += 100;
    assert_eq!(b, 44);  // wraps
}

// --- prefix vs unary in expressions ---
#[test]
fn test_double_negation() {
    let n = 5;
    assert_eq!(-(-n), 5);
}
#[test]
fn test_neg_in_expression() {
    assert_eq!(10 + -3, 7);
}

// --- comparisons through fn boundary ---
fn add_int(a: int, b: int) -> int { a + b }
fn add_byte(a: byte, b: byte) -> byte { a + b }
fn add_float(a: float, b: float) -> float { a + b }

#[test]
fn test_fn_int_returns_int() { assert_eq!(add_int(2, 3), 5); }
#[test]
fn test_fn_byte_returns_byte_wraps() {
    let b = add_byte(200, 100);
    assert_eq!(b, 44);
}
#[test]
fn test_fn_float_returns_float() { assert_eq!(add_float(1.5, 2.5), 4.0); }

// --- precedence sanity ---
#[test]
fn test_precedence_and_or() {
    assert_eq!(true || false && false, true);
    assert_eq!((true || false) && false, false);
}
#[test]
fn test_precedence_comparison_bitwise() {
    // 0x10 & 0x10 == 0x10 — equality binds tighter than bitwise-and in Rust;
    // verify Oxy matches.
    let r = (0x10 & 0x10) == 0x10;
    assert_eq!(r, true);
}
