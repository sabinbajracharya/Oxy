//! Debug formatting for Ferrite values.
//!
//! Provides `{:?}`-style debug output used by `dbg!` and format strings
//! with the `{:?}` specifier.

use crate::types::{Value, OPTION_TYPE, RESULT_TYPE};

/// Recursively format a value in debug representation (like Rust's `{:?}`).
///
/// Strings are quoted, chars are single-quoted, collections show their
/// contents recursively, and structs/enums show field names and values.
pub(crate) fn debug_format(val: &Value) -> String {
    match val {
        Value::String(s) => format!("\"{s}\""),
        Value::Char(c) => format!("'{c}'"),
        Value::Vec(v) => {
            let items: Vec<String> = v.iter().map(debug_format).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Tuple(t) => {
            let items: Vec<String> = t.iter().map(debug_format).collect();
            if t.len() == 1 {
                format!("({},)", items[0])
            } else {
                format!("({})", items.join(", "))
            }
        }
        Value::Struct { name, fields } => {
            let mut sorted: Vec<_> = fields.iter().collect();
            sorted.sort_by_key(|(k, _)| (*k).clone());
            let items: Vec<String> = sorted
                .iter()
                .map(|(k, v)| format!("{k}: {}", debug_format(v)))
                .collect();
            format!("{name} {{ {} }}", items.join(", "))
        }
        Value::EnumVariant {
            enum_name,
            variant,
            data,
        } => {
            // Built-in Option/Result: show without enum prefix for readability
            let prefix = if enum_name == OPTION_TYPE || enum_name == RESULT_TYPE {
                String::new()
            } else {
                format!("{enum_name}::")
            };
            if data.is_empty() {
                format!("{prefix}{variant}")
            } else {
                let items: Vec<String> = data.iter().map(debug_format).collect();
                format!("{prefix}{variant}({})", items.join(", "))
            }
        }
        Value::HashMap(m) => {
            let mut sorted: Vec<_> = m.iter().collect();
            sorted.sort_by_key(|(k, _)| (*k).clone());
            let items: Vec<String> = sorted
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}: {}",
                        debug_format(&Value::String(k.to_string())),
                        debug_format(v)
                    )
                })
                .collect();
            format!("{{{}}}", items.join(", "))
        }
        Value::Future(f) => format!("Future<{}>", f.name),
        Value::JoinHandle(v) => format!("JoinHandle({})", debug_format(v)),
        other => format!("{other}"),
    }
}
