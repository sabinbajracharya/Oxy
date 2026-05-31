# Language Server Protocol: How IDEs Understand Your Code

The Language Server Protocol is one of the quietly transformative ideas in developer tooling, and
to appreciate it you have to remember the world before it. Until LSP arrived around 2016, editor
language support was a combinatorial disaster. If you wanted Python to feel good in VS Code,
somebody wrote a VS Code Python plugin. Want it in JetBrains? A completely separate plugin, written
again from scratch. Emacs? Vim? Sublime? Each one its own implementation, each understanding Python
slightly differently, each with its own bugs. With N languages and M editors, the world needed
N×M implementations — and most of those boxes were simply never filled, which is why your favorite
obscure language felt great in one editor and like a text file in every other.

LSP inverted the whole thing with one move: standardize the conversation between editor and
language. Now a language author writes *one* server that understands their language, every editor
ships *one* generic client that speaks the protocol, and the two snap together. N+M instead of N×M.
That's the difference between "language support is a heroic per-editor effort" and "write the
server once, get every editor for free" — and it's the reason a small language like Oxy can offer
real autocompletion, hover docs, and live error underlining at all.

For Oxy that made the LSP non-optional. A language you can't get diagnostics or completions for, in
the editor you already use, doesn't feel like a real language. So `oxy-lsp` exists, and everything
the editor knows about Oxy flows through it. And here's the reassuring part, before the acronyms
pile up: a language server is just a process that reads JSON messages on stdin and writes JSON
messages on stdout. That's the whole protocol. It is far less scary than it sounds.

## What LSP is

The Language Server Protocol defines a JSON-RPC conversation between an editor ("client")
and a language-understanding process ("server"). The server knows about one language.
The client is the editor — VS Code, JetBrains, Neovim, Emacs.

The conversation:
```json
// Client → Server: file opened
{ "method": "textDocument/didOpen", "params": { "uri": "file:///my_file.ox", "text": "..." } }

// Server → Client: here are the errors
{ "method": "textDocument/publishDiagnostics", "params": { "uri": "...", "diagnostics": [...] } }

// Client → Server: what completions at position 5:7?
{ "method": "textDocument/completion", "params": { "uri": "...", "position": {"line": 5, "character": 7} } }

// Server → Client: here are the completion items
{ "result": [{ "label": "println", "kind": "Function" }, ...] }
```

JSON over stdio. That's the entire protocol.

## What Oxy's LSP provides

The `oxy-lsp` crate (at `crates/oxy-lsp/src/main.rs`) implements these LSP features:

| Feature | How it works |
|---------|-------------|
| **Error diagnostics** | Re-runs lex → parse → type check on every file change; converts errors to LSP diagnostics |
| **Completion** | On trigger, finds what token precedes the cursor; if `.` → method completions from `symbols.rs`; if identifier → keyword/function completions |
| **Hover documentation** | On hover, identifies the token under cursor; looks up its type from `symbols.rs` |
| **Go to definition** | (planned, not yet implemented) |

## The server implementation

```rust
// crates/oxy-lsp/src/main.rs
struct OxyLsp {
    client: Client,
    documents: Mutex<HashMap<Url, String>>,  // open file contents
}

impl OxyLsp {
    fn diagnose(source: &str) -> Vec<Diagnostic> {
        // Lex → parse → type check → convert errors to LSP diagnostics
        if let Err(e) = oxy_core::lexer::tokenize(source) {
            return vec![error_to_diagnostic(&e)];
        }
        let program = match oxy_core::parser::parse(source) {
            Ok(p) => p,
            Err(e) => return vec![error_to_diagnostic(&e)],
        };
        if let Err(e) = TypeChecker::new().check_program(&program) {
            return vec![error_to_diagnostic(&e)];
        }
        vec![]
    }
}
```

On every file change, `diagnose` re-runs the full pipeline (lex → parse → type check).
The output is converted to LSP `Diagnostic` objects and pushed to the client.

This is "re-parse on every keystroke" — acceptable for small files, where oxy-core's pipeline
completes in under 10ms. For very large files, debouncing (wait for a pause in typing) would
be needed.

## `symbols.rs` as the LSP's knowledge base

The LSP never hardcodes keyword names, type names, or method names. It reads them from
`crates/oxy-core/src/symbols.rs`:

```rust
// crates/oxy-lsp/src/main.rs
fn completion_items_for_dot(type_name: &str) -> Vec<CompletionItem> {
    // Look up the methods for this type from symbols.rs
    let methods = oxy_core::symbols::methods_for_type(type_name);
    methods.iter().map(|m| CompletionItem {
        label: m.name.to_string(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some(m.signature.to_string()),
        ..Default::default()
    }).collect()
}
```

When Oxy gets a new built-in method (say, `Vec::partition`), it is added to `symbols.rs`.
The LSP automatically offers it in completions. There is no "update the LSP" step.

## Running the LSP

```bash
# Standalone (IDE connects via stdio)
docker compose run --rm dev bash -c "cargo run -p oxy-lsp"

# The VS Code extension launches this automatically
```

The LSP binary starts, reads JSON-RPC messages from stdin, writes responses to stdout.
The VS Code extension (in `editors/vscode/`) configures VS Code to launch this binary
and connect to it.
