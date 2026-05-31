//! Display formatting for [`Value`] — `Display`, debug rendering, and the
//! `format!`/`print!` template engine.
//!
//! Extracted from [`super`] to keep the Value definition file under ~700 lines.

use std::fmt;

use super::{Value, OPTION_TYPE, RESULT_TYPE};

fn float_display(n: f64, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if n.fract() == 0.0 {
        write!(f, "{n:.1}")
    } else {
        write!(f, "{n}")
    }
}

/// Formats a [`Value`] for user-facing display (e.g. `println!`).
/// Shared renderer for [`Value`]. In `debug` mode (Rust `{:?}`-style) strings
/// and chars are quoted and that quoting recurses through collections, tuples,
/// struct field values, enum payloads, and map keys/values; in display mode the
/// textual scalars are written bare. Every other variant renders identically in
/// both modes, so `Display` and `to_debug_string` share this one walk.
struct ValueFmt<'a> {
    value: &'a Value,
    debug: bool,
}

/// Wrap a nested value so it inherits the current `debug` flag during recursion.
fn vf(value: &Value, debug: bool) -> ValueFmt<'_> {
    ValueFmt { value, debug }
}

impl fmt::Display for ValueFmt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let debug = self.debug;
        match self.value {
            Value::I64(n) => write!(f, "{n}"),
            Value::U8(n) => write!(f, "{n}"),
            Value::F64(n) => float_display(*n, f),
            Value::Bool(b) => write!(f, "{b}"),
            Value::String(s) => {
                if debug {
                    write!(f, "{s:?}")
                } else {
                    write!(f, "{s}")
                }
            }
            Value::Char(c) => {
                if debug {
                    write!(f, "{c:?}")
                } else {
                    write!(f, "{c}")
                }
            }
            Value::Unit => write!(f, "()"),
            Value::Function(func) => write!(f, "<fn {}>", func.name),
            Value::Range(start, end) => write!(f, "{start}..{end}"),
            Value::Vec(rc) => {
                let v = rc.borrow();
                write!(f, "[")?;
                for (i, elem) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", vf(elem, debug))?;
                }
                write!(f, "]")
            }
            Value::Array(a) => {
                write!(f, "[")?;
                for (i, elem) in a.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", vf(elem, debug))?;
                }
                write!(f, "]")
            }
            Value::Tuple(t) => {
                write!(f, "(")?;
                for (i, elem) in t.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", vf(elem, debug))?;
                }
                if t.len() == 1 {
                    write!(f, ",")?;
                }
                write!(f, ")")
            }
            Value::Struct { name, fields } => {
                write!(f, "{name} {{ ")?;
                let mut sorted: Vec<_> = fields.iter().collect();
                sorted.sort_by_key(|(k, _)| (*k).clone());
                for (i, (k, v)) in sorted.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {}", vf(v, debug))?;
                }
                write!(f, " }}")
            }
            Value::EnumVariant {
                enum_name,
                variant,
                data,
            } => {
                // Built-in Option/Result: show without enum prefix
                if enum_name == OPTION_TYPE || enum_name == RESULT_TYPE {
                    write!(f, "{variant}")?;
                } else {
                    write!(f, "{enum_name}::{variant}")?;
                }
                if !data.is_empty() {
                    write!(f, "(")?;
                    for (i, v) in data.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", vf(v, debug))?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Value::HashMap(rc) => {
                let m = rc.borrow();
                write!(f, "{{")?;
                let mut sorted: Vec<_> = m.iter().collect();
                sorted.sort_by(|(a, _), (b, _)| a.cmp(b));
                for (i, (k, v)) in sorted.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", vf(k, debug), vf(v, debug))?;
                }
                write!(f, "}}")
            }
            Value::HashSet(rc) => {
                let s = rc.borrow();
                write!(f, "{{")?;
                let mut sorted: Vec<&Value> = s.iter().collect();
                sorted.sort();
                for (i, elem) in sorted.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", vf(elem, debug))?;
                }
                write!(f, "}}")
            }
            Value::BTreeMap(rc) => {
                let m = rc.borrow();
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", vf(k, debug), vf(v, debug))?;
                }
                write!(f, "}}")
            }
            Value::BTreeSet(rc) => {
                let s = rc.borrow();
                write!(f, "{{")?;
                for (i, elem) in s.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", vf(elem, debug))?;
                }
                write!(f, "}}")
            }
            Value::BinaryHeap(rc) => {
                write!(f, "BinaryHeap([")?;
                let sorted = rc.borrow().clone().into_sorted_vec();
                for (i, elem) in sorted.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", vf(elem, debug))?;
                }
                write!(f, "])")
            }
            Value::VecDeque(rc) => {
                write!(f, "VecDeque([")?;
                for (i, elem) in rc.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", vf(elem, debug))?;
                }
                write!(f, "])")
            }
            Value::Iterator(_) => write!(f, "<iterator>"),
            Value::Future(_) => write!(f, "<future>"),
            Value::JoinHandle { task_id } => write!(f, "<join_handle {}>", task_id),
            Value::AsyncResult { .. } => write!(f, "<async_result>"),
            Value::Cell(rc) => write!(f, "{}", vf(&rc.borrow(), debug)),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ValueFmt {
            value: self,
            debug: false,
        }
        .fmt(f)
    }
}

impl Value {
    /// Rust-`{:?}`-style debug rendering: strings and chars are quoted, and the
    /// quoting recurses through nested collections, tuples, struct field
    /// values, enum payloads, and map keys/values. Numbers, bools, unit, and
    /// the opaque variants render the same as [`Display`](std::fmt::Display).
    pub fn to_debug_string(&self) -> String {
        ValueFmt {
            value: self,
            debug: true,
        }
        .to_string()
    }
}

/// Render a `format!`-style template against positional arguments. Supports
/// `{}` (Display), `{:?}` (Debug, via [`Value::to_debug_string`]), and the
/// `{{` / `}}` escapes. Shared by the JIT FFI print/format builtins and by
/// `assert!`'s optional message — it is pure `Value`→`String` formatting with
/// no backend dependency, so it lives here (wasm-safe) rather than in the
/// Cranelift-gated `jit` module.
pub(crate) fn format_template(template: &str, args: &[Value]) -> String {
    // No Display hook: `{}` falls back to each value's default `to_string`. Used
    // by callers that have no access to the JIT (stdlib `assert`, wasm builds).
    format_template_with(template, args, |_| None)
}

/// Like [`format_template`], but a `{}` placeholder first consults `display` to
/// let a value render through a user-defined `Display::fmt`. `display` returns
/// `Some(rendered)` to override, or `None` to fall back to the default
/// `to_string`. `{:?}` placeholders always use Debug and ignore the hook. Keeping
/// the placeholder-parsing logic here (rather than in the JIT FFI) means there is
/// one template engine, wasm-safe by default, with Display dispatch layered on
/// only where a backend can supply it.
pub(crate) fn format_template_with(
    template: &str,
    args: &[Value],
    mut display: impl FnMut(&Value) -> Option<String>,
) -> String {
    let mut result = String::new();
    let mut arg_idx = 0;
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                result.push('{');
            }
            '{' => {
                // Consume the placeholder body / format spec up to `}`, noting
                // whether it requested Debug (`?`) rendering.
                let mut is_debug = false;
                for cc in chars.by_ref() {
                    if cc == '}' {
                        break;
                    }
                    if cc == '?' {
                        is_debug = true;
                    }
                }
                if let Some(v) = args.get(arg_idx) {
                    if is_debug {
                        result.push_str(&v.to_debug_string());
                    } else if let Some(rendered) = display(v) {
                        result.push_str(&rendered);
                    } else {
                        result.push_str(&v.to_string());
                    }
                }
                arg_idx += 1;
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                result.push('}');
            }
            _ => result.push(c),
        }
    }
    result
}
