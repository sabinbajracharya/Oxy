use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

/// Convert a Value to f64.
pub fn value_to_f64(val: &Value, span: &Span) -> Result<f64, FerriError> {
    match val {
        Value::Integer(n) => Ok(*n as f64),
        Value::Float(f) => Ok(*f),
        _ => Err(FerriError::Runtime {
            message: format!("expected numeric argument, got {}", val.type_name()),
            line: span.line,
            column: span.column,
        }),
    }
}

/// Convert f64 result to Value (integer if whole number).
pub fn float_to_value(f: f64) -> Value {
    if f.is_finite() && f.fract() == 0.0 && f.abs() < i64::MAX as f64 {
        Value::Integer(f as i64)
    } else {
        Value::Float(f)
    }
}

/// Dispatch math:: function calls.
pub fn call(func_name: &str, args: &[Value], span: &Span) -> Result<Value, FerriError> {
    match func_name {
        "sqrt" => unary_op("sqrt", args, f64::sqrt, span),
        "abs" => {
            if args.len() != 1 {
                return Err(FerriError::Runtime {
                    message: "math::abs() takes 1 argument".into(),
                    line: span.line,
                    column: span.column,
                });
            }
            match &args[0] {
                Value::Integer(n) => Ok(Value::Integer(n.abs())),
                Value::Float(f) => Ok(float_to_value(f.abs())),
                _ => Err(FerriError::Runtime {
                    message: format!(
                        "math::abs() requires numeric argument, got {}",
                        args[0].type_name()
                    ),
                    line: span.line,
                    column: span.column,
                }),
            }
        }
        "pow" => binary_op("pow", args, f64::powf, span),
        "sin" => unary_op("sin", args, f64::sin, span),
        "cos" => unary_op("cos", args, f64::cos, span),
        "tan" => unary_op("tan", args, f64::tan, span),
        "asin" => unary_op("asin", args, f64::asin, span),
        "acos" => unary_op("acos", args, f64::acos, span),
        "atan" => unary_op("atan", args, f64::atan, span),
        "floor" => unary_op("floor", args, f64::floor, span),
        "ceil" => unary_op("ceil", args, f64::ceil, span),
        "round" => unary_op("round", args, f64::round, span),
        "min" => {
            if args.len() != 2 {
                return Err(FerriError::Runtime {
                    message: "math::min() takes 2 arguments".into(),
                    line: span.line,
                    column: span.column,
                });
            }
            match (&args[0], &args[1]) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(*a.min(b))),
                _ => {
                    let a = value_to_f64(&args[0], span)?;
                    let b = value_to_f64(&args[1], span)?;
                    Ok(float_to_value(a.min(b)))
                }
            }
        }
        "max" => {
            if args.len() != 2 {
                return Err(FerriError::Runtime {
                    message: "math::max() takes 2 arguments".into(),
                    line: span.line,
                    column: span.column,
                });
            }
            match (&args[0], &args[1]) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(*a.max(b))),
                _ => {
                    let a = value_to_f64(&args[0], span)?;
                    let b = value_to_f64(&args[1], span)?;
                    Ok(float_to_value(a.max(b)))
                }
            }
        }
        "log" => unary_op("log", args, f64::ln, span),
        "log2" => unary_op("log2", args, f64::log2, span),
        "log10" => unary_op("log10", args, f64::log10, span),
        _ => Err(FerriError::Runtime {
            message: format!("unknown math function `math::{func_name}`"),
            line: span.line,
            column: span.column,
        }),
    }
}

/// Get math constant by name.
pub fn constant(name: &str) -> Option<Value> {
    match name {
        "PI" => Some(Value::Float(std::f64::consts::PI)),
        "E" => Some(Value::Float(std::f64::consts::E)),
        _ => None,
    }
}

fn unary_op(
    name: &str,
    args: &[Value],
    op: fn(f64) -> f64,
    span: &Span,
) -> Result<Value, FerriError> {
    if args.len() != 1 {
        return Err(FerriError::Runtime {
            message: format!("math::{}() takes 1 argument", name),
            line: span.line,
            column: span.column,
        });
    }
    let x = value_to_f64(&args[0], span)?;
    Ok(float_to_value(op(x)))
}

fn binary_op(
    name: &str,
    args: &[Value],
    op: fn(f64, f64) -> f64,
    span: &Span,
) -> Result<Value, FerriError> {
    if args.len() != 2 {
        return Err(FerriError::Runtime {
            message: format!("math::{}() takes 2 arguments", name),
            line: span.line,
            column: span.column,
        });
    }
    let a = value_to_f64(&args[0], span)?;
    let b = value_to_f64(&args[1], span)?;
    Ok(float_to_value(op(a, b)))
}
