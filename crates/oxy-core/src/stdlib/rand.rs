//! Random number generation standard library module.
//!
//! Uses a global xorshift64 PRNG seeded from system time on first use.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::errors::check_arg_count;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

/// Global PRNG state for rand:: module (xorshift64).
static PRNG_STATE: AtomicU64 = AtomicU64::new(0);

/// Best-effort time-based seed. Returns 0 on platforms where wall-clock time
/// isn't available (notably `wasm32-unknown-unknown`, where `SystemTime::now()`
/// panics with "unreachable executed"). The caller handles the zero case.
fn seed_from_time() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }
    #[cfg(target_arch = "wasm32")]
    {
        0
    }
}

/// Get the next pseudo-random u64 from the global PRNG.
fn simple_random_u64() -> u64 {
    let mut state = PRNG_STATE.load(Ordering::Relaxed);
    if state == 0 {
        // Seed from current time when available; fall back to a fixed non-zero
        // constant on platforms without a clock (wasm). The constant means the
        // wasm playground gets a deterministic sequence per session, but the
        // PRNG still advances within a session.
        let raw = seed_from_time();
        state = if raw == 0 { 0x9E37_79B9_7F4A_7C15 } else { raw | 1 };
        PRNG_STATE.store(state, Ordering::Relaxed);
    }
    // WHY: xorshift64 chosen for its simplicity and speed in an interpreted language context;
    // cryptographic strength is unnecessary here. The magic constants (13, 7, 17) are one of
    // the specific shift-triplets proven by Marsaglia to produce a full 2^64-1 period,
    // ensuring the PRNG visits every non-zero state before repeating.
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    PRNG_STATE.store(state, Ordering::Relaxed);
    state
}

/// Get a pseudo-random f64 in [0.0, 1.0).
fn simple_random_f64() -> f64 {
    (simple_random_u64() >> 11) as f64 / ((1u64 << 53) as f64)
}

/// Dispatch rand:: function calls.
pub fn call(func_name: &str, args: &[Value], span: &Span) -> Result<Value, FerriError> {
    match func_name {
        "random" => {
            check_arg_count("rand::random", 0, args, span)?;
            Ok(Value::F64(simple_random_f64()))
        }
        "range" => {
            check_arg_count("rand::range", 2, args, span)?;
            match (&args[0], &args[1]) {
                (Value::I64(min), Value::I64(max)) => {
                    if min >= max {
                        return Err(FerriError::Runtime {
                            message: "rand::range() requires min < max".into(),
                            line: span.line,
                            column: span.column,
                        });
                    }
                    let range = (max - min) as u64;
                    let raw = simple_random_u64();
                    let val = min + (raw % range) as i64;
                    Ok(Value::I64(val))
                }
                _ => Err(FerriError::Runtime {
                    message: "rand::range() requires integer arguments".into(),
                    line: span.line,
                    column: span.column,
                }),
            }
        }
        // Inclusive integer in [lo, hi]. Like Rust's rand::Rng::gen_range
        // for an inclusive range — convenient for dice rolls etc. where
        // both endpoints should be reachable.
        "rand_int" => {
            check_arg_count("rand::rand_int", 2, args, span)?;
            match (&args[0], &args[1]) {
                (Value::I64(lo), Value::I64(hi)) => {
                    if lo > hi {
                        return Err(FerriError::Runtime {
                            message: "rand::rand_int(lo, hi) requires lo <= hi".into(),
                            line: span.line,
                            column: span.column,
                        });
                    }
                    let span_size = (hi - lo) as u64 + 1;
                    let raw = simple_random_u64();
                    Ok(Value::I64(lo + (raw % span_size) as i64))
                }
                _ => Err(FerriError::Runtime {
                    message: "rand::rand_int() requires integer arguments".into(),
                    line: span.line,
                    column: span.column,
                }),
            }
        }
        "bool" => {
            check_arg_count("rand::bool", 0, args, span)?;
            Ok(Value::Bool(simple_random_u64() % 2 == 0))
        }
        _ => Err(FerriError::Runtime {
            message: format!("unknown rand function `rand::{func_name}`"),
            line: span.line,
            column: span.column,
        }),
    }
}
