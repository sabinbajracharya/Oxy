# Oxy's LSP: A Walkthrough

<!-- OPUS_FILL
1-paragraph intro. "Open crates/oxy-lsp/src/main.rs. It's about 1200 lines.
The key thing to understand is how tower-lsp makes the protocol mechanics disappear —
you implement trait methods, the library handles the JSON-RPC plumbing."
-->

**File:** `crates/oxy-lsp/src/main.rs`

---

## `tower-lsp`: the framework

Oxy's LSP uses `tower-lsp` — a Rust library that implements the LSP transport layer.
Instead of parsing JSON-RPC manually, you implement the `LanguageServer` trait:

```rust
#[tower_lsp::async_trait]
impl LanguageServer for OxyLsp {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.update_and_diagnose(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // FULL sync: client sends the entire file on every change
        let text = params.content_changes.into_iter().last().unwrap().text;
        self.update_and_diagnose(params.text_document.uri, text).await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        // ...
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        // ...
    }
}
```

`tower-lsp` calls these methods when the corresponding LSP messages arrive. You return
typed structs; the library serializes them to JSON and sends them to the client.

---

## Diagnostics: re-running the pipeline

```rust
async fn update_and_diagnose(&self, uri: Url, source: String) {
    let diagnostics = Self::diagnose(&source);
    self.documents.lock().unwrap().insert(uri.clone(), source);
    self.client.publish_diagnostics(uri, diagnostics, None).await;
}

fn diagnose(source: &str) -> Vec<Diagnostic> {
    // Stage 1: lex
    if let Err(e) = oxy_core::lexer::tokenize(source) {
        return vec![error_to_diagnostic(&e)];
    }
    // Stage 2: parse
    let program = match oxy_core::parser::parse(source) {
        Ok(p) => p,
        Err(e) => return vec![error_to_diagnostic(&e)],
    };
    // Stage 3: type check
    match oxy_core::type_checker::TypeChecker::new().check_program(&program) {
        Ok(()) => vec![],
        Err(e) => vec![error_to_diagnostic(&e)],
    }
}
```

The three pipeline stages run sequentially. The LSP stops at the first stage that produces
an error — no point type-checking if parsing failed. This is why the error underline in
VS Code shows the lex error (earliest in the pipeline) when both a lex and type error exist.

---

## Completion: context-sensitive

```rust
async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
    let uri = &params.text_document_position.text_document.uri;
    let pos = params.text_document_position.position;

    let source = match self.get_document(uri) { ... };

    // Find the token at the cursor position
    let trigger = params.context.and_then(|c| c.trigger_character);

    let items = match trigger.as_deref() {
        Some(".") => {
            // Method completions: find what's before the dot
            let type_name = self.infer_type_before_dot(&source, pos);
            self.method_completions(&type_name)
        }
        Some("::") => {
            // Module/path completions
            self.path_completions(&source, pos)
        }
        _ => {
            // General: keywords + built-in types + visible functions
            self.general_completions(&source, pos)
        }
    };

    Ok(Some(CompletionResponse::Array(items)))
}
```

`method_completions(type_name)` reads `symbols::methods_for_type(type_name)` and returns
a completion item for each method. The method signatures come from `symbols.rs`.

---

## Error conversion: pipeline errors → LSP diagnostics

```rust
fn error_to_diagnostic(e: &PipelineError) -> Diagnostic {
    let (line, col, msg) = match e {
        PipelineError::Lexer { line, column, message } => (*line, *column, message.clone()),
        PipelineError::Parser { line, column, message } => (*line, *column, message.clone()),
        PipelineError::TypeError { line, column, message } => (*line, *column, message.clone()),
        PipelineError::Runtime { .. } => (0, 0, format!("{e}")),
    };

    Diagnostic {
        range: Range {
            start: Position { line: line.saturating_sub(1) as u32, character: col.saturating_sub(1) as u32 },
            end: Position { line: line.saturating_sub(1) as u32, character: (col + 1) as u32 },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        message: msg,
        ..Default::default()
    }
}
```

Note the `saturating_sub(1)`: Oxy spans are 1-based (line 1, column 1); LSP positions are
0-based (line 0, character 0). The conversion happens here, once, for every diagnostic.

---

## `tower-lsp` async: tokio under the hood

The LSP implementation is async (all trait methods are `async fn`). This is `tokio` —
the Rust async runtime. `tower-lsp` uses it to handle multiple concurrent LSP messages.

For Oxy's use case, this is mostly invisible: the synchronous `diagnose` function runs
inside `tokio::spawn` (for non-blocking diagnostics), and the document store uses
`Mutex` (not async `RwLock`) for simplicity. For a production LSP, you would use async-aware
data structures and incremental parsing.
