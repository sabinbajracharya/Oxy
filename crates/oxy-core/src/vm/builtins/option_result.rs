//! Option and Result method implementations — shared by interpreter and VM.

use crate::types::Value;

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
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
        "unwrap_or_else" => Err("unwrap_or_else: closure calls not supported in VM builtins yet".into()),
        "map" => Err("map: closure calls not supported in VM builtins yet".into()),
        "map_err" => Err("map_err: closure calls not supported in VM builtins yet".into()),
        "and_then" => Err("and_then: closure calls not supported in VM builtins yet".into()),
        "unwrap_err" if is_result => Err("unwrap_err not implemented".into()),
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
