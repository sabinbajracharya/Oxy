//! String method implementations — shared by interpreter and VM.

use crate::types::Value;

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let Value::String(s) = &receiver else {
        unreachable!()
    };
    match method {
        "len" => Ok(Value::I64(s.chars().count() as i64)),
        "is_empty" => Ok(Value::Bool(s.is_empty())),
        "to_uppercase" => Ok(Value::String(s.to_uppercase())),
        "to_lowercase" => Ok(Value::String(s.to_lowercase())),
        "trim" => Ok(Value::String(s.trim().to_string())),
        "contains" => {
            let pat = args.first().map(|v| v.to_string()).unwrap_or_default();
            Ok(Value::Bool(s.contains(&pat)))
        }
        "starts_with" => {
            let pat = args.first().map(|v| v.to_string()).unwrap_or_default();
            Ok(Value::Bool(s.starts_with(&pat)))
        }
        "ends_with" => {
            let pat = args.first().map(|v| v.to_string()).unwrap_or_default();
            Ok(Value::Bool(s.ends_with(&pat)))
        }
        "replace" => {
            let from = args.first().map(|v| v.to_string()).unwrap_or_default();
            let to = args.get(1).map(|v| v.to_string()).unwrap_or_default();
            Ok(Value::String(s.replace(&from, &to)))
        }
        "split" => {
            let pat = args.first().map(|v| v.to_string()).unwrap_or_default();
            let parts: Vec<Value> = s
                .split(&pat)
                .map(|p| Value::String(p.to_string()))
                .collect();
            Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(parts))))
        }
        "chars" => {
            let chars: Vec<Value> = s.chars().map(Value::Char).collect();
            Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(chars))))
        }
        "repeat" => {
            let n = args
                .first()
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(1);
            Ok(Value::String(s.repeat(n)))
        }
        "push_str" => {
            eprintln!("String::push_str is unsupported (strings are immutable in Oxy)");
            Ok(Value::Unit)
        }
        "char_at" => {
            let i = args
                .first()
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(0);
            Ok(s.chars().nth(i).map(Value::Char).unwrap_or(Value::Unit))
        }
        "substring" => {
            let start = args
                .first()
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(0);
            let end = args
                .get(1)
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(0);
            let chars: Vec<char> = s.chars().collect();
            if start <= end && end <= chars.len() {
                Ok(Value::String(chars[start..end].iter().collect()))
            } else {
                Err(format!("substring: invalid range {}..{}", start, end))
            }
        }
        "parse_int" => {
            let trimmed = s.trim();
            let result = if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
                i64::from_str_radix(&trimmed[2..], 16).map_err(|_| ())
            } else {
                trimmed.parse::<i64>().map_err(|_| ())
            };
            match result {
                Ok(n) => Ok(Value::ok(Value::I64(n))),
                Err(_) => Ok(Value::err(Value::String(format!(
                    "cannot parse \"{s}\" as integer"
                )))),
            }
        }
        "parse_float" => match s.trim().parse::<f64>() {
            Ok(n) => Ok(Value::ok(Value::F64(n))),
            Err(_) => Ok(Value::err(Value::String(format!(
                "cannot parse \"{s}\" as float"
            )))),
        },
        "clone" => Ok(Value::String(s.clone())),
        "to_string" => Ok(Value::String(s.clone())),
        _ => Err(format!("no method '{}' on type String", method)),
    }
}
