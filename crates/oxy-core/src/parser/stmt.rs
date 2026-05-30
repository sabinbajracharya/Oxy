use super::*;

impl Parser {
    // === Block parsing ===

    pub(super) fn parse_block(&mut self) -> Result<Block, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::LBrace)?;

        let mut stmts = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            stmts.push(self.parse_stmt()?);
        }

        let end_span = self.current_span();
        self.expect(TokenKind::RBrace)?;

        Ok(Block {
            stmts,
            span: self.merge_spans(start_span, end_span),
        })
    }

    // === Statement parsing ===

    pub(super) fn parse_stmt(&mut self) -> Result<Stmt, PipelineError> {
        // Check for 'label: while/loop/for
        if let Some(label) = self.try_parse_loop_label() {
            let label = Some(label);
            return match self.peek_kind() {
                TokenKind::While => self.parse_while_stmt(label.clone()),
                TokenKind::Loop => self.parse_loop_stmt(label.clone()),
                TokenKind::For => self.parse_for_stmt(label),
                _ => Err(self.error(format!(
                    "expected `while`, `loop`, or `for` after label, found {}",
                    self.peek_kind().description()
                ))),
            };
        }

        match self.peek_kind() {
            TokenKind::Let => self.parse_let_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::While => self.parse_while_stmt(None),
            TokenKind::Loop => self.parse_loop_stmt(None),
            TokenKind::For => self.parse_for_stmt(None),
            TokenKind::Break => self.parse_break_stmt(),
            TokenKind::Continue => self.parse_continue_stmt(),
            TokenKind::Use => {
                let use_def = self.parse_use_def(Visibility::Private)?;
                Ok(Stmt::Use(use_def))
            }
            // Nested items: `fn`, `async fn`, `struct`, `enum` inside a
            // function body. Hoist the item to top-level with a mangled name
            // based on the enclosing fn stack (e.g. `fn outer() { fn inner()
            // {} }` → top-level `outer__inner`), and leave a `Stmt::Use` in
            // place to alias the original name into the body's scope.
            TokenKind::Fn | TokenKind::Async | TokenKind::Struct | TokenKind::Enum => {
                let span = self.current_span();
                let item = self.parse_item()?;
                let original_name = item_name(&item).to_string();
                if self.ctx.fn_name_stack.is_empty() {
                    // Not actually nested — keep as a regular Stmt::Item so
                    // top-level callers (e.g. tests that call parse_stmt
                    // directly) still get the AST node. Production callers
                    // never hit this branch because parse_stmt is only
                    // invoked inside a fn body.
                    return Ok(Stmt::Item(Box::new(item)));
                }
                let prefix = self.ctx.fn_name_stack.join("__");
                let mangled = format!("{}__{}", prefix, original_name);
                let renamed = rename_item(item, mangled.clone());
                self.ctx.hoisted_items.push(renamed);
                Ok(Stmt::Use(UseDef {
                    path: vec![mangled],
                    tree: UseTree::Simple(Some(original_name)),
                    visibility: Visibility::Private,
                    span,
                }))
            }
            _ => self.parse_expr_stmt(),
        }
    }

    /// If current token is `Label(name)` followed by `:`, consume both and return `Some(name)`.
    fn try_parse_loop_label(&mut self) -> Option<String> {
        if let TokenKind::Label(name) = self.peek_kind() {
            let label = name.clone();
            if self.pos + 1 < self.tokens.len()
                && self.tokens[self.pos + 1].kind == TokenKind::Colon
            {
                self.advance(); // label
                self.advance(); // ':'
                return Some(label);
            }
        }
        None
    }

    fn parse_let_stmt(&mut self) -> Result<Stmt, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Let)?;

        let mutable = self.match_token(&TokenKind::Mut);

        // Check for destructuring: `let (x, y) = ...` or `let [a, b] = ...`
        if self.check(&TokenKind::LParen) || self.check(&TokenKind::LBracket) {
            let pattern = self.parse_pattern()?;
            self.expect(TokenKind::Eq)?;
            let value = self.parse_expr(Precedence::None)?;
            let end_span = self.current_span();
            self.expect(TokenKind::Semicolon)?;
            return Ok(Stmt::LetPattern {
                pattern: Box::new(pattern),
                mutable,
                value,
                span: self.merge_spans(start_span, end_span),
            });
        }

        let name = if self.match_token(&TokenKind::Underscore) {
            "_".to_string()
        } else {
            self.expect_ident()?
        };

        // Check for struct destructuring: `let Name { x, y } = ...`
        if self.check(&TokenKind::LBrace) {
            let struct_pattern = self.parse_struct_pattern(name, start_span)?;
            self.expect(TokenKind::Eq)?;
            let value = self.parse_expr(Precedence::None)?;
            let end_span = self.current_span();
            self.expect(TokenKind::Semicolon)?;
            return Ok(Stmt::LetPattern {
                pattern: Box::new(struct_pattern),
                mutable,
                value,
                span: self.merge_spans(start_span, end_span),
            });
        }

        let type_ann = if self.match_token(&TokenKind::Colon) {
            // Accept optional & or &mut in type position
            if self.check(&TokenKind::Amp) {
                self.advance();
                if self.check(&TokenKind::Mut) {
                    self.advance();
                }
            }
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let value = if self.match_token(&TokenKind::Eq) {
            Some(self.parse_expr(Precedence::None)?)
        } else {
            None
        };

        let end_span = self.current_span();
        self.expect(TokenKind::Semicolon)?;

        Ok(Stmt::Let {
            name,
            mutable,
            type_ann,
            value,
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Return)?;

        let value = if !self.check(&TokenKind::Semicolon) && !self.check(&TokenKind::RBrace) {
            Some(self.parse_expr(Precedence::None)?)
        } else {
            None
        };

        let end_span = self.current_span();
        // Semicolon is optional if at end of block
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }

        Ok(Stmt::Return {
            value,
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_while_stmt(&mut self, label: Option<String>) -> Result<Stmt, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::While)?;

        // Check for `while let`
        if self.check(&TokenKind::Let) {
            self.advance();
            let pattern = self.parse_pattern()?;
            self.expect(TokenKind::Eq)?;
            let expr = self.with_no_struct_literal(|p| p.parse_expr(Precedence::None))?;
            let body = self.parse_block()?;
            let end_span = body.span;
            return Ok(Stmt::WhileLet {
                label,
                pattern: Box::new(pattern),
                expr: Box::new(expr),
                body,
                span: self.merge_spans(start_span, end_span),
            });
        }

        let condition = self.with_no_struct_literal(|p| p.parse_expr(Precedence::None))?;
        let body = self.parse_block()?;
        let end_span = body.span;

        Ok(Stmt::While {
            label,
            condition: Box::new(condition),
            body,
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_loop_stmt(&mut self, label: Option<String>) -> Result<Stmt, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Loop)?;

        let body = self.parse_block()?;
        let end_span = body.span;

        Ok(Stmt::Loop {
            label,
            body,
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_for_stmt(&mut self, label: Option<String>) -> Result<Stmt, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::For)?;

        // Check for tuple destructuring: `for (a, b) in ...`
        if self.check(&TokenKind::LParen) {
            self.advance();
            let mut names = Vec::new();
            loop {
                if self.check(&TokenKind::Underscore) {
                    self.advance();
                    names.push("_".to_string());
                } else {
                    names.push(self.expect_ident()?);
                }
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
                if self.check(&TokenKind::RParen) {
                    break;
                }
            }
            self.expect(TokenKind::RParen)?;
            self.expect(TokenKind::In)?;
            let iterable = self.with_no_struct_literal(|p| p.parse_expr(Precedence::None))?;
            let body = self.parse_block()?;
            let end_span = body.span;
            return Ok(Stmt::ForDestructure {
                label,
                names,
                iterable: Box::new(iterable),
                body,
                span: self.merge_spans(start_span, end_span),
            });
        }

        let name = self.expect_ident()?;
        self.expect(TokenKind::In)?;

        let iterable = self.with_no_struct_literal(|p| p.parse_expr(Precedence::None))?;
        let body = self.parse_block()?;
        let end_span = body.span;

        Ok(Stmt::For {
            label,
            name,
            iterable: Box::new(iterable),
            body,
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_break_stmt(&mut self) -> Result<Stmt, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Break)?;

        // Check for labeled break: `break 'label`
        let label = if let TokenKind::Label(name) = self.peek_kind() {
            let n = name.clone();
            self.advance();
            Some(n)
        } else {
            None
        };

        let value = if !self.check(&TokenKind::Semicolon) && !self.check(&TokenKind::RBrace) {
            Some(Box::new(self.parse_expr(Precedence::None)?))
        } else {
            None
        };

        let end_span = self.current_span();
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }

        Ok(Stmt::Break {
            label,
            value,
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_continue_stmt(&mut self) -> Result<Stmt, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Continue)?;

        // Check for labeled continue: `continue 'label`
        let label = if let TokenKind::Label(name) = self.peek_kind() {
            let n = name.clone();
            self.advance();
            Some(n)
        } else {
            None
        };

        let end_span = self.current_span();
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }

        Ok(Stmt::Continue {
            label,
            span: self.merge_spans(start_span, end_span),
        })
    }

    pub(super) fn parse_expr_stmt(&mut self) -> Result<Stmt, PipelineError> {
        // Expression-with-block (`if`, `match`, `{ ... }`) at statement
        // position is a self-contained statement — it does NOT chain infix
        // operators with the following tokens. This matches Rust:
        //
        //     if c { return 1; }
        //     -1                          // separate trailing expression
        //
        // is two statements, not `if-expr - 1`. To use such an expression
        // as the LHS of an operator, wrap it in parentheses.
        let starts_block = matches!(
            self.peek_kind(),
            TokenKind::If | TokenKind::Match | TokenKind::LBrace
        );
        let expr = if starts_block {
            self.parse_prefix()?
        } else {
            self.parse_expr(Precedence::None)?
        };

        let has_semicolon = self.match_token(&TokenKind::Semicolon);

        Ok(Stmt::Expr {
            expr,
            has_semicolon,
        })
    }
}
