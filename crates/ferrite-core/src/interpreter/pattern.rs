//! Pattern matching and binding for `match` expressions and `for` loops.
//!
//! Handles matching values against patterns (literals, enum variants,
//! struct patterns, wildcards) and binding matched values to variables.

use crate::ast::*;
use crate::env::Env;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

use super::Interpreter;

impl Interpreter {
    /// Check if a pattern matches a value (without binding).
    ///
    /// Used in `match` arms to find the first matching pattern.
    pub(crate) fn pattern_matches(pattern: &Pattern, value: &Value) -> bool {
        match pattern {
            Pattern::Wildcard(_) => true,
            Pattern::Ident(_, _) => true, // Variable pattern always matches
            Pattern::Literal(expr) => match (expr, value) {
                (Expr::IntLiteral(n, _), Value::Integer(v)) => *n == *v,
                (Expr::FloatLiteral(n, _), Value::Float(v)) => *n == *v,
                (Expr::BoolLiteral(b, _), Value::Bool(v)) => *b == *v,
                (Expr::StringLiteral(s, _), Value::String(v)) => s == v,
                (Expr::CharLiteral(c, _), Value::Char(v)) => *c == *v,
                (
                    Expr::UnaryOp {
                        op: UnaryOp::Neg,
                        expr,
                        ..
                    },
                    Value::Integer(v),
                ) => {
                    if let Expr::IntLiteral(n, _) = expr.as_ref() {
                        -*n == *v
                    } else {
                        false
                    }
                }
                _ => false,
            },
            Pattern::EnumVariant {
                enum_name,
                variant,
                fields,
                ..
            } => {
                if let Value::EnumVariant {
                    enum_name: en,
                    variant: vn,
                    data,
                } = value
                {
                    en == enum_name
                        && vn == variant
                        && data.len() == fields.len()
                        && fields
                            .iter()
                            .zip(data.iter())
                            .all(|(pat, val)| Self::pattern_matches(pat, val))
                } else {
                    false
                }
            }
            Pattern::Struct { name, fields, .. } => {
                if let Value::Struct {
                    name: sn,
                    fields: sf,
                } = value
                {
                    sn == name
                        && fields.iter().all(|(fname, pat)| {
                            sf.get(fname).is_some_and(|v| Self::pattern_matches(pat, v))
                        })
                } else {
                    false
                }
            }
        }
    }

    /// Convert a value to an iterable list of values (for `for` loops).
    ///
    /// Supports ranges, vectors, strings (char iteration), and
    /// HashMaps (yields `(key, value)` tuples sorted by key).
    pub(crate) fn value_to_iter(
        &self,
        value: &Value,
        span: Span,
    ) -> Result<Vec<Value>, FerriError> {
        match value {
            Value::Range(start, end) => Ok((*start..*end).map(Value::Integer).collect()),
            Value::Vec(v) => Ok(v.clone()),
            Value::String(s) => Ok(s.chars().map(Value::Char).collect()),
            Value::HashMap(m) => {
                // Iterate as (key, value) tuples, sorted by key for determinism
                let mut pairs: Vec<_> = m
                    .iter()
                    .map(|(k, v)| Value::Tuple(vec![Value::String(k.clone()), v.clone()]))
                    .collect();
                pairs.sort_by(|a, b| {
                    if let (Value::Tuple(a), Value::Tuple(b)) = (a, b) {
                        a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal)
                    } else {
                        std::cmp::Ordering::Equal
                    }
                });
                Ok(pairs)
            }
            _ => Err(FerriError::Runtime {
                message: format!("cannot iterate over {}", value.type_name()),
                line: span.line,
                column: span.column,
            }),
        }
    }

    /// Bind pattern variables to matched values in the given environment.
    ///
    /// Called after `pattern_matches` returns true to actually define
    /// the captured variables (e.g. `Some(x)` binds `x` to the inner value).
    pub(crate) fn bind_pattern(pattern: &Pattern, value: &Value, env: &Env) {
        match pattern {
            Pattern::Ident(name, _) => {
                env.borrow_mut().define(name.clone(), value.clone(), false);
            }
            Pattern::EnumVariant { fields, .. } => {
                if let Value::EnumVariant { data, .. } = value {
                    for (pat, val) in fields.iter().zip(data.iter()) {
                        Self::bind_pattern(pat, val, env);
                    }
                }
            }
            Pattern::Struct { fields, .. } => {
                if let Value::Struct {
                    fields: sfields, ..
                } = value
                {
                    for (fname, pat) in fields {
                        if let Some(val) = sfields.get(fname) {
                            Self::bind_pattern(pat, val, env);
                        }
                    }
                }
            }
            Pattern::Wildcard(_) | Pattern::Literal(_) => {}
        }
    }
}
