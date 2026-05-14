use std::collections::HashMap;
use std::sync::Mutex;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use oxy_core::ast::{Item, Program};
use oxy_core::errors::FerriError;

struct OxyLsp {
    client: Client,
    documents: Mutex<HashMap<Url, String>>,
}

impl OxyLsp {
    fn update_and_diagnose(&self, uri: Url, source: String) {
        let diagnostics = Self::diagnose(&source);
        self.documents.lock().unwrap().insert(uri.clone(), source);
        let client = self.client.clone();
        tokio::spawn(async move {
            client.publish_diagnostics(uri, diagnostics, None).await;
        });
    }

    fn diagnose(source: &str) -> Vec<Diagnostic> {
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

    fn try_parse(source: &str) -> Option<Program> {
        oxy_core::parser::parse(source).ok()
    }

    /// Extract the word at a given position from source text.
    fn word_at_position(source: &str, position: Position) -> Option<String> {
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

        // Include trailing `!` for macros like println!
        if end < bytes.len() && bytes[end] == b'!' {
            end += 1;
        }

        Some(line[start..end].to_string())
    }
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn error_to_diagnostic(e: &FerriError) -> Diagnostic {
    let (message, line, column) = match e {
        FerriError::Lexer {
            message,
            line,
            column,
        } => (message.clone(), *line, *column),
        FerriError::Parser {
            message,
            line,
            column,
        } => (message.clone(), *line, *column),
        FerriError::Runtime {
            message,
            line,
            column,
        } => (message.clone(), *line, *column),
        _ => (e.to_string(), 1, 1),
    };

    // Oxy spans are 1-indexed; LSP is 0-indexed.
    let line0 = if line > 0 { line - 1 } else { 0 } as u32;
    let col0 = if column > 0 { column - 1 } else { 0 } as u32;
    let pos = Position::new(line0, col0);

    Diagnostic {
        range: Range::new(pos, pos),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("oxy".to_string()),
        message,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Completion data
// ---------------------------------------------------------------------------

fn keyword_completions() -> Vec<CompletionItem> {
    let keywords = [
        "let", "mut", "fn", "struct", "enum", "impl", "trait", "if", "else", "while", "loop",
        "for", "in", "match", "return", "break", "continue", "pub", "mod", "use", "const",
        "static", "type", "async", "await", "move",
    ];
    keywords
        .iter()
        .map(|kw| CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        })
        .collect()
}

fn type_completions() -> Vec<CompletionItem> {
    let types = [
        ("i64", "64-bit signed integer"),
        ("f64", "64-bit floating point"),
        ("bool", "Boolean type"),
        ("String", "Owned UTF-8 string"),
        ("Vec", "Growable array type"),
        ("HashMap", "Hash map collection"),
        ("Option", "Optional value: Some(T) or None"),
        ("Result", "Result type: Ok(T) or Err(E)"),
        ("Self", "Current type in impl block"),
    ];
    types
        .iter()
        .map(|(name, detail)| CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::CLASS),
            detail: Some(detail.to_string()),
            ..Default::default()
        })
        .collect()
}

fn builtin_function_completions() -> Vec<CompletionItem> {
    let builtins = [
        ("println!", "Print with newline"),
        ("print!", "Print without newline"),
        ("format!", "Format a string"),
        ("eprintln!", "Print to stderr"),
        ("dbg!", "Debug print"),
        ("panic!", "Panic with message"),
        ("todo!", "Mark unfinished code"),
        ("unimplemented!", "Mark unimplemented code"),
        ("vec!", "Create a Vec"),
        ("spawn", "Spawn an async task"),
        ("sleep", "Sleep for a duration"),
    ];
    builtins
        .iter()
        .map(|(name, detail)| CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(detail.to_string()),
            ..Default::default()
        })
        .collect()
}

fn module_completions() -> Vec<CompletionItem> {
    let modules = [
        ("json::", "JSON module"),
        ("http::", "HTTP module"),
        ("std::fs::", "Filesystem module"),
        ("std::env::", "Environment module"),
        ("std::process::", "Process module"),
    ];
    modules
        .iter()
        .map(|(name, detail)| CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::MODULE),
            detail: Some(detail.to_string()),
            ..Default::default()
        })
        .collect()
}

fn snippet_completions() -> Vec<CompletionItem> {
    let snippets: &[(&str, &str, &str)] = &[
        ("fn main", "fn main() {\n    $0\n}", "Main function"),
        (
            "fn",
            "fn ${1:name}(${2:params}) {\n    $0\n}",
            "Function definition",
        ),
        (
            "struct",
            "struct ${1:Name} {\n    $0\n}",
            "Struct definition",
        ),
        ("enum", "enum ${1:Name} {\n    $0\n}", "Enum definition"),
        ("impl", "impl ${1:Type} {\n    $0\n}", "Impl block"),
        (
            "match",
            "match ${1:expr} {\n    ${2:pattern} => $0,\n}",
            "Match expression",
        ),
        ("for", "for ${1:item} in ${2:iter} {\n    $0\n}", "For loop"),
        ("while", "while ${1:condition} {\n    $0\n}", "While loop"),
        (
            "if let",
            "if let ${1:pattern} = ${2:expr} {\n    $0\n}",
            "If let binding",
        ),
    ];
    snippets
        .iter()
        .map(|(label, body, detail)| CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some(detail.to_string()),
            insert_text: Some(body.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Hover data
// ---------------------------------------------------------------------------

fn keyword_hover(word: &str) -> Option<String> {
    let desc = match word {
        "let" => "Bind a value to a variable.\n\n```oxy\nlet x = 42;\nlet mut y = 0;\n```",
        "mut" => "Mark a variable as mutable.",
        "fn" => "Declare a function.\n\n```oxy\nfn add(a: i64, b: i64) -> i64 { a + b }\n```",
        "struct" => "Define a struct type.\n\n```oxy\nstruct Point { x: f64, y: f64 }\n```",
        "enum" => "Define an enum type.\n\n```oxy\nenum Color { Red, Green, Blue }\n```",
        "impl" => "Implement methods on a type.",
        "trait" => "Define a trait (interface).",
        "if" => "Conditional branching.",
        "else" => "Alternative branch of an `if` expression.",
        "while" => "Loop while a condition is true.",
        "loop" => "Loop forever (until `break`).",
        "for" => "Iterate over a range or collection.\n\n```oxy\nfor i in 0..10 { println!(\"{}\", i); }\n```",
        "in" => "Used in `for` loops to specify the iterator.",
        "match" => "Pattern matching.\n\n```oxy\nmatch value { 1 => \"one\", _ => \"other\" }\n```",
        "return" => "Return a value from a function.",
        "break" => "Exit a loop.",
        "continue" => "Skip to the next loop iteration.",
        "pub" => "Mark an item as public.",
        "mod" => "Define or reference a module.",
        "use" => "Import items from a module.",
        "const" => "Declare a compile-time constant.",
        "static" => "Declare a static variable.",
        "type" => "Create a type alias.",
        "async" => "Mark a function as asynchronous.",
        "await" => "Await an async expression.",
        "move" => "Move captured variables into a closure.",
        _ => return None,
    };
    Some(desc.to_string())
}

fn builtin_hover(word: &str) -> Option<String> {
    let desc = match word {
        "i64" => "**i64** — 64-bit signed integer type",
        "f64" => "**f64** — 64-bit floating-point type",
        "bool" => "**bool** — Boolean type (`true` or `false`)",
        "String" => "**String** — Owned, heap-allocated UTF-8 string",
        "Vec" => "**Vec\\<T\\>** — Growable array\n\n```oxy\nlet v: Vec<i64> = vec![1, 2, 3];\n```",
        "HashMap" => "**HashMap\\<K, V\\>** — Hash map collection",
        "Option" => "**Option\\<T\\>** — `Some(value)` or `None`",
        "Result" => "**Result\\<T, E\\>** — `Ok(value)` or `Err(error)`",
        "println!" => "**println!(fmt, ...)** — Print to stdout with a newline",
        "print!" => "**print!(fmt, ...)** — Print to stdout without a newline",
        "format!" => "**format!(fmt, ...)** — Format into a String",
        "eprintln!" => "**eprintln!(fmt, ...)** — Print to stderr with a newline",
        "dbg!" => "**dbg!(expr)** — Debug-print an expression and return it",
        "panic!" => "**panic!(msg)** — Abort with an error message",
        "todo!" => "**todo!()** — Mark unfinished code (panics at runtime)",
        "unimplemented!" => "**unimplemented!()** — Mark unimplemented code (panics at runtime)",
        "vec!" => "**vec![items...]** — Create a Vec from elements",
        "spawn" => "**spawn(async_fn)** — Spawn an async task",
        "sleep" => "**sleep(ms)** — Sleep for the given milliseconds",
        _ => return None,
    };
    Some(desc.to_string())
}

// ---------------------------------------------------------------------------
// Document symbols
// ---------------------------------------------------------------------------

fn item_to_symbol(item: &Item, source: &str) -> Option<DocumentSymbol> {
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

fn byte_offset_to_position(source: &str, offset: usize) -> Position {
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

        // Check if cursor is after a dot — suggest methods
        if is_after_dot(&source, pos) {
            return Ok(Some(CompletionResponse::Array(method_completions())));
        }

        let mut items = Vec::new();
        items.extend(keyword_completions());
        items.extend(type_completions());
        items.extend(builtin_function_completions());
        items.extend(module_completions());
        items.extend(snippet_completions());
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

        // Check keywords
        if let Some(desc) = keyword_hover(&word) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: desc,
                }),
                range: None,
            }));
        }

        // Check built-in types/functions
        if let Some(desc) = builtin_hover(&word) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: desc,
                }),
                range: None,
            }));
        }

        // Check user-defined items
        if let Some(program) = Self::try_parse(&source) {
            for item in &program.items {
                if let Some(desc) = item_hover_info(item, &word) {
                    return Ok(Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: desc,
                        }),
                        range: None,
                    }));
                }
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

        for item in &program.items {
            if let Some(span) = item_definition_span(item, &word) {
                let line0 = if span.line > 0 { span.line - 1 } else { 0 } as u32;
                let col0 = if span.column > 0 { span.column - 1 } else { 0 } as u32;
                let start = Position::new(line0, col0);
                let end_pos = byte_offset_to_position(&source, span.end);
                let loc = Location::new(uri.clone(), Range::new(start, end_pos));
                return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
            }
        }

        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Helpers for go-to-definition and hover on user items
// ---------------------------------------------------------------------------

/// Check if the cursor position is immediately after a dot.
fn is_after_dot(source: &str, position: Position) -> bool {
    let line = match source.lines().nth(position.line as usize) {
        Some(l) => l,
        None => return false,
    };
    let col = position.character as usize;
    if col == 0 || col > line.len() {
        return false;
    }
    let bytes = line.as_bytes();
    // Check if the character before cursor is a dot
    if let Some(&b'.') = bytes.get(col.saturating_sub(1)) {
        return true;
    }
    // Check for method call with two chars before (allow space after dot)
    false
}

/// Completions for method calls after a dot.
fn method_completions() -> Vec<CompletionItem> {
    let methods: &[(&str, CompletionItemKind, &str)] = &[
        // Vec methods
        (
            "push",
            CompletionItemKind::METHOD,
            "Add element to end of Vec",
        ),
        (
            "pop",
            CompletionItemKind::METHOD,
            "Remove and return last element",
        ),
        (
            "len",
            CompletionItemKind::METHOD,
            "Return number of elements",
        ),
        (
            "first",
            CompletionItemKind::METHOD,
            "Return first element as Option",
        ),
        (
            "last",
            CompletionItemKind::METHOD,
            "Return last element as Option",
        ),
        (
            "is_empty",
            CompletionItemKind::METHOD,
            "Check if Vec is empty",
        ),
        (
            "iter",
            CompletionItemKind::METHOD,
            "Return iterator over elements",
        ),
        ("map", CompletionItemKind::METHOD, "Transform each element"),
        (
            "filter",
            CompletionItemKind::METHOD,
            "Keep elements passing predicate",
        ),
        (
            "fold",
            CompletionItemKind::METHOD,
            "Reduce elements to single value",
        ),
        (
            "any",
            CompletionItemKind::METHOD,
            "Check if any element matches",
        ),
        (
            "all",
            CompletionItemKind::METHOD,
            "Check if all elements match",
        ),
        (
            "find",
            CompletionItemKind::METHOD,
            "Find first matching element",
        ),
        (
            "enumerate",
            CompletionItemKind::METHOD,
            "Iterate with index",
        ),
        (
            "for_each",
            CompletionItemKind::METHOD,
            "Execute closure on each element",
        ),
        ("flat_map", CompletionItemKind::METHOD, "Map and flatten"),
        (
            "position",
            CompletionItemKind::METHOD,
            "Find index of matching element",
        ),
        ("sort", CompletionItemKind::METHOD, "Sort elements"),
        ("reverse", CompletionItemKind::METHOD, "Reverse order"),
        ("join", CompletionItemKind::METHOD, "Join string elements"),
        // String methods
        (
            "to_uppercase",
            CompletionItemKind::METHOD,
            "Convert to uppercase",
        ),
        (
            "to_lowercase",
            CompletionItemKind::METHOD,
            "Convert to lowercase",
        ),
        (
            "contains",
            CompletionItemKind::METHOD,
            "Check if contains substring",
        ),
        (
            "starts_with",
            CompletionItemKind::METHOD,
            "Check if starts with prefix",
        ),
        (
            "ends_with",
            CompletionItemKind::METHOD,
            "Check if ends with suffix",
        ),
        (
            "trim",
            CompletionItemKind::METHOD,
            "Remove whitespace from ends",
        ),
        (
            "split",
            CompletionItemKind::METHOD,
            "Split into Vec by delimiter",
        ),
        ("replace", CompletionItemKind::METHOD, "Replace occurrences"),
        (
            "repeat",
            CompletionItemKind::METHOD,
            "Repeat string n times",
        ),
        (
            "chars",
            CompletionItemKind::METHOD,
            "Return iterator over chars",
        ),
        ("lines", CompletionItemKind::METHOD, "Split into lines"),
        // HashMap methods
        (
            "insert",
            CompletionItemKind::METHOD,
            "Insert key-value pair",
        ),
        ("get", CompletionItemKind::METHOD, "Get value by key"),
        (
            "remove",
            CompletionItemKind::METHOD,
            "Remove key-value pair",
        ),
        (
            "contains_key",
            CompletionItemKind::METHOD,
            "Check if key exists",
        ),
        (
            "keys",
            CompletionItemKind::METHOD,
            "Return iterator over keys",
        ),
        (
            "values",
            CompletionItemKind::METHOD,
            "Return iterator over values",
        ),
        // Option/Result methods
        ("unwrap", CompletionItemKind::METHOD, "Get value or panic"),
        (
            "unwrap_or",
            CompletionItemKind::METHOD,
            "Get value or default",
        ),
        (
            "is_some",
            CompletionItemKind::METHOD,
            "Check if Some variant",
        ),
        (
            "is_none",
            CompletionItemKind::METHOD,
            "Check if None variant",
        ),
        ("is_ok", CompletionItemKind::METHOD, "Check if Ok variant"),
        ("is_err", CompletionItemKind::METHOD, "Check if Err variant"),
        ("ok", CompletionItemKind::METHOD, "Convert Result to Option"),
        (
            "err",
            CompletionItemKind::METHOD,
            "Convert Result to Option<Err>",
        ),
        // Generic methods
        ("clone", CompletionItemKind::METHOD, "Create a shallow copy"),
        ("to_string", CompletionItemKind::METHOD, "Convert to String"),
        ("to_json", CompletionItemKind::METHOD, "Serialize to JSON"),
        (
            "to_json_pretty",
            CompletionItemKind::METHOD,
            "Serialize to pretty JSON",
        ),
    ];
    methods
        .iter()
        .map(|(name, kind, detail)| CompletionItem {
            label: name.to_string(),
            kind: Some(*kind),
            detail: Some(detail.to_string()),
            ..Default::default()
        })
        .collect()
}

fn item_name(item: &Item) -> Option<&str> {
    match item {
        Item::Function(f) => Some(&f.name),
        Item::Struct(s) => Some(&s.name),
        Item::Enum(e) => Some(&e.name),
        Item::Trait(t) => Some(&t.name),
        Item::Module(m) => Some(&m.name),
        Item::Const { name, .. } => Some(name),
        Item::TypeAlias { name, .. } => Some(name),
        _ => None,
    }
}

fn item_definition_span(item: &Item, name: &str) -> Option<oxy_core::lexer::Span> {
    if item_name(item) == Some(name) {
        Some(item.span())
    } else {
        None
    }
}

fn item_hover_info(item: &Item, name: &str) -> Option<String> {
    if item_name(item) != Some(name) {
        return None;
    }
    match item {
        Item::Function(f) => {
            let params: Vec<String> = f
                .params
                .iter()
                .map(|p| format!("{}: {}", p.name, p.type_ann.name))
                .collect();
            let ret = f
                .return_type
                .as_ref()
                .map(|t| format!(" -> {}", t.name))
                .unwrap_or_default();
            let asyncness = if f.is_async { "async " } else { "" };
            Some(format!(
                "```oxy\n{}fn {}({}){}\n```",
                asyncness,
                f.name,
                params.join(", "),
                ret
            ))
        }
        Item::Struct(s) => Some(format!("```oxy\nstruct {}\n```", s.name)),
        Item::Enum(e) => {
            let variants: Vec<&str> = e.variants.iter().map(|v| v.name.as_str()).collect();
            Some(format!(
                "```oxy\nenum {} {{ {} }}\n```",
                e.name,
                variants.join(", ")
            ))
        }
        Item::Trait(t) => Some(format!("```oxy\ntrait {}\n```", t.name)),
        _ => Some(format!("**{}**", name)),
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| OxyLsp {
        client,
        documents: Mutex::new(HashMap::new()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_to_diagnostic() {
        let err = FerriError::Parser {
            message: "unexpected token".to_string(),
            line: 3,
            column: 5,
        };
        let diag = error_to_diagnostic(&err);
        assert_eq!(diag.range.start.line, 2); // 0-indexed
        assert_eq!(diag.range.start.character, 4);
        assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diag.message, "unexpected token");
    }

    #[test]
    fn test_word_at_position() {
        let source = "fn hello(x: i64) {}";
        let word = OxyLsp::word_at_position(source, Position::new(0, 3));
        assert_eq!(word, Some("hello".to_string()));

        let word = OxyLsp::word_at_position(source, Position::new(0, 0));
        assert_eq!(word, Some("fn".to_string()));

        let word = OxyLsp::word_at_position(source, Position::new(0, 12));
        assert_eq!(word, Some("i64".to_string()));
    }

    #[test]
    fn test_keyword_hover() {
        assert!(keyword_hover("fn").is_some());
        assert!(keyword_hover("let").is_some());
        assert!(keyword_hover("notakeyword").is_none());
    }

    #[test]
    fn test_builtin_hover() {
        assert!(builtin_hover("i64").is_some());
        assert!(builtin_hover("println!").is_some());
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
