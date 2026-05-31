//! Pure arithmetic and cast helpers called by the JIT FFI bridge.
//!
//! These operate on `Value` types directly — the FFI functions in `ffi.rs`
//! call them after popping operands from the JitContext operand stack.

use crate::types::{FloatWidth, IntegerWidth, Value};

// --- Width-aware integer helpers ---

/// Promote two integers to a common type. Same-type (byte+byte) stays as
/// byte; any int+byte mix widens to int, since int is the wider type and
/// arithmetic between mixed widths conceptually happens at int.
pub(crate) fn promote_ints(a: Value, b: Value) -> (Value, Value) {
    if std::mem::discriminant(&a) == std::mem::discriminant(&b) {
        (a, b)
    } else {
        (Value::I64(a.as_i64()), Value::I64(b.as_i64()))
    }
}

/// Wrap an i64 result back to the target integer variant (byte or int).
pub(crate) fn wrap_to(v: i64, target: &Value) -> Value {
    match target {
        Value::U8(_) => Value::U8(v as u8),
        _ => Value::I64(v),
    }
}

// --- Arithmetic ---

pub(crate) fn vm_add(a: Value, b: Value) -> Result<Value, String> {
    if let (Value::String(sa), Value::String(sb)) = (&a, &b) {
        return Ok(Value::String(format!("{sa}{sb}")));
    }
    if let Value::String(s) = &a {
        return Ok(Value::String(format!("{s}{b}")));
    }
    if let Value::String(s) = &b {
        return Ok(Value::String(format!("{a}{s}")));
    }
    if a.is_float() || b.is_float() {
        return Ok(Value::F64(a.to_f64() + b.to_f64()));
    }
    if a.is_integer() && b.is_integer() {
        let (a, b) = promote_ints(a, b);
        return Ok(wrap_to(a.as_i64().wrapping_add(b.as_i64()), &a));
    }
    Err(format!(
        "cannot add {} and {}",
        a.type_name(),
        b.type_name()
    ))
}

pub(crate) fn vm_sub(a: Value, b: Value) -> Result<Value, String> {
    if a.is_float() || b.is_float() {
        return Ok(Value::F64(a.to_f64() - b.to_f64()));
    }
    if a.is_integer() && b.is_integer() {
        let (a, b) = promote_ints(a, b);
        return Ok(wrap_to(a.as_i64().wrapping_sub(b.as_i64()), &a));
    }
    Err(format!(
        "cannot subtract {} and {}",
        a.type_name(),
        b.type_name()
    ))
}

pub(crate) fn vm_mul(a: Value, b: Value) -> Result<Value, String> {
    if a.is_float() || b.is_float() {
        return Ok(Value::F64(a.to_f64() * b.to_f64()));
    }
    if a.is_integer() && b.is_integer() {
        let (a, b) = promote_ints(a, b);
        return Ok(wrap_to(a.as_i64().wrapping_mul(b.as_i64()), &a));
    }
    Err(format!(
        "cannot multiply {} and {}",
        a.type_name(),
        b.type_name()
    ))
}

pub(crate) fn vm_div(a: Value, b: Value) -> Result<Value, String> {
    if a.is_float() || b.is_float() {
        if b.to_f64() == 0.0 {
            return Err("division by zero".into());
        }
        return Ok(Value::F64(a.to_f64() / b.to_f64()));
    }
    if a.is_integer() && b.is_integer() {
        let (a, b) = promote_ints(a, b);
        let divisor = b.as_i64();
        if divisor == 0 {
            return Err("division by zero".into());
        }
        return Ok(wrap_to(a.as_i64() / divisor, &a));
    }
    Err(format!(
        "cannot divide {} and {}",
        a.type_name(),
        b.type_name()
    ))
}

pub(crate) fn vm_rem(a: Value, b: Value) -> Result<Value, String> {
    if a.is_float() || b.is_float() {
        if b.to_f64() == 0.0 {
            return Err("modulo by zero".into());
        }
        return Ok(Value::F64(a.to_f64() % b.to_f64()));
    }
    if a.is_integer() && b.is_integer() {
        let (a, b) = promote_ints(a, b);
        let divisor = b.as_i64();
        if divisor == 0 {
            return Err("modulo by zero".into());
        }
        return Ok(wrap_to(a.as_i64() % divisor, &a));
    }
    Err(format!(
        "cannot compute modulo of {} and {}",
        a.type_name(),
        b.type_name()
    ))
}

pub(crate) fn vm_neg(v: Value) -> Value {
    match v {
        Value::I64(n) => Value::I64(n.wrapping_neg()),
        Value::U8(n) => Value::U8(n.wrapping_neg()),
        Value::F64(n) => Value::F64(-n),
        v => v,
    }
}

pub(crate) fn vm_bitnot(v: Value) -> Value {
    match v {
        Value::I64(n) => Value::I64(!n),
        Value::U8(n) => Value::U8(!n),
        v => v,
    }
}

pub(crate) fn vm_bitand(a: Value, b: Value) -> Result<Value, String> {
    if a.is_integer() && b.is_integer() {
        let (a, b) = promote_ints(a, b);
        Ok(wrap_to(a.as_i64() & b.as_i64(), &a))
    } else {
        Err("bitwise AND requires integers".to_string())
    }
}

pub(crate) fn vm_bitor(a: Value, b: Value) -> Result<Value, String> {
    if a.is_integer() && b.is_integer() {
        let (a, b) = promote_ints(a, b);
        Ok(wrap_to(a.as_i64() | b.as_i64(), &a))
    } else {
        Err("bitwise OR requires integers".to_string())
    }
}

pub(crate) fn vm_bitxor(a: Value, b: Value) -> Result<Value, String> {
    if a.is_integer() && b.is_integer() {
        let (a, b) = promote_ints(a, b);
        Ok(wrap_to(a.as_i64() ^ b.as_i64(), &a))
    } else {
        Err("bitwise XOR requires integers".to_string())
    }
}

pub(crate) fn vm_shl(a: Value, b: Value) -> Result<Value, String> {
    if a.is_integer() && b.is_integer() {
        let shift = b.as_u64() as u32;
        Ok(wrap_to(a.as_i64().wrapping_shl(shift), &a))
    } else {
        Err("shift left requires integers".to_string())
    }
}

pub(crate) fn vm_shr(a: Value, b: Value) -> Result<Value, String> {
    if a.is_integer() && b.is_integer() {
        let shift = b.as_u64() as u32;
        Ok(wrap_to(a.as_i64().wrapping_shr(shift), &a))
    } else {
        Err("shift right requires integers".to_string())
    }
}

// --- Cast helpers ---

/// Extract an i64 from any Value type (for cast/conversion purposes).
pub(crate) fn value_to_i64(val: &Value) -> i64 {
    match val {
        Value::I64(n) => *n,
        Value::U8(n) => *n as i64,
        Value::F64(n) => *n as i64,
        Value::Char(c) => *c as u32 as i64,
        _ => 0,
    }
}

/// Cast a Value to a specific integer width with wrapping.
pub(crate) fn cast_to_int(val: &Value, width: IntegerWidth) -> Value {
    let bits = value_to_i64(val);
    match width {
        IntegerWidth::I64 => Value::I64(bits),
        IntegerWidth::U8 => Value::U8(bits as u8),
    }
}

/// Cast a Value to a specific float width.
pub(crate) fn cast_to_float(val: &Value, _width: FloatWidth) -> Value {
    let f = match val {
        Value::F64(n) => *n,
        Value::Char(c) => *c as u32 as f64,
        _ => value_to_i64(val) as f64,
    };
    Value::F64(f)
}
