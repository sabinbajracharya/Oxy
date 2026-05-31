# The VS Code Extension

<!-- OPUS_FILL
Write a 1-paragraph intro. The VS Code extension is the final layer — the thing users
actually interact with. It is thin glue: syntax highlighting and LSP client.
The language intelligence comes from the LSP; the extension just wires it up.
Frame it as: "The extension is small on purpose. The real work is in the LSP."
-->

**Directory:** `editors/vscode/`

Files:
- `package.json` — extension manifest, LSP client configuration
- `oxy.tmLanguage.json` — TextMate grammar for syntax highlighting

---

## Syntax highlighting: `oxy.tmLanguage.json`

Syntax highlighting uses TextMate grammars — a JSON file that defines regex patterns
for different token classes:

```json
{
  "name": "Oxy",
  "scopeName": "source.oxy",
  "patterns": [
    { "include": "#keywords" },
    { "include": "#strings" },
    { "include": "#comments" },
    { "include": "#numbers" }
  ],
  "repository": {
    "keywords": {
      "match": "\\b(fn|let|mut|if|else|while|for|in|struct|enum|impl|trait|pub|use|mod|return|match|self|Self|async|await|break|continue|loop|const|type|where|move|dyn|super|crate|as|ref|static)\\b",
      "name": "keyword.control.oxy"
    },
    "types": {
      "match": "\\b(int|float|byte|bool|String|char|Vec|HashMap|Option|Result)\\b",
      "name": "storage.type.oxy"
    }
  }
}
```

When a new keyword is added to Oxy, the `oxy.tmLanguage.json` should be updated.
Syntax highlighting does not go through the LSP — it is static regex matching.

**Note:** the LSP provides richer highlighting (semantic tokens) but the TextMate grammar
is the fallback when the LSP is not running or for large files where semantic highlighting
is slow.

---

## The LSP client: `package.json`

```json
{
  "name": "oxy-lang",
  "contributes": {
    "languages": [
      {
        "id": "oxy",
        "aliases": ["Oxy"],
        "extensions": [".ox"],
        "configuration": "./language-configuration.json"
      }
    ],
    "grammars": [
      {
        "language": "oxy",
        "scopeName": "source.oxy",
        "path": "./oxy.tmLanguage.json"
      }
    ]
  },
  "activationEvents": ["onLanguage:oxy"],
  "main": "./extension.js"
}
```

The `main` file (`extension.js`) contains the LSP client code — the JavaScript that:
1. Finds the `oxy-lsp` binary
2. Launches it as a child process
3. Connects VS Code to it via the `vscode-languageclient` library

```javascript
// extension.js (simplified)
const serverOptions = {
    command: 'docker',
    args: ['compose', 'run', '--rm', '-T', 'dev', 'bash', '-c', 'cargo run -p oxy-lsp'],
};

const clientOptions = {
    documentSelector: [{ scheme: 'file', language: 'oxy' }],
};

const client = new LanguageClient('oxy-lsp', 'Oxy Language Server', serverOptions, clientOptions);
client.start();
```

The current implementation launches the LSP via Docker — meaning VS Code → Docker → `oxy-lsp`.
This avoids requiring `oxy-lsp` to be installed natively on the developer's machine.

---

## Building and installing the extension

```bash
# Build the .vsix package
docker compose run --rm build-ext

# Install in VS Code
code --install-extension editors/vscode/oxy-lang-*.vsix
```

The `.vsix` is a zip file containing `package.json`, `oxy.tmLanguage.json`, and `extension.js`.
VS Code installs it like any other extension.

---

## What works when you open an `.ox` file

1. VS Code sees the `.ox` extension → activates the `oxy-lang` extension
2. Extension launches `oxy-lsp` (via Docker or native, depending on setup)
3. Extension connects VS Code to `oxy-lsp` via LSP
4. On file open: `oxy-lsp` receives `textDocument/didOpen`, runs diagnostics, sends results
5. VS Code shows error underlines
6. On `.` keypress: VS Code sends `textDocument/completion`, `oxy-lsp` returns method completions
7. VS Code shows the completion dropdown with method names from `symbols.rs`
