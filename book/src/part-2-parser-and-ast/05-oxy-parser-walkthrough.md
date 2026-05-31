# Oxy's Parser: A Full Walkthrough

The lexer handed us a flat list of tokens; the parser turns it into the tree we just spent two
chapters describing. Where the lexer was a single file, the parser is split across five, each
owning one domain: `mod.rs` holds the `Parser` struct and the precedence machinery, `expr.rs`
does the Pratt expression parsing, `stmt.rs` handles statements, `item.rs` handles top-level
declarations, and `ty.rs` and `pattern.rs` handle type annotations and patterns. The split is by
*what is being parsed*, not by phase — there's still only one left-to-right pass. Open the files
and let's walk through how they fit together.

**Files:**
- `crates/oxy-core/src/parser/mod.rs` — `Parser` struct, `Precedence`, helper methods
- `crates/oxy-core/src/parser/expr.rs` — expression parsing (Pratt)
- `crates/oxy-core/src/parser/stmt.rs` — statement parsing
- `crates/oxy-core/src/parser/item.rs` — item parsing (fn, struct, enum, impl, trait, mod)
- `crates/oxy-core/src/parser/ty.rs` — type annotation parsing
- `crates/oxy-core/src/parser/pattern.rs` — pattern parsing (match arms, let destructure)

---

## The public API

```rust
// Called from the pipeline:
pub fn parse(source: &str) -> Result<Program, PipelineError> {
    let tokens = tokenize(source)?;
    Parser::new(tokens).parse_program()
}
```

Tokenize, then parse. The parser never touches the source string — it only sees the token list.

## The `Parser` struct

```rust
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    ctx: ParseContext,
}
```

`pos` is the current position in the token list. The parser advances forward one token at
a time, never going back. `ctx` holds mode flags — the most important being `no_struct_literal`,
which prevents `if score {` from being parsed as a struct initializer.

## Helper methods

The parser's internals revolve around a small set of helpers that every parse function uses:

| Method | Does |
|--------|------|
| `peek_kind()` | Returns the current token's kind without consuming it |
| `advance()` | Returns the current token and advances `pos` by 1 |
| `check(&kind)` | Returns `true` if current token matches kind (no consume) |
| `match_token(&kind)` | Consumes and returns `true` if current token matches |
| `expect(kind)` | Consumes and errors if current token does not match |
| `expect_ident()` | Consumes and returns the name if current token is `Ident` |
| `current_span()` | Returns the current token's span |
| `prev_span()` | Returns the previous token's span |
| `is_at_end()` | Returns `true` if current token is `Eof` |

Every parse function is built from these primitives. There is no backtracking, no
lookahead beyond one token (except in a few disambiguation cases).

## `parse_program` — the entry point

```rust
fn parse_program(&mut self) -> Result<Program, PipelineError> {
    let start = self.current_span();
    let mut items = Vec::new();

    while !self.is_at_end() {
        items.push(self.parse_item()?);
    }

    // Append any items hoisted out of function bodies
    items.extend(self.ctx.hoisted_items.drain(..));

    Ok(Program { items, span: self.merge_spans(start, self.prev_span()) })
}
```

Loop, parse one item at a time, stop at EOF. After the loop, any items extracted from
function bodies (`Stmt::Item`) are appended to the program with synthesized names.

## `parse_item` — dispatching on keyword

```rust
fn parse_item(&mut self) -> Result<Item, PipelineError> {
    let vis = self.parse_visibility();
    let attrs = self.parse_attributes();

    match self.peek_kind() {
        TokenKind::Fn | TokenKind::Async => Ok(Item::Function(self.parse_fn(vis, attrs)?)),
        TokenKind::Struct => Ok(Item::Struct(self.parse_struct(vis, attrs)?)),
        TokenKind::Enum => Ok(Item::Enum(self.parse_enum(vis, attrs)?)),
        TokenKind::Impl => self.parse_impl(vis),
        TokenKind::Trait => Ok(Item::Trait(self.parse_trait(vis)?)),
        TokenKind::Mod => Ok(Item::Module(self.parse_module(vis)?)),
        TokenKind::Use => Ok(Item::Use(self.parse_use(vis)?)),
        TokenKind::Type => Ok(self.parse_type_alias(vis)?),
        TokenKind::Const | TokenKind::Static => Ok(self.parse_const(vis)?),
        _ => Err(self.expected_error("item")),
    }
}
```

Visibility and attributes are parsed first (they can appear before any item), then the
keyword determines which parse function to call.

## `parse_fn` — a function definition

```rust
fn parse_fn(&mut self, vis: Visibility, attrs: Vec<Attribute>) -> Result<FnDef, PipelineError> {
    let start = self.current_span();
    let is_async = self.match_token(&TokenKind::Async);
    self.expect(TokenKind::Fn)?;
    let name = self.expect_ident()?;
    let generic_params = self.parse_generic_params()?;

    self.expect(TokenKind::LParen)?;
    let params = self.parse_params()?;
    self.expect(TokenKind::RParen)?;

    let return_type = if self.match_token(&TokenKind::Arrow) {
        Some(self.parse_type()?)
    } else {
        None
    };

    let body = self.parse_block()?;
    Ok(FnDef { name, is_async, generic_params, params, return_type, body, attributes: attrs, visibility: vis, span: ... })
}
```

The structure mirrors the syntax: `[async] fn name<T>(params) -> ReturnType { body }`.
Each `expect()` call consumes a required token and errors with a clear message if it is
not there. `match_token()` optionally consumes tokens that may or may not be present.

## `parse_stmt` — statement dispatch

```rust
fn parse_stmt(&mut self) -> Result<Stmt, PipelineError> {
    match self.peek_kind() {
        TokenKind::Let => self.parse_let(),
        TokenKind::Return => self.parse_return(),
        TokenKind::While => self.parse_while(),
        TokenKind::Loop => self.parse_loop(),
        TokenKind::For => self.parse_for(),
        TokenKind::Break => self.parse_break(),
        TokenKind::Continue => self.parse_continue(),
        TokenKind::Use => Ok(Stmt::Use(self.parse_use(Visibility::Private)?)),

        // Nested item declarations inside function bodies
        TokenKind::Fn | TokenKind::Struct | TokenKind::Enum | ... => {
            let item = self.parse_item()?;
            // Hoist to top level with mangled name, add local alias
            self.hoist_nested_item(item)
        }

        // Everything else: expression statement
        _ => {
            let expr = self.parse_expr(Precedence::None)?;
            let has_semicolon = self.match_token(&TokenKind::Semicolon);
            Ok(Stmt::Expr { expr, has_semicolon })
        }
    }
}
```

The "everything else is an expression" catch-all is what allows Oxy to use expressions as
statements, return values from blocks, and write things like `let x = if cond { 1 } else { 2 }`.

## `parse_let` — variable bindings

```rust
fn parse_let(&mut self) -> Result<Stmt, PipelineError> {
    let start = self.current_span();
    self.expect(TokenKind::Let)?;
    let mutable = self.match_token(&TokenKind::Mut);

    // Check for pattern: `let Some(x) = ...`
    if /* pattern token */ {
        let pattern = self.parse_pattern()?;
        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr(Precedence::None)?;
        return Ok(Stmt::LetPattern { pattern: Box::new(pattern), mutable, value, span: ... });
    }

    let name = self.expect_ident()?;
    let type_ann = if self.match_token(&TokenKind::Colon) {
        Some(self.parse_type()?)
    } else {
        None
    };
    let value = if self.match_token(&TokenKind::Eq) {
        Some(self.parse_expr(Precedence::None)?)
    } else {
        None
    };
    self.expect(TokenKind::Semicolon)?;
    Ok(Stmt::Let { name, mutable, type_ann, value, span: ... })
}
```

Both `let x = 5` and `let Some(x) = maybe_value` are handled here. The parser peeks at
the token after `let [mut]` to decide whether it is a pattern or a plain identifier.

## Error messages

When the parser encounters an unexpected token, it calls `expected_error`:

```rust
fn expected_error(&self, expected: &str) -> PipelineError {
    let tok = &self.tokens[self.pos.min(self.tokens.len() - 1)];
    PipelineError::Parser {
        message: format!(
            "expected {expected}, found {}",
            tok.kind.description()
        ),
        line: tok.span.line,
        column: tok.span.column,
    }
}
```

The token's span provides the line/column. The token's `description()` provides the human-
readable name ("'fn'", "integer literal", "identifier"). The result is error messages like:

```
error: expected identifier, found '('
  at line 3, column 5
```
