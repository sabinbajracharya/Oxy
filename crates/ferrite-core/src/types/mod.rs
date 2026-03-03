//! Value system for the Ferrite language.
//!
//! All values at runtime are represented by the [`Value`] enum.
//! Ferrite uses reference counting internally — no borrow checker.

use std::collections::HashMap;
use std::fmt;

use crate::ast::{Block, Param, TypeAnnotation};
use crate::env::Env;

/// A runtime value in Ferrite.
#[derive(Debug, Clone)]
pub enum Value {
    /// 64-bit signed integer.
    Integer(i64),
    /// 64-bit floating-point number.
    Float(f64),
    /// Boolean.
    Bool(bool),
    /// UTF-8 string.
    String(String),
    /// Character.
    Char(char),
    /// Unit value `()`.
    Unit,
    /// A function value (closure).
    Function(Box<FunctionData>),
    /// A range value: `start..end` (end-exclusive, stored as actual end).
    Range(i64, i64),
    /// A vector (dynamic array).
    Vec(Vec<Value>),
    /// A tuple.
    Tuple(Vec<Value>),
    /// A struct instance: `Point { x: 1.0, y: 2.0 }`
    Struct {
        name: String,
        fields: HashMap<String, Value>,
    },
    /// An enum variant instance.
    EnumVariant {
        enum_name: String,
        variant: String,
        data: Vec<Value>,
    },
    /// A hash map (string keys for simplicity).
    HashMap(HashMap<String, Value>),
    /// A future (lazy thunk wrapping an async function call).
    Future(Box<FutureData>),
    /// A join handle (eagerly evaluated, wraps a result).
    JoinHandle(Box<Value>),
}

/// Data for an async future (boxed to keep Value enum small).
#[derive(Debug, Clone)]
pub struct FutureData {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub closure_env: Env,
    pub args: Vec<Value>,
}

/// Data for a function value (boxed to keep Value enum small).
#[derive(Debug, Clone)]
pub struct FunctionData {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub closure_env: Env,
}

impl Value {
    /// Returns the type name of this value for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Integer(_) => "i64",
            Value::Float(_) => "f64",
            Value::Bool(_) => "bool",
            Value::String(_) => "String",
            Value::Char(_) => "char",
            Value::Unit => "()",
            Value::Function(_) => "fn",
            Value::Range(_, _) => "Range",
            Value::Vec(_) => "Vec",
            Value::Tuple(_) => "tuple",
            Value::Struct { name, .. } => {
                // Return a static str for common names, else fallback
                // We leak the string since type_name returns &'static str
                // This is fine — struct names live for the program lifetime
                Box::leak(name.clone().into_boxed_str())
            }
            Value::EnumVariant { enum_name, .. } => Box::leak(enum_name.clone().into_boxed_str()),
            Value::HashMap(_) => "HashMap",
            Value::Future(_) => "Future",
            Value::JoinHandle(_) => "JoinHandle",
        }
    }

    /// Returns true if this value is truthy (for conditions).
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Integer(n) => *n != 0,
            Value::Unit => false,
            Value::Range(_, _) => true,
            Value::Vec(v) => !v.is_empty(),
            Value::Tuple(t) => !t.is_empty(),
            Value::Struct { .. } => true,
            Value::EnumVariant { .. } => true,
            Value::HashMap(m) => !m.is_empty(),
            Value::Future(_) => true,
            Value::JoinHandle(_) => true,
            _ => true,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Integer(n) => write!(f, "{n}"),
            Value::Float(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{n:.1}")
                } else {
                    write!(f, "{n}")
                }
            }
            Value::Bool(b) => write!(f, "{b}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Char(c) => write!(f, "{c}"),
            Value::Unit => write!(f, "()"),
            Value::Function(func) => write!(f, "<fn {}>", func.name),
            Value::Range(start, end) => write!(f, "{start}..{end}"),
            Value::Vec(v) => {
                write!(f, "[")?;
                for (i, elem) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, "]")
            }
            Value::Tuple(t) => {
                write!(f, "(")?;
                for (i, elem) in t.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{elem}")?;
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
                    write!(f, "{k}: {v}")?;
                }
                write!(f, " }}")
            }
            Value::EnumVariant {
                enum_name,
                variant,
                data,
            } => {
                // Built-in Option/Result: show without enum prefix
                if enum_name == "Option" || enum_name == "Result" {
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
                        write!(f, "{v}")?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Value::HashMap(m) => {
                write!(f, "{{")?;
                let mut sorted: Vec<_> = m.iter().collect();
                sorted.sort_by_key(|(k, _)| (*k).clone());
                for (i, (k, v)) in sorted.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
            Value::Future(_) => write!(f, "<future>"),
            Value::JoinHandle(_) => write!(f, "<join_handle>"),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Unit, Value::Unit) => true,
            (Value::Range(a1, a2), Value::Range(b1, b2)) => a1 == b1 && a2 == b2,
            (Value::Vec(a), Value::Vec(b)) => a == b,
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (
                Value::Struct {
                    name: na,
                    fields: fa,
                },
                Value::Struct {
                    name: nb,
                    fields: fb,
                },
            ) => na == nb && fa == fb,
            (
                Value::EnumVariant {
                    enum_name: ea,
                    variant: va,
                    data: da,
                },
                Value::EnumVariant {
                    enum_name: eb,
                    variant: vb,
                    data: db,
                },
            ) => ea == eb && va == vb && da == db,
            (Value::HashMap(a), Value::HashMap(b)) => a == b,
            _ => false,
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => a.partial_cmp(b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
            (Value::String(a), Value::String(b)) => a.partial_cmp(b),
            (Value::Char(a), Value::Char(b)) => a.partial_cmp(b),
            (Value::Bool(a), Value::Bool(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Value::Integer(42)), "42");
        assert_eq!(format!("{}", Value::Float(3.5)), "3.5");
        assert_eq!(format!("{}", Value::Float(1.0)), "1.0");
        assert_eq!(format!("{}", Value::Bool(true)), "true");
        assert_eq!(format!("{}", Value::String("hello".into())), "hello");
        assert_eq!(format!("{}", Value::Char('x')), "x");
        assert_eq!(format!("{}", Value::Unit), "()");
    }

    #[test]
    fn test_type_name() {
        assert_eq!(Value::Integer(0).type_name(), "i64");
        assert_eq!(Value::String("".into()).type_name(), "String");
        assert_eq!(Value::Unit.type_name(), "()");
    }

    #[test]
    fn test_is_truthy() {
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::Integer(1).is_truthy());
        assert!(!Value::Integer(0).is_truthy());
        assert!(!Value::Unit.is_truthy());
        assert!(Value::String("".into()).is_truthy());
    }

    #[test]
    fn test_equality() {
        assert_eq!(Value::Integer(42), Value::Integer(42));
        assert_ne!(Value::Integer(1), Value::Integer(2));
        assert_ne!(Value::Integer(1), Value::Bool(true));
        assert_eq!(Value::String("a".into()), Value::String("a".into()));
    }

    #[test]
    fn test_ordering() {
        assert!(Value::Integer(1) < Value::Integer(2));
        assert!(Value::String("a".into()) < Value::String("b".into()));
        assert_eq!(
            Value::Integer(1).partial_cmp(&Value::String("a".into())),
            None
        );
    }
}
