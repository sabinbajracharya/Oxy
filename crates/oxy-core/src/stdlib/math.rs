//! Math standard library module.
//!
//! Provides trigonometric, rounding, logarithmic, and other numeric functions
//! along with constants like `PI` and `E`.

use crate::errors::check_arg_count;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

/// Convert a Value to f64.
pub fn value_to_f64(val: &Value, span: &Span) -> Result<f64, FerriError> {
    match val {
        Value::I64(n) => Ok(*n as f64),
        Value::F64(f) => Ok(*f),
        _ => Err(FerriError::Runtime {
            message: format!("expected numeric argument, got {}", val.type_name()),
            line: span.line,
            column: span.column,
        }),
    }
}

/// Wrap an f64 math result as a Value. Always returns `Value::F64` — never
/// auto-coerces whole numbers to `Value::I64`. Previously this collapsed
/// e.g. `math::sin(0.0)` to int `0`, which surprised users whose code
/// expected a uniform float return type.
///
/// Shared by `vm::builtins::numeric` so float-returning method calls
/// (`.sqrt()`, `.sin()`, etc.) get the same wrapping as the `math::*`
/// free functions.
pub fn float_to_value(f: f64) -> Value {
    Value::F64(f)
}

/// Dispatch math:: function calls.
pub fn call(func_name: &str, args: &[Value], span: &Span) -> Result<Value, FerriError> {
    match func_name {
        "sqrt" => unary_op("sqrt", args, f64::sqrt, span),
        "abs" => {
            check_arg_count("math::abs", 1, args, span)?;
            match &args[0] {
                Value::I64(n) => Ok(Value::I64(n.abs())),
                Value::F64(f) => Ok(float_to_value(f.abs())),
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
            check_arg_count("math::min", 2, args, span)?;
            match (&args[0], &args[1]) {
                (Value::I64(a), Value::I64(b)) => Ok(Value::I64(*a.min(b))),
                _ => {
                    let a = value_to_f64(&args[0], span)?;
                    let b = value_to_f64(&args[1], span)?;
                    Ok(float_to_value(a.min(b)))
                }
            }
        }
        "max" => {
            check_arg_count("math::max", 2, args, span)?;
            match (&args[0], &args[1]) {
                (Value::I64(a), Value::I64(b)) => Ok(Value::I64(*a.max(b))),
                _ => {
                    let a = value_to_f64(&args[0], span)?;
                    let b = value_to_f64(&args[1], span)?;
                    Ok(float_to_value(a.max(b)))
                }
            }
        }
        "gcd" => {
            check_arg_count("math::gcd", 2, args, span)?;
            let a = math_int(&args[0], "gcd", span)?;
            let b = math_int(&args[1], "gcd", span)?;
            Ok(Value::I64(gcd(a, b)))
        }
        "lcm" => {
            check_arg_count("math::lcm", 2, args, span)?;
            let a = math_int(&args[0], "lcm", span)?;
            let b = math_int(&args[1], "lcm", span)?;
            Ok(Value::I64(lcm(a, b)))
        }
        "log" => unary_op("log", args, f64::ln, span),
        "log2" => unary_op("log2", args, f64::log2, span),
        "log10" => unary_op("log10", args, f64::log10, span),
        "clamp" => {
            check_arg_count("math::clamp", 3, args, span)?;
            match (&args[0], &args[1], &args[2]) {
                (Value::I64(v), Value::I64(lo), Value::I64(hi)) => {
                    Ok(Value::I64((*v).clamp(*lo, *hi)))
                }
                _ => {
                    let v = value_to_f64(&args[0], span)?;
                    let lo = value_to_f64(&args[1], span)?;
                    let hi = value_to_f64(&args[2], span)?;
                    Ok(float_to_value(v.clamp(lo, hi)))
                }
            }
        }
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
        "PI" => Some(Value::F64(std::f64::consts::PI)),
        "E" => Some(Value::F64(std::f64::consts::E)),
        _ => None,
    }
}

fn unary_op(
    name: &str,
    args: &[Value],
    op: fn(f64) -> f64,
    span: &Span,
) -> Result<Value, FerriError> {
    check_arg_count(&format!("math::{name}"), 1, args, span)?;
    let x = value_to_f64(&args[0], span)?;
    Ok(float_to_value(op(x)))
}

fn binary_op(
    name: &str,
    args: &[Value],
    op: fn(f64, f64) -> f64,
    span: &Span,
) -> Result<Value, FerriError> {
    check_arg_count(&format!("math::{name}"), 2, args, span)?;
    let a = value_to_f64(&args[0], span)?;
    let b = value_to_f64(&args[1], span)?;
    Ok(float_to_value(op(a, b)))
}

fn math_int(val: &Value, name: &str, span: &Span) -> Result<i64, FerriError> {
    match val {
        Value::I64(n) => Ok(*n),
        _ => Err(FerriError::Runtime {
            message: format!(
                "math::{name} requires integer arguments, got {}",
                val.type_name()
            ),
            line: span.line,
            column: span.column,
        }),
    }
}

fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.abs()
}

fn lcm(a: i64, b: i64) -> i64 {
    if a == 0 || b == 0 {
        0
    } else {
        (a / gcd(a, b)).abs() * b.abs()
    }
}
