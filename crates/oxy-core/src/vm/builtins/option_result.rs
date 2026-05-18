//! Option and Result method implementations — shared by interpreter and VM.

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
    let is_option = matches!(&receiver, Value::EnumVariant { enum_name, .. } if enum_name == "Option");
    let is_result = matches!(&receiver, Value::EnumVariant { enum_name, .. } if enum_name == "Result");
    let is_ok = receiver.is_ok_variant();
    let is_some = receiver.is_some_variant();

    match method {
        "is_some" if is_option => {
            Ok(Value::Bool(is_some))
        }
        "is_none" if is_option => {
            Ok(Value::Bool(!is_some))
        }
        "is_ok" if is_result => {
            Ok(Value::Bool(is_ok))
        }
        "is_err" if is_result => {
            Ok(Value::Bool(!is_ok))
        }
        "unwrap" => match &receiver {
            Value::EnumVariant { variant, data, .. }
                if variant == "Some" || variant == "Ok" =>
            {
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
        "expect" => {
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
        "unwrap_or" => match &receiver {
            Value::EnumVariant { variant, data, .. }
                if variant == "Some" || variant == "Ok" =>
            {
                Ok(data.first().cloned().unwrap_or(Value::Unit))
            }
            _ => Ok(args.first().cloned().unwrap_or(Value::Unit)),
        },
        "unwrap_or_else" => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            match &receiver {
                Value::EnumVariant { variant, data, .. }
                    if variant == "Some" || variant == "Ok" =>
                {
                    Ok(data.first().cloned().unwrap_or(Value::Unit))
                }
                _ => call_closure(closure, &[]),
            }
        }
        "map" => {
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
        "map_err" => {
            let closure = args.first().cloned().unwrap_or(Value::Unit);
            match &receiver {
                Value::EnumVariant { variant, data, .. } if variant == "Ok" => {
                    Ok(receiver.clone())
                }
                Value::EnumVariant { variant, data, .. } if variant == "Err" => {
                    let inner = data.first().cloned().unwrap_or(Value::Unit);
                    let result = call_closure(closure, &[inner])?;
                    Ok(Value::err(result))
                }
                _ => Err(format!("no method '{}' on this type", method)),
            }
        }
        "and_then" => {
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
        "unwrap_err" if is_result => {
            match &receiver {
                Value::EnumVariant { variant, data, .. } if variant == "Err" => {
                    Ok(data.first().cloned().unwrap_or(Value::Unit))
                }
                _ => Err("called `unwrap_err` on an `Ok` value".into()),
            }
        },
        "ok" if is_result => match &receiver {
            Value::EnumVariant { variant, data, .. } if variant == "Ok" => {
                Ok(Value::some(data.first().cloned().unwrap_or(Value::Unit)))
            }
            _ => Ok(Value::none()),
        },
        "err" if is_result => match &receiver {
            Value::EnumVariant { variant, data, .. } if variant == "Err" => {
                Ok(Value::some(data.first().cloned().unwrap_or(Value::Unit)))
            }
            _ => Ok(Value::none()),
        },
        "clone" => Ok(receiver.clone()),
        "to_string" => Ok(Value::String(receiver.to_string())),
        _ => Err(format!(
            "no method '{}' on type {}",
            method,
            receiver.type_name()
        )),
    }
}
