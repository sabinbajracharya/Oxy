//! Integer and Float method implementations — shared by interpreter and VM.

use crate::symbols;
use crate::types::Value;

/// Convert a float result to Integer if it's a whole number (matching interpreter behavior).
fn float_to_value(f: f64) -> Value {
    if f.is_finite() && f.fract() == 0.0 && f.abs() < i64::MAX as f64 {
        Value::I64(f as i64)
    } else {
        Value::F64(f)
    }
}

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let to_f64 = |v: &Value| match v {
        Value::I8(n) => *n as f64,
        Value::I16(n) => *n as f64,
        Value::I32(n) => *n as f64,
        Value::I64(n) => *n as f64,
        Value::U8(n) => *n as f64,
        Value::U16(n) => *n as f64,
        Value::U32(n) => *n as f64,
        Value::U64(n) => *n as f64,
        Value::F32(x) => *x as f64,
        Value::F64(x) => *x,
        _ => 0.0,
    };
    match method {
        symbols::numeric_m::ABS => match &receiver {
            Value::I8(n) => Ok(Value::I64(n.abs() as i64)),
            Value::I16(n) => Ok(Value::I64(n.abs() as i64)),
            Value::I32(n) => Ok(Value::I64(n.abs() as i64)),
            Value::I64(n) => Ok(Value::I64(n.abs())),
            Value::U8(n) => Ok(Value::I64(*n as i64)),
            Value::U16(n) => Ok(Value::I64(*n as i64)),
            Value::U32(n) => Ok(Value::I64(*n as i64)),
            Value::U64(n) => Ok(Value::I64(*n as i64)),
            Value::F32(x) => Ok(float_to_value(x.abs() as f64)),
            Value::F64(x) => Ok(float_to_value(x.abs())),
            _ => Ok(Value::I64(0)),
        },
        symbols::numeric_m::SQRT => Ok(float_to_value(to_f64(&receiver).sqrt())),
        symbols::numeric_m::FLOOR => Ok(float_to_value(to_f64(&receiver).floor())),
        symbols::numeric_m::CEIL => Ok(float_to_value(to_f64(&receiver).ceil())),
        symbols::numeric_m::ROUND => Ok(float_to_value(to_f64(&receiver).round())),
        symbols::numeric_m::POW => {
            let base = to_f64(&receiver);
            let exp = to_f64(args.first().unwrap_or(&Value::Unit));
            Ok(float_to_value(base.powf(exp)))
        }
        symbols::numeric_m::SIN => Ok(float_to_value(to_f64(&receiver).sin())),
        symbols::numeric_m::COS => Ok(float_to_value(to_f64(&receiver).cos())),
        symbols::numeric_m::TAN => Ok(float_to_value(to_f64(&receiver).tan())),
        symbols::numeric_m::MIN => {
            let a = to_f64(&receiver);
            let b = to_f64(args.first().unwrap_or(&Value::Unit));
            Ok(float_to_value(a.min(b)))
        }
        symbols::numeric_m::MAX => {
            let a = to_f64(&receiver);
            let b = to_f64(args.first().unwrap_or(&Value::Unit));
            Ok(float_to_value(a.max(b)))
        }
        symbols::numeric_m::CLAMP => {
            let v = to_f64(&receiver);
            let lo = to_f64(args.first().unwrap_or(&Value::Unit));
            let hi = to_f64(args.get(1).unwrap_or(&Value::Unit));
            Ok(float_to_value(v.clamp(lo, hi)))
        }
        symbols::numeric_m::TO_STRING => Ok(Value::String(receiver.to_string())),
        _ => Err(format!(
            "no method '{}' on type {}",
            method,
            receiver.type_name()
        )),
    }
}

pub fn method_names() -> &'static [&'static str] {
    &[
        symbols::numeric_m::ABS,
        symbols::numeric_m::SQRT,
        symbols::numeric_m::FLOOR,
        symbols::numeric_m::CEIL,
        symbols::numeric_m::ROUND,
        symbols::numeric_m::POW,
        symbols::numeric_m::SIN,
        symbols::numeric_m::COS,
        symbols::numeric_m::TAN,
        symbols::numeric_m::MIN,
        symbols::numeric_m::MAX,
        symbols::numeric_m::CLAMP,
        symbols::numeric_m::TO_STRING,
    ]
}
