//! Integer and Float method implementations — shared by interpreter and VM.

use crate::symbols;
use crate::types::Value;

/// Wrap an f64 method result as a Value. Always returns Value::F64 —
/// never collapses whole-number floats to Value::I64. See the matching
/// helper in `stdlib::math` for the rationale.
fn float_to_value(f: f64) -> Value {
    Value::F64(f)
}

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let to_f64 = |v: &Value| match v {
        Value::I64(n) => *n as f64,
        Value::U8(n) => *n as f64,
        Value::F64(x) => *x,
        _ => 0.0,
    };
    match method {
        symbols::numeric_m::ABS => match &receiver {
            Value::I64(n) => Ok(Value::I64(n.abs())),
            Value::U8(n) => Ok(Value::I64(*n as i64)),
            Value::F64(x) => Ok(float_to_value(x.abs())),
            _ => Ok(Value::I64(0)),
        },
        symbols::numeric_m::SQRT => Ok(float_to_value(to_f64(&receiver).sqrt())),
        symbols::numeric_m::FLOOR => Ok(float_to_value(to_f64(&receiver).floor())),
        symbols::numeric_m::CEIL => Ok(float_to_value(to_f64(&receiver).ceil())),
        symbols::numeric_m::ROUND => Ok(float_to_value(to_f64(&receiver).round())),
        symbols::numeric_m::POW => {
            // Preserve int-receiver type when the exponent is also int and
            // non-negative; otherwise widen to float. Stops `2.pow(10)`
            // from sliding into a float.
            if let (Value::I64(b), Some(Value::I64(e))) = (&receiver, args.first()) {
                if *e >= 0 {
                    let mut acc: i64 = 1;
                    for _ in 0..*e {
                        acc = acc.wrapping_mul(*b);
                    }
                    return Ok(Value::I64(acc));
                }
            }
            Ok(float_to_value(
                to_f64(&receiver).powf(to_f64(args.first().unwrap_or(&Value::Unit))),
            ))
        }
        symbols::numeric_m::SIN => Ok(float_to_value(to_f64(&receiver).sin())),
        symbols::numeric_m::COS => Ok(float_to_value(to_f64(&receiver).cos())),
        symbols::numeric_m::TAN => Ok(float_to_value(to_f64(&receiver).tan())),
        symbols::numeric_m::MIN => match (&receiver, args.first()) {
            (Value::I64(a), Some(Value::I64(b))) => Ok(Value::I64(*a.min(b))),
            _ => Ok(float_to_value(
                to_f64(&receiver).min(to_f64(args.first().unwrap_or(&Value::Unit))),
            )),
        },
        symbols::numeric_m::MAX => match (&receiver, args.first()) {
            (Value::I64(a), Some(Value::I64(b))) => Ok(Value::I64(*a.max(b))),
            _ => Ok(float_to_value(
                to_f64(&receiver).max(to_f64(args.first().unwrap_or(&Value::Unit))),
            )),
        },
        symbols::numeric_m::CLAMP => match (&receiver, args.first(), args.get(1)) {
            (Value::I64(v), Some(Value::I64(lo)), Some(Value::I64(hi))) => {
                Ok(Value::I64((*v).clamp(*lo, *hi)))
            }
            _ => Ok(float_to_value(to_f64(&receiver).clamp(
                to_f64(args.first().unwrap_or(&Value::Unit)),
                to_f64(args.get(1).unwrap_or(&Value::Unit)),
            ))),
        },
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
