# Oxide Language — VS Code Extension

Syntax highlighting and Language Server Protocol (LSP) support for [Oxide](../../README.md) (`.ox` files).

## Features

- **Syntax highlighting** for all Oxide keywords, types, operators, strings, numbers, comments
- **Real-time diagnostics** — parse errors shown as you type
- **Autocompletion** — keywords, built-in types, functions, modules, and code snippets
- **Hover information** — documentation for keywords, types, and built-in functions
- **Document symbols** — outline view of functions, structs, enums, traits
- **Go-to definition** — jump to function/struct/enum/trait definitions in the same file
- **Bracket matching** and auto-closing
- **Comment toggling** (`Cmd+/` for line comments, `Shift+Alt+A` for block comments)

## Installation

### 1. Build the LSP server

```bash
# From the project root
docker compose run --rm dev bash -c "cargo build --release -p oxide-lsp"

# The binary will be at target/release/oxide-lsp
```

### 2. Install the extension

```bash
# Install npm dependencies (needed for the LSP client)
cd editors/vscode
docker compose run --rm dev bash -c "apt-get update -qq && apt-get install -y -qq nodejs npm > /dev/null && cd editors/vscode && npm install --omit=dev"

# Symlink into VS Code extensions
ln -s $(pwd) ~/.vscode/extensions/oxide-lang

# Or copy
cp -r . ~/.vscode/extensions/oxide-lang
```

### 3. Configure the LSP binary path

Open VS Code settings and set `oxide.lsp.path` to the absolute path of your `oxide-lsp` binary:

```json
{
    "oxide.lsp.path": "/path/to/project-oxide/target/release/oxide-lsp"
}
```

### 4. Reload VS Code

`Cmd+Shift+P` → "Reload Window"

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `oxide.lsp.mode` | `auto` | `auto` = Docker if no custom path, `docker` = always Docker, `native` = local binary |
| `oxide.lsp.path` | `oxide-lsp` | Path to local `oxide-lsp` binary (only used in `native` mode) |
| `oxide.lsp.enabled` | `true` | Enable/disable the language server |

## File Association

Files with the `.ox` extension are automatically associated with the Oxide language.
