# CLAUDE.md — Project Context

## Project: Oxy

Interpreted programming language written in Rust. Replicates Rust syntax without borrow checker/ownership. File extension: `.ox`.

### Success Criterion

**Complete bytecode migration and interpreter removal — then all tests pass natively.**

- Making tests pass by routing through interpreter fallback (`emit_eval`, `self.interpreter.call_method()`, etc.) is NOT success.
- Every feature must have native bytecode execution.
- The end goal is to delete the interpreter directory entirely — one execution path: compiler → VM.
- **Test regressions during architectural changes are expected and acceptable.** Do not revert necessary architectural work just to keep tests green. Temporarily disable failing tests if they distract from productive work — re-enable when the migration is complete.
- Tests are a verification tool, not the goal itself. The goal is zero interpreter code.

## Build & Test (Docker — no local Rust)

```bash
docker compose run --rm dev bash -c "cargo test"                    # All tests
docker compose run --rm dev bash -c "cargo test -p oxy-core"    # Core only
docker compose run --rm dev bash -c "cargo test -p oxy-lsp"     # LSP only
docker compose run --rm dev bash -c "cargo fmt --all"               # Format
docker compose run --rm dev bash -c "cargo clippy -- -D warnings"   # Lint
docker compose run --rm dev bash -c "cargo run -- run examples/hello.ox"  # Run
docker compose run --rm test                                        # Full CI check
docker compose run --rm setup                                       # Install npm deps
docker compose run --rm build-ext                                   # Package .vsix
```

## Architecture

```
crates/
├── oxy-core/src/
│   ├── lib.rs           # Public API exports
│   ├── lexer/           # Tokenizer → Vec<Token>
│   │   ├── mod.rs       # Scanner (scan_token, scan_string, scan_fstring, scan_number)
│   │   └── token.rs     # Token, TokenKind (keywords, operators, literals), Span
│   ├── ast/mod.rs       # AST nodes: Item, Expr, Stmt, FnDef, StructDef, EnumDef, Attribute, FStringPart
│   ├── parser/mod.rs    # Pratt parser (~3200 lines). Precedence levels 0-14.
│   ├── interpreter/mod.rs  # Tree-walking evaluator (~7000+ lines). All runtime logic.
│   ├── types/mod.rs     # Value enum: Integer, Float, Bool, String, Vec, HashMap, Struct, EnumVariant, Function(Box), Future(Box), etc.
│   ├── env/mod.rs       # Lexical scope chain (parent pointer)
│   ├── json/mod.rs      # Hand-written JSON ser/de (no deps)
│   ├── http/mod.rs      # HTTP client wrapping ureq
│   └── errors.rs        # FerriError: Lexer/Parser/Runtime with line/column, Return(Box<Value>), Break, Continue
├── oxy-cli/src/main.rs  # CLI: run, repl, --dump-tokens, --dump-ast
└── oxy-lsp/src/main.rs  # LSP server (tower-lsp): diagnostics, completion, hover, symbols, goto-def
editors/vscode/
├── extension.js         # LSP client — launches oxy-lsp via Docker or native binary
├── package.json         # Extension manifest with oxy.lsp.mode/path/enabled settings
├── syntaxes/oxy.tmLanguage.json  # TextMate grammar
└── language-configuration.json       # Brackets, comments, indentation
```

## Key Patterns (follow these when adding features)

### Adding a built-in macro (e.g. `println!`, `vec!`, `format!`)
→ Add match arm in `interpreter::eval_macro_call()`

### Adding a path-call module (e.g. `math::sqrt()`, `json::parse()`, `http::get()`)
→ Add match arm in `interpreter::eval_path_call()` under the module prefix

### Adding a path constant (e.g. `math::PI`)
→ Handle in `interpreter::eval_expr()` under `Expr::Path`

### Adding methods on built-in types (e.g. `.sqrt()`, `.len()`, `.clone()`)
→ Add match arm in `interpreter::call_method()` → type-specific dispatcher:
  - `call_vec_method()`, `call_string_method()`, `call_hashmap_method()`
  - Numeric methods handled inline in `call_method()`

### Adding a new expression type
1. Add variant to `Expr` enum in `ast/mod.rs`
2. Parse it in `parser/mod.rs` (`parse_prefix` or `parse_infix`)
3. Evaluate it in `interpreter::eval_expr()`

### Adding a new item type (struct/enum feature)
1. Extend AST node in `ast/mod.rs` (e.g. add field to `StructDef`)
2. Parse in `parser::parse_item()`
3. Register in `interpreter::register_item()`

### Adding a new Value type
1. Add variant to `Value` enum in `types/mod.rs` + Display impl
2. Handle in `interpreter::call_method()` for methods
3. Handle in binary/comparison operators if needed

## Critical Implementation Details

- **Value::Function and Value::Future are boxed** — `Function(Box<FunctionData>)`, `Future(Box<FutureData>)` to prevent stack overflow in recursive code
- **FerriError::Return and Break are boxed** — `Return(Box<Value>)`, `Break(Option<Box<Value>>)` to satisfy clippy result_large_err
- **Spans are 1-indexed** — line 1, column 1. LSP converts to 0-indexed.
- **Struct init disambiguation** — `Ident { ... }` is struct init only if name starts with uppercase
- **Option/Result** — modeled as `Value::EnumVariant` with `enum_name: "Option"/"Result"`
- **Test pattern** — `run_and_capture(src)` returns `Vec<String>` of output lines (each ending `\n`). Source must wrap in `fn main() { ... }`.
- **Pratt parser precedence** — None(0) → Assignment(1) → Range(2) → Or(3) → And(4) → Equality(8) → Comparison(9) → Term(11) → Factor(12) → Unary(13) → Call(14)
- **Method dispatch order** — Vec → String → HashMap → Option/Result → HttpResponse → HttpRequestBuilder → numeric methods → .to_json() → impl blocks → trait impls → trait defaults
- **Derived traits** tracked in `interpreter.derived_traits: HashMap<String, HashSet<String>>`
- **Async** — simulated, not real threads. `async fn` returns lazy `Value::Future`, `.await` evaluates it.

## Conventions

- `rustfmt.toml` config (max width 100)
- `thiserror` for error types
- Conventional commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`
- Always add trailer: `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`
- Tests: `test_<what>_<scenario>` naming, unit tests in `#[cfg(test)]` modules

# context-mode — MANDATORY routing rules

You have context-mode MCP tools available. These rules are NOT optional — they protect your context window from flooding. A single unrouted command can dump 56 KB into context and waste the entire session.

## BLOCKED commands — do NOT attempt these

### curl / wget — BLOCKED
Any Bash command containing `curl` or `wget` is intercepted and replaced with an error message. Do NOT retry.
Instead use:
- `ctx_fetch_and_index(url, source)` to fetch and index web pages
- `ctx_execute(language: "javascript", code: "const r = await fetch(...)")` to run HTTP calls in sandbox

### Inline HTTP — BLOCKED
Any Bash command containing `fetch('http`, `requests.get(`, `requests.post(`, `http.get(`, or `http.request(` is intercepted and replaced with an error message. Do NOT retry with Bash.
Instead use:
- `ctx_execute(language, code)` to run HTTP calls in sandbox — only stdout enters context

### WebFetch — BLOCKED
WebFetch calls are denied entirely. The URL is extracted and you are told to use `ctx_fetch_and_index` instead.
Instead use:
- `ctx_fetch_and_index(url, source)` then `ctx_search(queries)` to query the indexed content

## REDIRECTED tools — use sandbox equivalents

### Bash (>20 lines output)
Bash is ONLY for: `git`, `mkdir`, `rm`, `mv`, `cd`, `ls`, `npm install`, `pip install`, and other short-output commands.
For everything else, use:
- `ctx_batch_execute(commands, queries)` — run multiple commands + search in ONE call
- `ctx_execute(language: "shell", code: "...")` — run in sandbox, only stdout enters context

### Read (for analysis)
If you are reading a file to **Edit** it → Read is correct (Edit needs content in context).
If you are reading to **analyze, explore, or summarize** → use `ctx_execute_file(path, language, code)` instead. Only your printed summary enters context. The raw file content stays in the sandbox.

### Grep (large results)
Grep results can flood context. Use `ctx_execute(language: "shell", code: "grep ...")` to run searches in sandbox. Only your printed summary enters context.

## Tool selection hierarchy

1. **GATHER**: `ctx_batch_execute(commands, queries)` — Primary tool. Runs all commands, auto-indexes output, returns search results. ONE call replaces 30+ individual calls.
2. **FOLLOW-UP**: `ctx_search(queries: ["q1", "q2", ...])` — Query indexed content. Pass ALL questions as array in ONE call.
3. **PROCESSING**: `ctx_execute(language, code)` | `ctx_execute_file(path, language, code)` — Sandbox execution. Only stdout enters context.
4. **WEB**: `ctx_fetch_and_index(url, source)` then `ctx_search(queries)` — Fetch, chunk, index, query. Raw HTML never enters context.
5. **INDEX**: `ctx_index(content, source)` — Store content in FTS5 knowledge base for later search.

## Subagent routing

When spawning subagents (Agent/Task tool), the routing block is automatically injected into their prompt. Bash-type subagents are upgraded to general-purpose so they have access to MCP tools. You do NOT need to manually instruct subagents about context-mode.

## Output constraints

- Keep responses under 500 words.
- Write artifacts (code, configs, PRDs) to FILES — never return them as inline text. Return only: file path + 1-line description.
- When indexing content, use descriptive source labels so others can `ctx_search(source: "label")` later.

## ctx commands

| Command | Action |
|---------|--------|
| `ctx stats` | Call the `ctx_stats` MCP tool and display the full output verbatim |
| `ctx doctor` | Call the `ctx_doctor` MCP tool, run the returned shell command, display as checklist |
| `ctx upgrade` | Call the `ctx_upgrade` MCP tool, run the returned shell command, display as checklist |
