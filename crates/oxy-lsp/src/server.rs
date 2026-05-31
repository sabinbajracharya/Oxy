//! LSP server: the [`OxyLsp`] backend, tower-lsp protocol handlers, and
//! the `main` entry point. This module is the glue — diagnostics and helper
//! functions live in the crate root, and completion/hover data lives in
//! [`completions`].

use std::collections::HashMap;
use std::sync::Mutex;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::completions::*;
use crate::{byte_offset_to_position, error_to_diagnostic, is_ident_char, item_to_symbol};

pub(crate) struct OxyLsp {
    client: Client,
    documents: Mutex<HashMap<Url, String>>,
}

impl OxyLsp {
    pub(crate) fn update_and_diagnose(&self, uri: Url, source: String) {
        let diagnostics = Self::diagnose(&source);
        self.documents.lock().unwrap().insert(uri.clone(), source);
        let client = self.client.clone();
        tokio::spawn(async move {
            client.publish_diagnostics(uri, diagnostics, None).await;
        });
    }

    pub(crate) fn diagnose(source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Try lexing first
        if let Err(e) = oxy_core::lexer::tokenize(source) {
            diagnostics.push(error_to_diagnostic(&e));
            return diagnostics;
        }

        // Then try parsing
        let program = match oxy_core::parser::parse(source) {
            Ok(p) => p,
            Err(e) => {
                diagnostics.push(error_to_diagnostic(&e));
                return diagnostics;
            }
        };

        // Run type checking
        if let Err(e) = oxy_core::type_checker::TypeChecker::new().check_program(&program) {
            diagnostics.push(error_to_diagnostic(&e));
        }

        diagnostics
    }

    fn get_document(&self, uri: &Url) -> Option<String> {
        self.documents.lock().unwrap().get(uri).cloned()
    }

    pub(crate) fn try_parse(source: &str) -> Option<oxy_core::ast::Program> {
        oxy_core::parser::parse(source).ok()
    }

    /// Extract the word at a given position from source text.
    pub(crate) fn word_at_position(source: &str, position: Position) -> Option<String> {
        let line = source.lines().nth(position.line as usize)?;
        let col = position.character as usize;
        if col > line.len() {
            return None;
        }

        let bytes = line.as_bytes();
        let mut start = col;
        let mut end = col;

        while start > 0 && is_ident_char(bytes[start - 1]) {
            start -= 1;
        }
        while end < bytes.len() && is_ident_char(bytes[end]) {
            end += 1;
        }

        if start == end {
            return None;
        }

        // No special handling for `!` — Oxy uses regular function calls.

        Some(line[start..end].to_string())
    }
}

// ---------------------------------------------------------------------------
// LanguageServer impl
// ---------------------------------------------------------------------------

#[tower_lsp::async_trait]
impl LanguageServer for OxyLsp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions::default()),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Oxy LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.update_and_diagnose(uri, text);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            self.update_and_diagnose(uri, change.text);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.lock().unwrap().remove(&uri);
        // Clear diagnostics on close
        let client = self.client.clone();
        tokio::spawn(async move {
            client.publish_diagnostics(uri, vec![], None).await;
        });
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let source = match self.get_document(uri) {
            Some(s) => s,
            None => {
                let mut items = Vec::new();
                items.extend(keyword_completions());
                items.extend(type_completions());
                items.extend(builtin_function_completions());
                items.extend(module_completions());
                items.extend(snippet_completions());
                return Ok(Some(CompletionResponse::Array(items)));
            }
        };

        // Check if cursor is after a dot — suggest methods (type-aware + builtins)
        if is_after_dot(&source, pos) {
            let mut items = method_completions();
            // Try to infer receiver type and add user-defined methods
            if let Some(program) = Self::try_parse(&source) {
                if let Some(type_name) = try_infer_receiver_type(&source, pos) {
                    items.extend(find_methods_for_type(&program, &type_name));
                }
            }
            return Ok(Some(CompletionResponse::Array(items)));
        }

        // Check if cursor is after :: — suggest module/type members
        if is_after_colon_colon(&source, pos) {
            if let Some(program) = Self::try_parse(&source) {
                let prefix = extract_prefix_before_colon_colon(&source, pos);
                let items = module_member_completions(&program, &prefix);
                if !items.is_empty() {
                    return Ok(Some(CompletionResponse::Array(items)));
                }
            }
        }

        let mut items = Vec::new();
        items.extend(keyword_completions());
        items.extend(type_completions());
        items.extend(builtin_function_completions());
        items.extend(module_completions());
        items.extend(snippet_completions());
        // Add user-defined items from AST
        if let Some(program) = Self::try_parse(&source) {
            items.extend(user_defined_completions(&program));
        }
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let source = match self.get_document(uri) {
            Some(s) => s,
            None => return Ok(None),
        };

        let word = match Self::word_at_position(&source, pos) {
            Some(w) => w,
            None => return Ok(None),
        };

        let make_hover = |value: String| -> Result<Option<Hover>> {
            Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: None,
            }))
        };

        // Check keywords
        if let Some(desc) = keyword_hover(&word) {
            return make_hover(desc);
        }

        // Check built-in types/functions
        if let Some(desc) = builtin_hover(&word) {
            return make_hover(desc);
        }

        // Check user-defined items (search all items including nested modules)
        if let Some(program) = Self::try_parse(&source) {
            if let Some(desc) = find_item_hover(&program.items, &word) {
                return make_hover(desc);
            }
        }

        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;
        let source = match self.get_document(uri) {
            Some(s) => s,
            None => return Ok(None),
        };

        let program = match Self::try_parse(&source) {
            Some(p) => p,
            None => return Ok(None),
        };

        let symbols: Vec<DocumentSymbol> = program
            .items
            .iter()
            .filter_map(|item| item_to_symbol(item, &source))
            .collect();

        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let source = match self.get_document(uri) {
            Some(s) => s,
            None => return Ok(None),
        };

        let word = match Self::word_at_position(&source, pos) {
            Some(w) => w,
            None => return Ok(None),
        };

        let program = match Self::try_parse(&source) {
            Some(p) => p,
            None => return Ok(None),
        };

        // Resolve through use imports first
        let resolved = resolve_use_import(&program.items, &word);

        // Search all items (including nested in modules) for the definition
        if let Some(span) = find_definition_span(&program.items, &resolved) {
            let line0 = if span.line > 0 { span.line - 1 } else { 0 } as u32;
            let col0 = if span.column > 0 { span.column - 1 } else { 0 } as u32;
            let start = Position::new(line0, col0);
            let end_pos = byte_offset_to_position(&source, span.end);
            let loc = Location::new(uri.clone(), Range::new(start, end_pos));
            return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
        }

        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub(crate) async fn serve() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| OxyLsp {
        client,
        documents: Mutex::new(HashMap::new()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
