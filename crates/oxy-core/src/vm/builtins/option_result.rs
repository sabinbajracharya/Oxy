//! Option and Result method implementations — shared by interpreter and VM.

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
    let is_option =
        matches!(&receiver, Value::EnumVariant { enum_name, .. } if enum_name == "Option");
    let is_result =
        matches!(&receiver, Value::EnumVariant { enum_name, .. } if enum_name == "Result");
    let is_ok = receiver.is_ok_variant();
    let is_some = receiver.is_some_variant();

    match method {
        symbols::option_result_m::IS_SOME if is_option => Ok(Value::Bool(is_some)),
        symbols::option_result_m::IS_NONE if is_option => Ok(Value::Bool(!is_some)),
        symbols::option_result_m::IS_OK if is_result => Ok(Value::Bool(is_ok)),
        symbols::option_result_m::IS_ERR if is_result => Ok(Value::Bool(!is_ok)),
        symbols::option_result_m::UNWRAP => match &receiver {
            Value::EnumVariant { variant, data, .. } if variant == "Some" || variant == "Ok" => {
                Ok(data.first().cloned().unwrap_or(Value::Unit))
            }
            Value::EnumVariant { variant, .. } if variant == "None" => {
                Err("called `unwrap` on a `None` value".into())
            }
            Value::EnumVariant { variant, .. } if variant == "Err" => {
                Err("called `unwrap` on an `Err` value".into())
            }
            _ => Err(format!("no method '{}' on this type", method)),
        },
        symbols::option_result_m::EXPECT => {
            let msg = args
                .first()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "expect failed".into());
            match &receiver {
                Value::EnumVariant { variant, data, .. }
                    if variant == "Some" || variant == "Ok" =>
                {
                    Ok(data.first().cloned().unwrap_or(Value::Unit))
                }
                _ => Err(msg),
            }
        }
        symbols::option_result_m::UNWRAP_OR => match &receiver {
            Value::EnumVariant { variant, data, .. } if variant == "Some" || variant == "Ok" => {
                Ok(data.first().cloned().unwrap_or(Value::Unit))
            }
            _ => Ok(args.first().cloned().unwrap_or(Value::Unit)),
        },
        symbols::option_result_m::UNWRAP_OR_ELSE => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            match &receiver {
                Value::EnumVariant { variant, data, .. }
                    if variant == "Some" || variant == "Ok" =>
                {
                    Ok(data.first().cloned().unwrap_or(Value::Unit))
                }
                Value::EnumVariant { data, .. } if is_option => call_closure(closure, &[]),
                Value::EnumVariant { data, .. } if is_result => {
                    let err_val = data.first().cloned().unwrap_or(Value::Unit);
                    call_closure(closure, &[err_val])
                }
                _ => Err(format!("no method '{}' on this type", method)),
            }
        }
        symbols::option_result_m::OR if is_option => match &receiver {
            Value::EnumVariant { variant, .. } if variant == "Some" => Ok(receiver.clone()),
            _ => Ok(args.first().cloned().unwrap_or(Value::none())),
        },
        symbols::option_result_m::OK_OR if is_option => match &receiver {
            Value::EnumVariant { variant, data, .. } if variant == "Some" => {
                Ok(Value::ok(data.first().cloned().unwrap_or(Value::Unit)))
            }
            _ => Ok(Value::err(args.first().cloned().unwrap_or(Value::Unit))),
        },
        symbols::option_result_m::OK_OR_ELSE if is_option => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            match &receiver {
                Value::EnumVariant { variant, data, .. } if variant == "Some" => {
                    Ok(Value::ok(data.first().cloned().unwrap_or(Value::Unit)))
                }
                _ => {
                    let err_val = call_closure(closure, &[])?;
                    Ok(Value::err(err_val))
                }
            }
        }
        symbols::option_result_m::OR_ELSE => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            match &receiver {
                Value::EnumVariant { variant, .. } if variant == "Some" || variant == "Ok" => {
                    Ok(receiver.clone())
                }
                Value::EnumVariant { data, .. } if is_result => {
                    let err_val = data.first().cloned().unwrap_or(Value::Unit);
                    call_closure(closure, &[err_val])
                }
                _ => call_closure(closure, &[]),
            }
        }
        symbols::option_result_m::MAP => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            match &receiver {
                Value::EnumVariant { variant, data, .. }
                    if variant == "Some" || variant == "Ok" =>
                {
                    let inner = data.first().cloned().unwrap_or(Value::Unit);
                    let result = call_closure(closure, &[inner])?;
                    // Re-wrap in Some/Ok
                    if is_option || is_ok {
                        if is_option {
                            Ok(Value::some(result))
                        } else {
                            Ok(Value::ok(result))
                        }
                    } else {
                        Err("map called on non-Option/Result".into())
                    }
                }
                Value::EnumVariant { .. } => Ok(receiver.clone()), // None/Err pass through
                _ => Err(format!("no method '{}' on this type", method)),
            }
        }
        symbols::option_result_m::MAP_ERR => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            match &receiver {
                Value::EnumVariant { variant, data, .. } if variant == "Ok" => Ok(receiver.clone()),
                Value::EnumVariant { variant, data, .. } if variant == "Err" => {
                    let inner = data.first().cloned().unwrap_or(Value::Unit);
                    let result = call_closure(closure, &[inner])?;
                    Ok(Value::err(result))
                }
                _ => Err(format!("no method '{}' on this type", method)),
            }
        }
        symbols::option_result_m::AND_THEN => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            match &receiver {
                Value::EnumVariant { variant, data, .. }
                    if variant == "Some" || variant == "Ok" =>
                {
                    let inner = data.first().cloned().unwrap_or(Value::Unit);
                    call_closure(closure, &[inner])
                }
                _ => Ok(receiver.clone()),
            }
        }
        symbols::option_result_m::UNWRAP_ERR if is_result => match &receiver {
            Value::EnumVariant { variant, data, .. } if variant == "Err" => {
                Ok(data.first().cloned().unwrap_or(Value::Unit))
            }
            _ => Err("called `unwrap_err` on an `Ok` value".into()),
        },
        symbols::option_result_m::OK if is_result => match &receiver {
            Value::EnumVariant { variant, data, .. } if variant == "Ok" => {
                Ok(Value::some(data.first().cloned().unwrap_or(Value::Unit)))
            }
            _ => Ok(Value::none()),
        },
        symbols::option_result_m::ERR if is_result => match &receiver {
            Value::EnumVariant { variant, data, .. } if variant == "Err" => {
                Ok(Value::some(data.first().cloned().unwrap_or(Value::Unit)))
            }
            _ => Ok(Value::none()),
        },
        symbols::option_result_m::CLONE => Ok(receiver.clone()),
        symbols::option_result_m::TO_STRING => Ok(Value::String(receiver.to_string())),
        _ => Err(format!(
            "no method '{}' on type {}",
            method,
            receiver.type_name()
        )),
    }
}

pub fn method_names() -> &'static [&'static str] {
    &[
        symbols::option_result_m::IS_SOME,
        symbols::option_result_m::IS_NONE,
        symbols::option_result_m::IS_OK,
        symbols::option_result_m::IS_ERR,
        symbols::option_result_m::UNWRAP,
        symbols::option_result_m::EXPECT,
        symbols::option_result_m::UNWRAP_OR,
        symbols::option_result_m::UNWRAP_OR_ELSE,
        symbols::option_result_m::OR,
        symbols::option_result_m::OR_ELSE,
        symbols::option_result_m::OK_OR,
        symbols::option_result_m::OK_OR_ELSE,
        symbols::option_result_m::MAP,
        symbols::option_result_m::MAP_ERR,
        symbols::option_result_m::AND_THEN,
        symbols::option_result_m::UNWRAP_ERR,
        symbols::option_result_m::OK,
        symbols::option_result_m::ERR,
        symbols::option_result_m::CLONE,
        symbols::option_result_m::TO_STRING,
    ]
}
