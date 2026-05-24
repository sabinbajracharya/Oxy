//! Result method implementations — shared by interpreter and VM.
//!
//! Dispatched from `vm/mod.rs` only when the receiver is a
//! `Result` enum variant, so this function never needs to ask
//! "is this a Result?" — every arm can assume it is.

use crate::symbols;
use crate::types::Value;

fn inner_of(receiver: &Value) -> Value {
    if let Value::EnumVariant { data, .. } = receiver {
        data.first().cloned().unwrap_or(Value::Unit)
    } else {
        Value::Unit
    }
}

pub fn dispatch<F>(
    receiver: Value,
    method: &str,
    args: &[Value],
    mut call_closure: F,
) -> Result<Value, String>
where
    F: FnMut(Value, &[Value]) -> Result<Value, String>,
{
    use symbols::option_result_m as m;
    let is_ok = receiver.is_ok_variant();
    let first_arg = || args.first().cloned().unwrap_or(Value::Unit);

    match method {
        m::IS_OK => Ok(Value::Bool(is_ok)),
        m::IS_ERR => Ok(Value::Bool(!is_ok)),

        m::UNWRAP => {
            if is_ok {
                Ok(inner_of(&receiver))
            } else {
                Err("called `unwrap` on an `Err` value".into())
            }
        }
        m::UNWRAP_ERR => {
            if !is_ok {
                Ok(inner_of(&receiver))
            } else {
                Err("called `unwrap_err` on an `Ok` value".into())
            }
        }
        m::EXPECT => {
            if is_ok {
                Ok(inner_of(&receiver))
            } else {
                Err(args
                    .first()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "expect failed".into()))
            }
        }
        m::UNWRAP_OR => {
            if is_ok {
                Ok(inner_of(&receiver))
            } else {
                Ok(first_arg())
            }
        }
        m::UNWRAP_OR_ELSE => {
            if is_ok {
                Ok(inner_of(&receiver))
            } else {
                // Closure receives the err value (Option's variant takes no args).
                let err_val = inner_of(&receiver);
                call_closure(first_arg(), &[err_val])
            }
        }

        m::OK => {
            if is_ok {
                Ok(Value::some(inner_of(&receiver)))
            } else {
                Ok(Value::none())
            }
        }
        m::ERR => {
            if !is_ok {
                Ok(Value::some(inner_of(&receiver)))
            } else {
                Ok(Value::none())
            }
        }

        m::MAP => {
            if is_ok {
                let result = call_closure(first_arg(), &[inner_of(&receiver)])?;
                Ok(Value::ok(result))
            } else {
                Ok(receiver)
            }
        }
        m::MAP_ERR => {
            if !is_ok {
                let result = call_closure(first_arg(), &[inner_of(&receiver)])?;
                Ok(Value::err(result))
            } else {
                Ok(receiver)
            }
        }
        m::AND_THEN => {
            if is_ok {
                call_closure(first_arg(), &[inner_of(&receiver)])
            } else {
                Ok(receiver)
            }
        }
        m::OR_ELSE => {
            if !is_ok {
                let err_val = inner_of(&receiver);
                call_closure(first_arg(), &[err_val])
            } else {
                Ok(receiver)
            }
        }

        m::CLONE => Ok(receiver.clone()),
        m::TO_STRING => Ok(Value::String(receiver.to_string())),

        _ => Err(format!("no method '{}' on Result", method)),
    }
}

pub fn method_names() -> &'static [&'static str] {
    use symbols::option_result_m as m;
    &[
        m::IS_OK,
        m::IS_ERR,
        m::UNWRAP,
        m::UNWRAP_ERR,
        m::EXPECT,
        m::UNWRAP_OR,
        m::UNWRAP_OR_ELSE,
        m::OK,
        m::ERR,
        m::MAP,
        m::MAP_ERR,
        m::AND_THEN,
        m::OR_ELSE,
        m::CLONE,
        m::TO_STRING,
    ]
}
