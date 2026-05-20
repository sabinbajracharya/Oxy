# Oxy Language — VS Code Extension

Syntax highlighting and Language Server Protocol (LSP) support for [Oxy](../../README.md) (`.ox` files).

## Features

- **Syntax highlighting** — keywords, types, operators, strings, numbers, comments, attributes
- **Real-time diagnostics** — lexer, parser, type checker, and compiler errors (including visibility violations) shown as you type
- **Autocompletion** — keywords, built-in and user-defined types/functions/methods, modules, code snippets, and `::`-scoped module member completion
- **Hover** — documentation for keywords, built-in types/functions, and user-defined items (functions, structs, enums, traits)
- **Go-to definition** — jump to definitions in the same file, resolves `use` imports
- **Document symbols** — outline view of functions, structs, enums, traits, modules
- **Dot-completions** — type-aware method suggestions after `.`
- **Bracket matching**, auto-closing, comment toggling

## Quick Start

The extension works out of the box with Docker (default `auto` mode).

```bash
# From the project root, symlink into VS Code extensions
ln -s "$(pwd)/editors/vscode" ~/.vscode/extensions/oxy-lang
```

Reload VS Code: `Cmd+Shift+P` → "Reload Window". Open any `.ox` file.

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `oxy.lsp.mode` | `auto` | `auto` = Docker if no custom path, `docker` = always Docker, `native` = local binary |
| `oxy.lsp.path` | `oxy-lsp` | Path to local binary (only used in `native` mode) |
| `oxy.lsp.enabled` | `true` | Enable/disable the language server |
