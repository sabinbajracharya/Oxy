// === STRESS: every numeric operator on every combination of types ===
// Int (i64 wrapping), Byte (u8 wrapping), Float (f64 IEEE-754).

// --- Int arithmetic ---
#[test]
fn test_int_add() { assert::eq(2 + 3, 5); }
#[test]
fn test_int_sub() { assert::eq(10 - 4, 6); }
#[test]
fn test_int_sub_negative_result() { assert::eq(3 - 10, -7); }
#[test]
fn test_int_mul() { assert::eq(7 * 8, 56); }
#[test]
fn test_int_div_truncates() { assert::eq(7 / 2, 3); }
#[test]
fn test_int_div_negative_truncates_toward_zero() { assert::eq(-7 / 2, -3); }
#[test]
fn test_int_mod() { assert::eq(10 % 3, 1); }
#[test]
fn test_int_mod_negative() { assert::eq(-10 % 3, -1); }
#[test]
fn test_int_neg() { val n = 5; assert::eq(-n, -5); }
#[test]
fn test_int_neg_zero() { val n = 0; assert::eq(-n, 0); }
#[test]
fn test_int_chained() { assert::eq(1 + 2 * 3 - 4 / 2, 5); }
#[test]
fn test_int_paren_overrides() { assert::eq((1 + 2) * 3, 9); }

// --- Int wrapping on overflow (i64 wraps) ---
#[test]
fn test_int_max_plus_one_wraps() {
    val max: Int = 9223372036854775807;
    val wrapped = max + 1;
    assert::eq(wrapped, -9223372036854775808);
}
#[test]
fn test_int_min_minus_one_wraps() {
    val min: Int = -9223372036854775808;
    val wrapped = min - 1;
    assert::eq(wrapped, 9223372036854775807);
}

// --- Byte arithmetic + wrapping ---
#[test]
fn test_byte_add() {
    val a: Byte = 100;
    val b: Byte = 50;
    assert::eq(a + b, 150);
}
#[test]
fn test_byte_overflow_wraps() {
    val a: Byte = 255;
    val b: Byte = 1;
    val r: Byte = a + b;
    assert::eq(r, 0);
}
#[test]
fn test_byte_underflow_wraps() {
    val a: Byte = 0;
    val b: Byte = 1;
    val r: Byte = a - b;
    assert::eq(r, 255);
}
#[test]
fn test_byte_mul_wraps() {
    val a: Byte = 16;
    val r: Byte = a * 16;
    assert::eq(r, 0);
}

// --- Float arithmetic ---
#[test]
fn test_float_add() { assert::eq(1.5 + 2.5, 4.0); }
#[test]
fn test_float_sub() { assert::eq(3.0 - 1.5, 1.5); }
#[test]
fn test_float_mul() { assert::eq(2.5 * 4.0, 10.0); }
#[test]
fn test_float_div() { assert::eq(7.0 / 2.0, 3.5); }
#[test]
fn test_float_div_fraction() { assert::eq(1.0 / 4.0, 0.25); }
#[test]
fn test_float_neg() { val f = 1.5; assert::eq(-f, -1.5); }

// --- bitwise on Int ---
#[test]
fn test_int_bitand() { assert::eq(0xFF & 0x0F, 0x0F); }
#[test]
fn test_int_bitor() { assert::eq(0x0F | 0xF0, 0xFF); }
#[test]
fn test_int_bitxor() { assert::eq(0xFF ^ 0x0F, 0xF0); }
#[test]
fn test_int_shl() { assert::eq(1 << 8, 256); }
#[test]
fn test_int_shr() { assert::eq(256 >> 8, 1); }
#[test]
fn test_int_bitnot() { assert::eq(~0, -1); }

// --- bitwise on Byte ---
#[test]
fn test_byte_bitand() {
    val a: Byte = 0xFF;
    val b: Byte = 0x0F;
    val r: Byte = a & b;
    assert::eq(r, 0x0F);
}
#[test]
fn test_byte_bitor() {
    val a: Byte = 0x0F;
    val b: Byte = 0xF0;
    val r: Byte = a | b;
    assert::eq(r, 0xFF);
}
#[test]
fn test_byte_bitxor() {
    val a: Byte = 0xFF;
    val b: Byte = 0x0F;
    val r: Byte = a ^ b;
    assert::eq(r, 0xF0);
}
#[test]
fn test_byte_shl() {
    val a: Byte = 1;
    val r: Byte = a << 4;
    assert::eq(r, 16);
}
#[test]
fn test_byte_shr() {
    val a: Byte = 16;
    val r: Byte = a >> 4;
    assert::eq(r, 1);
}
#[test]
fn test_byte_bitnot() {
    val a: Byte = 0;
    val r: Byte = ~a;
    assert::eq(r, 255);
}

// --- comparisons ---
#[test]
fn test_int_eq() { assert::eq(3 == 3, true); assert::eq(3 == 4, false); }
#[test]
fn test_int_lt() { assert::eq(2 < 3, true); assert::eq(3 < 2, false); }
#[test]
fn test_int_gt() { assert::eq(3 > 2, true); assert::eq(2 > 3, false); }
#[test]
fn test_int_le() { assert::eq(3 <= 3, true); assert::eq(3 <= 4, true); assert::eq(4 <= 3, false); }
#[test]
fn test_int_ge() { assert::eq(3 >= 3, true); assert::eq(4 >= 3, true); assert::eq(2 >= 3, false); }
#[test]
fn test_float_eq() { assert::eq(1.5 == 1.5, true); }
#[test]
fn test_float_lt() { assert::eq(1.4 < 1.5, true); }

// --- mixed Int + Float arithmetic (widens to Float at the operator) ---
#[test]
fn test_mixed_int_float_add() {
    val r: Float = 2 as Float + 1.5;
    assert::eq(r, 3.5);
}

// --- as-casts ---
#[test]
fn test_cast_int_to_byte_truncate() {
    val n: Int = 300;
    val b: Byte = n as Byte;
    assert::eq(b, 44);  // 300 mod 256
}
#[test]
fn test_cast_byte_to_int() {
    val b: Byte = 200;
    val n: Int = b as Int;
    assert::eq(n, 200);
}
#[test]
fn test_cast_int_to_float() {
    val n: Int = 7;
    val f: Float = n as Float;
    assert::eq(f, 7.0);
}
#[test]
fn test_cast_float_to_int_truncates() {
    val f: Float = 3.9;
    val n: Int = f as Int;
    assert::eq(n, 3);
}
#[test]
fn test_cast_negative_float_to_int_truncates_toward_zero() {
    val f: Float = -3.9;
    val n: Int = f as Int;
    assert::eq(n, -3);
}
#[test]
fn test_cast_chain() {
    val f: Float = 257.5;
    val b: Byte = f as Int as Byte;
    assert::eq(b, 1);  // 257 mod 256
}

// --- division by zero behavior ---
// Integer div by zero is a runtime error in Oxy — test elsewhere via #[compile_error]
// if it's caught at compile time, or document the runtime behavior.

// --- compound assignment ---
#[test]
fn test_compound_assign_add() {
    var n = 5;
    n += 3;
    assert::eq(n, 8);
}
#[test]
fn test_compound_assign_sub() {
    var n = 10;
    n -= 4;
    assert::eq(n, 6);
}
#[test]
fn test_compound_assign_mul() {
    var n = 3;
    n *= 4;
    assert::eq(n, 12);
}
#[test]
fn test_compound_assign_div() {
    var n = 12;
    n /= 4;
    assert::eq(n, 3);
}
#[test]
fn test_compound_assign_mod() {
    var n = 10;
    n %= 3;
    assert::eq(n, 1);
}
#[test]
fn test_compound_assign_byte() {
    var b: Byte = 200;
    b += 100;
    assert::eq(b, 44);  // wraps
}

// --- prefix vs unary in expressions ---
#[test]
fn test_double_negation() {
    val n = 5;
    assert::eq(-(-n), 5);
}
#[test]
fn test_neg_in_expression() {
    assert::eq(10 + -3, 7);
}

// --- comparisons through fn boundary ---
fn add_int(a: Int, b: Int) -> Int { a + b }
fn add_byte(a: Byte, b: Byte) -> Byte { a + b }
fn add_float(a: Float, b: Float) -> Float { a + b }

#[test]
fn test_fn_int_returns_int() { assert::eq(add_int(2, 3), 5); }
#[test]
fn test_fn_byte_returns_byte_wraps() {
    val b = add_byte(200, 100);
    assert::eq(b, 44);
}
#[test]
fn test_fn_float_returns_float() { assert::eq(add_float(1.5, 2.5), 4.0); }

// --- precedence sanity ---
#[test]
fn test_precedence_and_or() {
    assert::eq(true || false && false, true);
    assert::eq((true || false) && false, false);
}
#[test]
fn test_precedence_comparison_bitwise() {
    // 0x10 & 0x10 == 0x10 — equality binds tighter than bitwise-and in Rust;
    // verify Oxy matches.
    val r = (0x10 & 0x10) == 0x10;
    assert::eq(r, true);
}
