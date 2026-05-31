//! Pattern types for match arms and destructuring let.
//!
//! Extracted from [`super`] to keep mod.rs under ~300 lines.

use super::expr::Expr;
use crate::lexer::Span;

/// A pattern for match arms and let destructuring.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Literal value: `42`, `"hello"`, `true`
    Literal(Expr),
    /// Wildcard: `_`
    Wildcard(Span),
    /// Variable binding: `x`
    Ident(String, Span),
    /// Enum variant pattern: `Shape::Circle(r)` or `Option::Some(x)`
    EnumVariant {
        enum_name: String,
        variant: String,
        fields: Vec<Pattern>,
        span: Span,
    },
    /// Struct pattern: `Point { x, y }`
    Struct {
        name: String,
        fields: Vec<(String, Pattern)>,
        span: Span,
    },
    /// Tuple pattern: `(x, y, z)`
    Tuple(Vec<Pattern>, Span),
    /// Or-pattern: `A | B | C`
    Or(Vec<Pattern>, Span),
    /// Slice pattern: `[a, b, ..]`
    Slice(Vec<Pattern>, Span),
    /// Rest pattern: `..` (used inside slice/tuple patterns)
    Rest(Span),
    /// Range pattern: `start..end`, `start..=end`, `..end`, `start..`
    Range {
        start: Option<i64>,
        end: Option<i64>,
        inclusive: bool,
        span: Span,
    },
}

impl Pattern {
    /// Returns the source span of this pattern.
    pub fn span(&self) -> Span {
        match self {
            Pattern::Literal(e) => e.span(),
            Pattern::Wildcard(s) | Pattern::Ident(_, s) | Pattern::Rest(s) => *s,
            Pattern::EnumVariant { span, .. }
            | Pattern::Struct { span, .. }
            | Pattern::Tuple(_, span)
            | Pattern::Or(_, span)
            | Pattern::Slice(_, span)
            | Pattern::Range { span, .. } => *span,
        }
    }

    pub(crate) fn pretty_print(&self, out: &mut String) {
        match self {
            Pattern::Literal(e) => e.pretty_print(out, 0),
            Pattern::Wildcard(_) => out.push('_'),
            Pattern::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                if let Some(s) = start {
                    out.push_str(&s.to_string());
                }
                out.push_str("..");
                if *inclusive {
                    out.push('=');
                }
                if let Some(e) = end {
                    out.push_str(&e.to_string());
                }
            }
            Pattern::Ident(name, _) => out.push_str(name),
            Pattern::EnumVariant {
                enum_name,
                variant,
                fields,
                ..
            } => {
                out.push_str(&format!("{enum_name}::{variant}"));
                if !fields.is_empty() {
                    out.push('(');
                    for (i, f) in fields.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        f.pretty_print(out);
                    }
                    out.push(')');
                }
            }
            Pattern::Struct { name, fields, .. } => {
                out.push_str(name);
                out.push_str(" { ");
                for (i, (fname, pat)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(fname);
                    out.push_str(": ");
                    pat.pretty_print(out);
                }
                out.push_str(" }");
            }
            Pattern::Tuple(pats, _) => {
                out.push('(');
                for (i, p) in pats.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    p.pretty_print(out);
                }
                out.push(')');
            }
            Pattern::Or(pats, _) => {
                for (i, p) in pats.iter().enumerate() {
                    if i > 0 {
                        out.push_str(" | ");
                    }
                    p.pretty_print(out);
                }
            }
            Pattern::Slice(pats, _) => {
                out.push('[');
                for (i, p) in pats.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    p.pretty_print(out);
                }
                out.push(']');
            }
            Pattern::Rest(_) => {
                out.push_str("..");
            }
        }
    }
}
