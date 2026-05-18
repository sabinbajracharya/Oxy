//! Integer and Float method implementations — shared by interpreter and VM.

use crate::types::Value;

/// Convert a float result to Integer if it's a whole number (matching interpreter behavior).
fn float_to_value(f: f64) -> Value {
    if f.is_finite() && f.fract() == 0.0 && f.abs() < i64::MAX as f64 {
        Value::Integer(f as i64)
    } else {
        Value::Float(f)
    }
}

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let to_f64 = |v: &Value| match v {
        Value::Integer(n) => *n as f64,
        Value::Float(x) => *x,
        _ => 0.0,
    };
    match method {
        "abs" => match &receiver {
            Value::Integer(n) => Ok(Value::Integer(n.abs())),
            Value::Float(x) => Ok(float_to_value(x.abs())),
            _ => Ok(Value::Integer(0)),
        },
        "sqrt" => Ok(float_to_value(to_f64(&receiver).sqrt())),
        "floor" => Ok(float_to_value(to_f64(&receiver).floor())),
        "ceil" => Ok(float_to_value(to_f64(&receiver).ceil())),
        "round" => Ok(float_to_value(to_f64(&receiver).round())),
        "pow" => {
            let base = to_f64(&receiver);
            let exp = to_f64(args.first().unwrap_or(&Value::Unit));
            Ok(float_to_value(base.powf(exp)))
        }
        "sin" => Ok(Value::Float(to_f64(&receiver).sin())),
        "cos" => Ok(Value::Float(to_f64(&receiver).cos())),
        "tan" => Ok(Value::Float(to_f64(&receiver).tan())),
        "min" => {
            let a = to_f64(&receiver);
            let b = to_f64(args.first().unwrap_or(&Value::Unit));
            Ok(float_to_value(a.min(b)))
        }
        "max" => {
            let a = to_f64(&receiver);
            let b = to_f64(args.first().unwrap_or(&Value::Unit));
            Ok(float_to_value(a.max(b)))
        }
        "clamp" => {
            let v = to_f64(&receiver);
            let lo = to_f64(args.first().unwrap_or(&Value::Unit));
            let hi = to_f64(args.get(1).unwrap_or(&Value::Unit));
            Ok(float_to_value(v.clamp(lo, hi)))
        }
        _ => Err(format!(
            "no method '{}' on type {}",
            method,
            receiver.type_name()
        )),
    }
}
