//! Oxy Language Server Protocol implementation.
//!
//! The LSP provides diagnostics, completions, hover, go-to-definition, and
//! document symbols. Modules:
//! - [`server`]: the tower-lsp backend and entry point.
//! - [`completions`]: completion, hover, and go-to-def data helpers.

mod completions;
mod server;

use oxy_core::ast::Item;
use oxy_core::diagnostics::{DiagnosticSeverity as OxyDiagnosticSeverity, LabelKind};
use oxy_core::errors::PipelineError;
use tower_lsp::lsp_types::*;

pub(crate) fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

pub(crate) fn error_to_diagnostic(e: &PipelineError) -> Diagnostic {
    let d = e.to_diagnostic();
    let primary = d.primary_label();

    // Oxy spans are 1-indexed; LSP is 0-indexed.
    let (start, end) = if let Some(label) = primary {
        let line0 = label.span.line.saturating_sub(1) as u32;
        let col0 = label.span.column.saturating_sub(1) as u32;
        let width = (label.span.end.saturating_sub(label.span.start)).max(1) as u32;
        (
            Position::new(line0, col0),
            Position::new(line0, col0.saturating_add(width)),
        )
    } else {
        let (line, column) = d.line_column().unwrap_or((1, 1));
        let line0 = line.saturating_sub(1) as u32;
        let col0 = column.saturating_sub(1) as u32;
        let pos = Position::new(line0, col0);
        (pos, pos)
    };

    let related_information: Vec<DiagnosticRelatedInformation> = d
        .labels
        .iter()
        .filter(|l| l.kind == LabelKind::Secondary)
        .map(|label| {
            let line0 = label.span.line.saturating_sub(1) as u32;
            let col0 = label.span.column.saturating_sub(1) as u32;
            let width = (label.span.end.saturating_sub(label.span.start)).max(1) as u32;
            let start = Position::new(line0, col0);
            let end = Position::new(line0, col0.saturating_add(width));
            DiagnosticRelatedInformation {
                location: Location {
                    uri: Url::parse("file://unknown").expect("valid synthetic URI"),
                    range: Range::new(start, end),
                },
                message: label
                    .message
                    .clone()
                    .unwrap_or_else(|| "related location".to_string()),
            }
        })
        .collect();

    Diagnostic {
        range: Range::new(start, end),
        severity: Some(match d.severity {
            OxyDiagnosticSeverity::Error => tower_lsp::lsp_types::DiagnosticSeverity::ERROR,
            OxyDiagnosticSeverity::Warning => tower_lsp::lsp_types::DiagnosticSeverity::WARNING,
            OxyDiagnosticSeverity::Note => tower_lsp::lsp_types::DiagnosticSeverity::INFORMATION,
        }),
        code: Some(NumberOrString::String(d.code.to_string())),
        source: Some("oxy".to_string()),
        message: d.message,
        related_information: if related_information.is_empty() {
            None
        } else {
            Some(related_information)
        },
        ..Default::default()
    }
}

pub(crate) fn item_to_symbol(item: &Item, source: &str) -> Option<DocumentSymbol> {
    let (name, kind, span) = match item {
        Item::Function(f) => (f.name.clone(), SymbolKind::FUNCTION, f.span),
        Item::Struct(s) => (s.name.clone(), SymbolKind::STRUCT, s.span),
        Item::Enum(e) => (e.name.clone(), SymbolKind::ENUM, e.span),
        Item::Trait(t) => (t.name.clone(), SymbolKind::INTERFACE, t.span),
        Item::Module(m) => (m.name.clone(), SymbolKind::MODULE, m.span),
        Item::Impl(i) => (format!("impl {}", i.type_name), SymbolKind::CLASS, i.span),
        Item::ImplTrait(i) => (
            format!("impl {} for {}", i.trait_name, i.type_name),
            SymbolKind::CLASS,
            i.span,
        ),
        Item::Const { name, span, .. } => (name.clone(), SymbolKind::CONSTANT, *span),
        Item::TypeAlias { name, span, .. } => (name.clone(), SymbolKind::TYPE_PARAMETER, *span),
        Item::Use(_) => return None,
    };

    let line0 = if span.line > 0 { span.line - 1 } else { 0 } as u32;
    let col0 = if span.column > 0 { span.column - 1 } else { 0 } as u32;
    let start = Position::new(line0, col0);

    // Compute end position from byte offsets
    let end = byte_offset_to_position(source, span.end);

    let range = Range::new(start, end);

    #[allow(deprecated)] // DocumentSymbol::new requires deprecated `deprecated` field
    Some(DocumentSymbol {
        name,
        detail: None,
        kind,
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: None,
    })
}

pub(crate) fn byte_offset_to_position(source: &str, offset: usize) -> Position {
    let offset = offset.min(source.len());
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    Position::new(line, col)
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    server::serve().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completions::*;
    use crate::server::OxyLsp;

    #[test]
    fn test_error_to_diagnostic() {
        let err = PipelineError::Parser {
            message: "unexpected token".to_string(),
            line: 3,
            column: 5,
        };
        let diag = error_to_diagnostic(&err);
        assert_eq!(diag.range.start.line, 2); // 0-indexed
        assert_eq!(diag.range.start.character, 4);
        assert_eq!(
            diag.severity,
            Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR)
        );
        assert_eq!(
            diag.code,
            Some(NumberOrString::String(
                oxy_core::diagnostics::codes::PAR_UNEXPECTED_TOKEN.to_string()
            ))
        );
        assert_eq!(diag.message, "unexpected token");
    }

    #[test]
    fn test_word_at_position() {
        let source = "fn hello(x: Int) {}";
        let word = OxyLsp::word_at_position(source, Position::new(0, 3));
        assert_eq!(word, Some("hello".to_string()));

        let word = OxyLsp::word_at_position(source, Position::new(0, 0));
        assert_eq!(word, Some("fn".to_string()));

        let word = OxyLsp::word_at_position(source, Position::new(0, 12));
        assert_eq!(word, Some("Int".to_string()));
    }

    #[test]
    fn test_keyword_hover() {
        assert!(keyword_hover("fn").is_some());
        assert!(keyword_hover("val").is_some());
        assert!(keyword_hover("notakeyword").is_none());
    }

    #[test]
    fn test_builtin_hover() {
        assert!(builtin_hover("Int").is_some());
        assert!(builtin_hover("io::println").is_some());
        assert!(builtin_hover("println").is_some());
        assert!(builtin_hover("string::format").is_some());
        assert!(builtin_hover("format").is_some());
        assert!(builtin_hover("unknown").is_none());
    }

    #[test]
    fn test_diagnose_valid_source() {
        let diagnostics = OxyLsp::diagnose("fn main() {}");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_diagnose_invalid_source() {
        let diagnostics = OxyLsp::diagnose("fn {");
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_document_symbols() {
        let source = "fn foo() {} struct Bar {} enum Baz { A, B }";
        let program = OxyLsp::try_parse(source).unwrap();
        let symbols: Vec<_> = program
            .items
            .iter()
            .filter_map(|item| item_to_symbol(item, source))
            .collect();
        assert_eq!(symbols.len(), 3);
        assert_eq!(symbols[0].name, "foo");
        assert_eq!(symbols[0].kind, SymbolKind::FUNCTION);
        assert_eq!(symbols[1].name, "Bar");
        assert_eq!(symbols[1].kind, SymbolKind::STRUCT);
        assert_eq!(symbols[2].name, "Baz");
        assert_eq!(symbols[2].kind, SymbolKind::ENUM);
    }

    #[test]
    fn test_byte_offset_to_position() {
        let source = "line1\nline2\nline3";
        let pos = byte_offset_to_position(source, 6);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);

        let pos = byte_offset_to_position(source, 8);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 2);
    }

    #[test]
    fn test_completion_lists_not_empty() {
        assert!(!keyword_completions().is_empty());
        assert!(!type_completions().is_empty());
        assert!(!builtin_function_completions().is_empty());
        assert!(!module_completions().is_empty());
        assert!(!snippet_completions().is_empty());
    }
}
