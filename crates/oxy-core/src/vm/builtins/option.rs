//! Option method implementations — shared by interpreter and VM.
//!
//! Dispatched from `vm/mod.rs` only when the receiver is an
//! `Option` enum variant, so this function never needs to ask
//! "is this an Option?" — every arm can assume it is.

use crate::symbols;
use crate::types::Value;

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
    let is_some = receiver.is_some_variant();
    let first_arg = || args.first().cloned().unwrap_or(Value::Unit);

    match method {
        m::IS_SOME => Ok(Value::Bool(is_some)),
        m::IS_NONE => Ok(Value::Bool(!is_some)),

        m::UNWRAP => {
            if is_some {
                Ok(receiver.inner_of())
            } else {
                Err("called `unwrap` on a `None` value".into())
            }
        }
        m::EXPECT => {
            if is_some {
                Ok(receiver.inner_of())
            } else {
                Err(args
                    .first()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "expect failed".into()))
            }
        }
        m::UNWRAP_OR => {
            if is_some {
                Ok(receiver.inner_of())
            } else {
                Ok(first_arg())
            }
        }
        m::UNWRAP_OR_ELSE => {
            if is_some {
                Ok(receiver.inner_of())
            } else {
                call_closure(first_arg(), &[])
            }
        }

        m::OR => {
            if is_some {
                Ok(receiver)
            } else {
                Ok(args.first().cloned().unwrap_or_else(Value::none))
            }
        }
        m::OR_ELSE => {
            if is_some {
                Ok(receiver)
            } else {
                call_closure(first_arg(), &[])
            }
        }

        m::OK_OR => {
            if is_some {
                Ok(Value::ok(receiver.inner_of()))
            } else {
                Ok(Value::err(first_arg()))
            }
        }
        m::OK_OR_ELSE => {
            if is_some {
                Ok(Value::ok(receiver.inner_of()))
            } else {
                let err_val = call_closure(first_arg(), &[])?;
                Ok(Value::err(err_val))
            }
        }

        m::MAP => {
            if is_some {
                let result = call_closure(first_arg(), &[receiver.inner_of()])?;
                Ok(Value::some(result))
            } else {
                Ok(receiver)
            }
        }
        m::AND_THEN => {
            if is_some {
                call_closure(first_arg(), &[receiver.inner_of()])
            } else {
                Ok(receiver)
            }
        }

        m::CLONE => Ok(receiver.clone()),
        m::TO_STRING => Ok(Value::String(receiver.to_string())),

        _ => Err(format!("no method '{}' on Option", method)),
    }
}

pub fn method_names() -> &'static [&'static str] {
    use symbols::option_result_m as m;
    &[
        m::IS_SOME,
        m::IS_NONE,
        m::UNWRAP,
        m::EXPECT,
        m::UNWRAP_OR,
        m::UNWRAP_OR_ELSE,
        m::OR,
        m::OR_ELSE,
        m::OK_OR,
        m::OK_OR_ELSE,
        m::MAP,
        m::AND_THEN,
        m::CLONE,
        m::TO_STRING,
    ]
}
