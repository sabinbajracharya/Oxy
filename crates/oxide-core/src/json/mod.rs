//! Hand-written JSON serializer and deserializer for Oxide values.

use crate::types::{Value, OPTION_TYPE, RESULT_TYPE};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

pub fn serialize(value: &Value) -> Result<String, String> {
    serialize_value(value)
}

pub fn serialize_pretty(value: &Value) -> Result<String, String> {
    serialize_value_pretty(value, 0)
}

fn serialize_value(value: &Value) -> Result<String, String> {
    match value {
        Value::Integer(n) => Ok(n.to_string()),
        Value::Float(f) => Ok(format_float(*f)),
        Value::Bool(b) => Ok(b.to_string()),
        Value::String(s) => Ok(escape_json_string(s)),
        Value::Char(c) => Ok(escape_json_string(&c.to_string())),
        Value::Unit => Ok("null".to_string()),
        Value::Vec(v) | Value::Tuple(v) => {
            let items: Result<Vec<String>, String> = v.iter().map(serialize_value).collect();
            Ok(format!("[{}]", items?.join(", ")))
        }
        Value::HashMap(m) => serialize_map(m),
        Value::Struct { fields, .. } => serialize_map(fields),
        Value::EnumVariant {
            enum_name,
            variant,
            data,
        } => serialize_enum(enum_name, variant, data),
        Value::Function(_) => Err("cannot serialize function".to_string()),
        Value::Range(..) => Err("cannot serialize range".to_string()),
        Value::Future(_) => Err("cannot serialize future".to_string()),
        Value::JoinHandle(_) => Err("cannot serialize join handle".to_string()),
    }
}

fn serialize_value_pretty(value: &Value, indent: usize) -> Result<String, String> {
    match value {
        Value::Integer(n) => Ok(n.to_string()),
        Value::Float(f) => Ok(format_float(*f)),
        Value::Bool(b) => Ok(b.to_string()),
        Value::String(s) => Ok(escape_json_string(s)),
        Value::Char(c) => Ok(escape_json_string(&c.to_string())),
        Value::Unit => Ok("null".to_string()),
        Value::Vec(v) | Value::Tuple(v) => {
            if v.is_empty() {
                return Ok("[]".to_string());
            }
            let inner_indent = indent + 2;
            let pad = " ".repeat(inner_indent);
            let close_pad = " ".repeat(indent);
            let items: Result<Vec<String>, String> = v
                .iter()
                .map(|item| {
                    let s = serialize_value_pretty(item, inner_indent)?;
                    Ok(format!("{pad}{s}"))
                })
                .collect();
            Ok(format!("[\n{}\n{close_pad}]", items?.join(",\n")))
        }
        Value::HashMap(m) => serialize_map_pretty(m, indent),
        Value::Struct { fields, .. } => serialize_map_pretty(fields, indent),
        Value::EnumVariant {
            enum_name,
            variant,
            data,
        } => serialize_enum_pretty(enum_name, variant, data, indent),
        Value::Function(_) => Err("cannot serialize function".to_string()),
        Value::Range(..) => Err("cannot serialize range".to_string()),
        Value::Future(_) => Err("cannot serialize future".to_string()),
        Value::JoinHandle(_) => Err("cannot serialize join handle".to_string()),
    }
}

fn format_float(f: f64) -> String {
    let s = f.to_string();
    if s.ends_with('.') {
        format!("{s}0")
    } else {
        s
    }
}

fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if c.is_control() => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn sorted_keys(m: &HashMap<String, Value>) -> Vec<&String> {
    let mut keys: Vec<&String> = m.keys().collect();
    keys.sort();
    keys
}

fn serialize_map(m: &HashMap<String, Value>) -> Result<String, String> {
    let keys = sorted_keys(m);
    let pairs: Result<Vec<String>, String> = keys
        .iter()
        .map(|k| {
            let v = serialize_value(&m[*k])?;
            Ok(format!("{}: {v}", escape_json_string(k)))
        })
        .collect();
    Ok(format!("{{{}}}", pairs?.join(", ")))
}

fn serialize_map_pretty(m: &HashMap<String, Value>, indent: usize) -> Result<String, String> {
    if m.is_empty() {
        return Ok("{}".to_string());
    }
    let keys = sorted_keys(m);
    let inner_indent = indent + 2;
    let pad = " ".repeat(inner_indent);
    let close_pad = " ".repeat(indent);
    let pairs: Result<Vec<String>, String> = keys
        .iter()
        .map(|k| {
            let v = serialize_value_pretty(&m[*k], inner_indent)?;
            Ok(format!("{pad}{}: {v}", escape_json_string(k)))
        })
        .collect();
    Ok(format!("{{\n{}\n{close_pad}}}", pairs?.join(",\n")))
}

fn serialize_enum(enum_name: &str, variant: &str, data: &[Value]) -> Result<String, String> {
    // Special case: Option
    if enum_name == OPTION_TYPE {
        return match variant {
            "None" => Ok("null".to_string()),
            "Some" if data.len() == 1 => serialize_value(&data[0]),
            _ => Ok(format!(
                "{{\"variant\": {}, \"data\": {}}}",
                escape_json_string(variant),
                serialize_data(data)?
            )),
        };
    }
    // Special case: Result
    if enum_name == RESULT_TYPE {
        return match variant {
            "Ok" if data.len() == 1 => {
                let v = serialize_value(&data[0])?;
                Ok(format!("{{\"Ok\": {v}}}"))
            }
            "Err" if data.len() == 1 => {
                let v = serialize_value(&data[0])?;
                Ok(format!("{{\"Err\": {v}}}"))
            }
            _ => Ok(format!(
                "{{\"variant\": {}, \"data\": {}}}",
                escape_json_string(variant),
                serialize_data(data)?
            )),
        };
    }
    // General enum
    if data.is_empty() {
        Ok(escape_json_string(variant))
    } else if data.len() == 1 {
        let v = serialize_value(&data[0])?;
        Ok(format!(
            "{{\"variant\": {}, \"data\": {v}}}",
            escape_json_string(variant)
        ))
    } else {
        Ok(format!(
            "{{\"variant\": {}, \"data\": {}}}",
            escape_json_string(variant),
            serialize_data(data)?
        ))
    }
}

fn serialize_enum_pretty(
    enum_name: &str,
    variant: &str,
    data: &[Value],
    indent: usize,
) -> Result<String, String> {
    // Special case: Option
    if enum_name == OPTION_TYPE {
        return match variant {
            "None" => Ok("null".to_string()),
            "Some" if data.len() == 1 => serialize_value_pretty(&data[0], indent),
            _ => serialize_generic_enum_pretty(variant, data, indent),
        };
    }
    // Special case: Result
    if enum_name == RESULT_TYPE {
        return match variant {
            "Ok" if data.len() == 1 => {
                let inner_indent = indent + 2;
                let pad = " ".repeat(inner_indent);
                let close_pad = " ".repeat(indent);
                let v = serialize_value_pretty(&data[0], inner_indent)?;
                Ok(format!("{{\n{pad}\"Ok\": {v}\n{close_pad}}}"))
            }
            "Err" if data.len() == 1 => {
                let inner_indent = indent + 2;
                let pad = " ".repeat(inner_indent);
                let close_pad = " ".repeat(indent);
                let v = serialize_value_pretty(&data[0], inner_indent)?;
                Ok(format!("{{\n{pad}\"Err\": {v}\n{close_pad}}}"))
            }
            _ => serialize_generic_enum_pretty(variant, data, indent),
        };
    }
    // General enum
    if data.is_empty() {
        Ok(escape_json_string(variant))
    } else {
        serialize_generic_enum_pretty(variant, data, indent)
    }
}

fn serialize_generic_enum_pretty(
    variant: &str,
    data: &[Value],
    indent: usize,
) -> Result<String, String> {
    let inner_indent = indent + 2;
    let pad = " ".repeat(inner_indent);
    let close_pad = " ".repeat(indent);
    let data_str = if data.len() == 1 {
        serialize_value_pretty(&data[0], inner_indent)?
    } else {
        let arr_indent = inner_indent + 2;
        let arr_pad = " ".repeat(arr_indent);
        let items: Result<Vec<String>, String> = data
            .iter()
            .map(|item| {
                let s = serialize_value_pretty(item, arr_indent)?;
                Ok(format!("{arr_pad}{s}"))
            })
            .collect();
        format!("[\n{}\n{pad}]", items?.join(",\n"))
    };
    Ok(format!(
        "{{\n{pad}\"variant\": {},\n{pad}\"data\": {data_str}\n{close_pad}}}",
        escape_json_string(variant)
    ))
}

fn serialize_data(data: &[Value]) -> Result<String, String> {
    if data.len() == 1 {
        return serialize_value(&data[0]);
    }
    let items: Result<Vec<String>, String> = data.iter().map(serialize_value).collect();
    Ok(format!("[{}]", items?.join(", ")))
}

// ---------------------------------------------------------------------------
// Deserialization
// ---------------------------------------------------------------------------

pub fn deserialize(input: &str) -> Result<Value, String> {
    let mut parser = JsonParser::new(input);
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.pos < parser.input.len() {
        return Err(format!(
            "unexpected trailing characters at position {}",
            parser.pos
        ));
    }
    Ok(value)
}

struct JsonParser {
    input: Vec<char>,
    pos: usize,
}

impl JsonParser {
    fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            match self.input[self.pos] {
                ' ' | '\t' | '\n' | '\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.input.get(self.pos).copied();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        match self.advance() {
            Some(ch) if ch == expected => Ok(()),
            Some(ch) => Err(format!(
                "expected '{expected}' but found '{ch}' at position {}",
                self.pos - 1
            )),
            None => Err(format!("expected '{expected}' but found end of input")),
        }
    }

    fn parse_value(&mut self) -> Result<Value, String> {
        self.skip_whitespace();
        match self.peek() {
            None => Err("unexpected end of input".to_string()),
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => self.parse_string().map(Value::String),
            Some('t') => self.parse_literal("true").map(|_| Value::Bool(true)),
            Some('f') => self.parse_literal("false").map(|_| Value::Bool(false)),
            Some('n') => self.parse_literal("null").map(|_| Value::Unit),
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_number(),
            Some(c) => Err(format!(
                "unexpected character '{c}' at position {}",
                self.pos
            )),
        }
    }

    fn parse_object(&mut self) -> Result<Value, String> {
        self.expect('{')?;
        self.skip_whitespace();
        let mut map = HashMap::new();
        if self.peek() == Some('}') {
            self.advance();
            return Ok(Value::HashMap(map));
        }
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(':')?;
            let value = self.parse_value()?;
            map.insert(key, value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.advance();
                }
                Some('}') => {
                    self.advance();
                    return Ok(Value::HashMap(map));
                }
                Some(c) => {
                    return Err(format!(
                        "expected ',' or '}}' but found '{c}' at position {}",
                        self.pos
                    ));
                }
                None => return Err("unexpected end of input in object".to_string()),
            }
        }
    }

    fn parse_array(&mut self) -> Result<Value, String> {
        self.expect('[')?;
        self.skip_whitespace();
        let mut items = Vec::new();
        if self.peek() == Some(']') {
            self.advance();
            return Ok(Value::Vec(items));
        }
        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.advance();
                }
                Some(']') => {
                    self.advance();
                    return Ok(Value::Vec(items));
                }
                Some(c) => {
                    return Err(format!(
                        "expected ',' or ']' but found '{c}' at position {}",
                        self.pos
                    ));
                }
                None => return Err("unexpected end of input in array".to_string()),
            }
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        self.expect('"')?;
        let mut s = String::new();
        loop {
            match self.advance() {
                None => return Err("unterminated string".to_string()),
                Some('"') => return Ok(s),
                Some('\\') => match self.advance() {
                    None => return Err("unterminated escape sequence".to_string()),
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some('/') => s.push('/'),
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('r') => s.push('\r'),
                    Some('b') => s.push('\u{08}'),
                    Some('f') => s.push('\u{0C}'),
                    Some('u') => {
                        let cp = self.parse_unicode_escape()?;
                        if let Some(c) = char::from_u32(cp) {
                            s.push(c);
                        } else {
                            return Err(format!("invalid unicode codepoint: {cp:#x}"));
                        }
                    }
                    Some(c) => return Err(format!("invalid escape character: '{c}'")),
                },
                Some(c) => s.push(c),
            }
        }
    }

    fn parse_unicode_escape(&mut self) -> Result<u32, String> {
        let mut hex = String::with_capacity(4);
        for _ in 0..4 {
            match self.advance() {
                Some(c) if c.is_ascii_hexdigit() => hex.push(c),
                Some(c) => return Err(format!("invalid hex digit '{c}' in unicode escape")),
                None => return Err("unterminated unicode escape".to_string()),
            }
        }
        u32::from_str_radix(&hex, 16).map_err(|e| format!("invalid unicode escape: {e}"))
    }

    fn parse_number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        let mut is_float = false;

        if self.peek() == Some('-') {
            self.advance();
        }

        // Integer part
        if self.peek() == Some('0') {
            self.advance();
        } else {
            self.consume_digits()?;
        }

        // Fractional part
        if self.peek() == Some('.') {
            is_float = true;
            self.advance();
            self.consume_digits()?;
        }

        // Exponent
        if matches!(self.peek(), Some('e' | 'E')) {
            is_float = true;
            self.advance();
            if matches!(self.peek(), Some('+' | '-')) {
                self.advance();
            }
            self.consume_digits()?;
        }

        let num_str: String = self.input[start..self.pos].iter().collect();
        if is_float {
            num_str
                .parse::<f64>()
                .map(Value::Float)
                .map_err(|e| format!("invalid float: {e}"))
        } else {
            num_str
                .parse::<i64>()
                .map(Value::Integer)
                .map_err(|e| format!("invalid integer: {e}"))
        }
    }

    fn consume_digits(&mut self) -> Result<(), String> {
        if !matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            return Err(format!("expected digit at position {}", self.pos));
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.advance();
        }
        Ok(())
    }

    fn parse_literal(&mut self, literal: &str) -> Result<(), String> {
        for expected in literal.chars() {
            match self.advance() {
                Some(c) if c == expected => {}
                _ => return Err(format!("expected '{literal}' at position {}", self.pos)),
            }
        }
        Ok(())
    }
}
