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
            Pattern::Wildcard(_) | Pattern::Rest(_) => true,
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
            Pattern::Tuple(pats, _) => {
                if let Value::Tuple(vals) = value {
                    Self::match_with_rest(pats, vals)
                } else {
                    false
                }
            }
            Pattern::Slice(pats, _) => {
                if let Value::Vec(rc) = value {
                    Self::match_with_rest(pats, &rc.borrow())
                } else {
                    false
                }
            }
            Pattern::Range {
                start,
                end,
                inclusive,
                ..
            } => match value {
                Value::Integer(n) => {
                    let n = *n;
                    let below_start = if let Some(s) = start { n < *s } else { false };
                    let above_end = if let Some(e) = end {
                        if *inclusive {
                            n > *e
                        } else {
                            n >= *e
                        }
                    } else {
                        false
                    };
                    !below_start && !above_end
                }
                Value::Char(c) => {
                    let c = *c as i64;
                    let below_start = if let Some(s) = start { c < *s } else { false };
                    let above_end = if let Some(e) = end {
                        if *inclusive {
                            c > *e
                        } else {
                            c >= *e
                        }
                    } else {
                        false
                    };
                    !below_start && !above_end
                }
                _ => false,
            },
            Pattern::Or(alternatives, _) => {
                alternatives.iter().any(|p| Self::pattern_matches(p, value))
            }
        }
    }

    /// Match patterns against values, supporting `..` rest patterns.
    fn match_with_rest(pats: &[Pattern], vals: &[Value]) -> bool {
        let has_rest = pats.iter().any(|p| matches!(p, Pattern::Rest(_)));
        if has_rest {
            let rest_pos = pats
                .iter()
                .position(|p| matches!(p, Pattern::Rest(_)))
                .unwrap();
            let before = &pats[..rest_pos];
            let after = &pats[rest_pos + 1..];
            if vals.len() < before.len() + after.len() {
                return false;
            }
            before
                .iter()
                .zip(vals.iter())
                .all(|(p, v)| Self::pattern_matches(p, v))
                && after
                    .iter()
                    .zip(vals[vals.len() - after.len()..].iter())
                    .all(|(p, v)| Self::pattern_matches(p, v))
        } else {
            pats.len() == vals.len()
                && pats
                    .iter()
                    .zip(vals.iter())
                    .all(|(p, v)| Self::pattern_matches(p, v))
        }
    }

    /// Convert a value to an iterable list of values (for `for` loops).
    ///
    /// Delegates to [`Value::into_iterable`]; for Iterator values, drains remaining elements.
    pub(crate) fn value_to_iter(
        &mut self,
        value: &Value,
        span: Span,
    ) -> Result<Vec<Value>, FerriError> {
        match value {
            Value::Iterator(iter) => Ok(self.collect_remaining((**iter).clone())),
            other => other
                .clone()
                .into_iterable()
                .map_err(|msg| FerriError::Runtime {
                    message: msg,
                    line: span.line,
                    column: span.column,
                }),
        }
    }

    /// Bind pattern variables to matched values in the given environment.
    ///
    /// Called after `pattern_matches` returns true to actually define
    /// the captured variables (e.g. `Some(x)` binds `x` to the inner value).
    pub(crate) fn bind_pattern(pattern: &Pattern, value: &Value, env: &Env, mutable: bool) {
        match pattern {
            Pattern::Ident(name, _) => {
                env.borrow_mut()
                    .define(name.clone(), value.clone(), mutable);
            }
            Pattern::EnumVariant { fields, .. } => {
                if let Value::EnumVariant { data, .. } = value {
                    for (pat, val) in fields.iter().zip(data.iter()) {
                        Self::bind_pattern(pat, val, env, mutable);
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
                            Self::bind_pattern(pat, val, env, mutable);
                        }
                    }
                }
            }
            Pattern::Tuple(pats, _) => {
                if let Value::Tuple(vals) = value {
                    Self::bind_with_rest(pats, vals, env, mutable);
                }
            }
            Pattern::Slice(pats, _) => {
                if let Value::Vec(rc) = value {
                    Self::bind_with_rest(pats, &rc.borrow(), env, mutable);
                }
            }
            Pattern::Or(alternatives, _) => {
                // Bind using the first matching alternative
                for alt in alternatives {
                    if Self::pattern_matches(alt, value) {
                        Self::bind_pattern(alt, value, env, mutable);
                        break;
                    }
                }
            }
            Pattern::Wildcard(_)
            | Pattern::Literal(_)
            | Pattern::Rest(_)
            | Pattern::Range { .. } => {}
        }
    }

    /// Bind patterns against values, supporting `..` rest patterns.
    fn bind_with_rest(pats: &[Pattern], vals: &[Value], env: &Env, mutable: bool) {
        let has_rest = pats.iter().any(|p| matches!(p, Pattern::Rest(_)));
        if has_rest {
            let rest_pos = pats
                .iter()
                .position(|p| matches!(p, Pattern::Rest(_)))
                .unwrap();
            let before = &pats[..rest_pos];
            let after = &pats[rest_pos + 1..];
            for (pat, val) in before.iter().zip(vals.iter()) {
                Self::bind_pattern(pat, val, env, mutable);
            }
            for (pat, val) in after.iter().zip(vals[vals.len() - after.len()..].iter()) {
                Self::bind_pattern(pat, val, env, mutable);
            }
        } else {
            for (pat, val) in pats.iter().zip(vals.iter()) {
                Self::bind_pattern(pat, val, env, mutable);
            }
        }
    }
}
