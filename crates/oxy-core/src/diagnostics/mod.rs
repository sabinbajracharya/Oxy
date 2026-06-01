//! Structured diagnostics model for lexer/parser/type/runtime errors.
//!
//! The compiler/runtime can keep using `PipelineError` for control-flow and
//! API compatibility while progressively migrating to this richer model.

use crate::lexer::Span;

pub mod codes;

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Note,
}

/// Diagnostic origin category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCategory {
    Lexer,
    Parser,
    TypeChecker,
    Runtime,
    Other,
}

/// Label role inside a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelKind {
    Primary,
    Secondary,
}

/// One highlighted source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub kind: LabelKind,
    pub span: Span,
    pub message: Option<String>,
}

impl Label {
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self {
            kind: LabelKind::Primary,
            span,
            message: Some(message.into()),
        }
    }

    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self {
            kind: LabelKind::Secondary,
            span,
            message: Some(message.into()),
        }
    }
}

/// Additional context line attached to a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    Note,
    Help,
}

/// Note/help message attached to a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub kind: NoteKind,
    pub message: String,
}

/// One concrete text replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixItEdit {
    pub span: Span,
    pub replacement: String,
}

/// Suggested fix attached to a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixIt {
    pub message: String,
    pub edits: Vec<FixItEdit>,
}

/// First-class compiler/runtime diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: DiagnosticSeverity,
    pub category: DiagnosticCategory,
    pub message: String,
    pub labels: Vec<Label>,
    pub notes: Vec<Note>,
    pub fix_its: Vec<FixIt>,
}

impl Diagnostic {
    pub fn new(
        code: &'static str,
        severity: DiagnosticSeverity,
        category: DiagnosticCategory,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity,
            category,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            fix_its: Vec::new(),
        }
    }

    pub fn error(
        code: &'static str,
        category: DiagnosticCategory,
        message: impl Into<String>,
    ) -> Self {
        Self::new(code, DiagnosticSeverity::Error, category, message)
    }

    pub fn with_primary_span(mut self, span: Span) -> Self {
        self.labels.push(Label {
            kind: LabelKind::Primary,
            span,
            message: None,
        });
        self
    }

    pub fn with_primary_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label::primary(span, message));
        self
    }

    pub fn with_secondary_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label::secondary(span, message));
        self
    }

    pub fn with_note(mut self, message: impl Into<String>) -> Self {
        self.notes.push(Note {
            kind: NoteKind::Note,
            message: message.into(),
        });
        self
    }

    pub fn with_help(mut self, message: impl Into<String>) -> Self {
        self.notes.push(Note {
            kind: NoteKind::Help,
            message: message.into(),
        });
        self
    }

    pub fn with_fix_it(mut self, message: impl Into<String>, edits: Vec<FixItEdit>) -> Self {
        self.fix_its.push(FixIt {
            message: message.into(),
            edits,
        });
        self
    }

    pub fn primary_label(&self) -> Option<&Label> {
        self.labels.iter().find(|l| l.kind == LabelKind::Primary)
    }

    pub fn primary_span(&self) -> Option<Span> {
        self.primary_label().map(|l| l.span)
    }

    pub fn line_column(&self) -> Option<(usize, usize)> {
        self.primary_span().map(|span| (span.line, span.column))
    }
}

/// Build a single-column span from 1-based line/column coordinates.
pub fn span_from_line_column(line: usize, column: usize) -> Span {
    let start = column.saturating_sub(1);
    Span::new(start, start.saturating_add(1), line, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_builder_keeps_primary_label() {
        let d = Diagnostic::error(codes::TYP_MISMATCH, DiagnosticCategory::TypeChecker, "bad")
            .with_primary_label(Span::new(10, 13, 2, 4), "expected Int")
            .with_secondary_label(Span::new(30, 33, 6, 2), "found String")
            .with_help("consider a cast");

        assert_eq!(d.primary_span(), Some(Span::new(10, 13, 2, 4)));
        assert_eq!(d.line_column(), Some((2, 4)));
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.labels.len(), 2);
    }
}
