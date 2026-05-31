//! String method implementations — shared by interpreter and VM.

use crate::symbols;
use crate::types::Value;

pub fn dispatch(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String> {
    let Value::String(s) = &receiver else {
        unreachable!()
    };
    match method {
        symbols::string_m::LEN => Ok(Value::I64(s.chars().count() as i64)),
        symbols::string_m::IS_EMPTY => Ok(Value::Bool(s.is_empty())),
        symbols::string_m::TO_UPPERCASE => Ok(Value::String(s.to_uppercase())),
        symbols::string_m::TO_LOWERCASE => Ok(Value::String(s.to_lowercase())),
        symbols::string_m::TRIM => Ok(Value::String(s.trim().to_string())),
        symbols::string_m::CONTAINS => {
            let pat = args.first().map(|v| v.to_string()).unwrap_or_default();
            Ok(Value::Bool(s.contains(&pat)))
        }
        symbols::string_m::STARTS_WITH => {
            let pat = args.first().map(|v| v.to_string()).unwrap_or_default();
            Ok(Value::Bool(s.starts_with(&pat)))
        }
        symbols::string_m::ENDS_WITH => {
            let pat = args.first().map(|v| v.to_string()).unwrap_or_default();
            Ok(Value::Bool(s.ends_with(&pat)))
        }
        symbols::string_m::REPLACE => {
            let from = args.first().map(|v| v.to_string()).unwrap_or_default();
            let to = args.get(1).map(|v| v.to_string()).unwrap_or_default();
            Ok(Value::String(s.replace(&from, &to)))
        }
        symbols::string_m::LINES => {
            let parts: Vec<Value> = s.lines().map(|l| Value::String(l.to_string())).collect();
            Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(parts))))
        }
        symbols::string_m::SPLIT_WHITESPACE => {
            let parts: Vec<Value> = s
                .split_whitespace()
                .map(|p| Value::String(p.to_string()))
                .collect();
            Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(parts))))
        }
        symbols::string_m::SPLIT => {
            let pat = args.first().map(|v| v.to_string()).unwrap_or_default();
            let parts: Vec<Value> = s
                .split(&pat)
                .map(|p| Value::String(p.to_string()))
                .collect();
            Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(parts))))
        }
        symbols::string_m::CHARS => {
            let chars: Vec<Value> = s.chars().map(Value::Char).collect();
            Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(chars))))
        }
        symbols::string_m::REPEAT => {
            let n = args
                .first()
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(1);
            Ok(Value::String(s.repeat(n)))
        }
        symbols::string_m::PUSH_STR => {
            Err("String::push_str is unsupported (strings are immutable in Oxy)".to_string())
        }
        symbols::string_m::CHAR_AT => {
            let i = args
                .first()
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(0);
            Ok(s.chars().nth(i).map(Value::Char).unwrap_or(Value::Unit))
        }
        symbols::string_m::SUBSTRING => {
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
        symbols::string_m::PARSE_INT => {
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
        symbols::string_m::PARSE_FLOAT => match s.trim().parse::<f64>() {
            Ok(n) => Ok(Value::ok(Value::F64(n))),
            Err(_) => Ok(Value::err(Value::String(format!(
                "cannot parse \"{s}\" as float"
            )))),
        },
        symbols::string_m::FIND => {
            let pat = args.first().map(|v| v.to_string()).unwrap_or_default();
            match s.find(&pat) {
                Some(byte_idx) => {
                    let char_idx = s[..byte_idx].chars().count() as i64;
                    Ok(Value::some(Value::I64(char_idx)))
                }
                None => Ok(Value::none()),
            }
        }
        symbols::string_m::CLONE => Ok(Value::String(s.clone())),
        symbols::string_m::TO_STRING => Ok(Value::String(s.clone())),
        _ => Err(format!("no method '{}' on type String", method)),
    }
}

pub fn method_names() -> &'static [&'static str] {
    &[
        symbols::string_m::LEN,
        symbols::string_m::IS_EMPTY,
        symbols::string_m::TO_UPPERCASE,
        symbols::string_m::TO_LOWERCASE,
        symbols::string_m::TRIM,
        symbols::string_m::CONTAINS,
        symbols::string_m::STARTS_WITH,
        symbols::string_m::ENDS_WITH,
        symbols::string_m::REPLACE,
        symbols::string_m::LINES,
        symbols::string_m::SPLIT,
        symbols::string_m::SPLIT_WHITESPACE,
        symbols::string_m::CHARS,
        symbols::string_m::REPEAT,
        symbols::string_m::PUSH_STR,
        symbols::string_m::CHAR_AT,
        symbols::string_m::SUBSTRING,
        symbols::string_m::PARSE_INT,
        symbols::string_m::PARSE_FLOAT,
        symbols::string_m::FIND,
        symbols::string_m::CLONE,
        symbols::string_m::TO_STRING,
    ]
}
