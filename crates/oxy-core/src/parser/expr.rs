use super::*;

impl Parser {
    fn parse_struct_init(&mut self, name: String, start_span: Span) -> Result<Expr, PipelineError> {
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        let mut base = None;
        while !self.check(&TokenKind::RBrace) {
            // `..base_expr` — must be last item
            if self.match_token(&TokenKind::DotDot) {
                base = Some(Box::new(self.parse_expr(Precedence::None)?));
                self.match_token(&TokenKind::Comma);
                break;
            }
            // Accept both identifiers (named fields) and integers (tuple struct fields)
            let field_name = if let TokenKind::IntLiteral(n, _) = self.peek_kind() {
                let name = n.to_string();
                self.advance();
                name
            } else {
                self.expect_ident()?
            };
            // Shorthand: `Point { x, y }` where x and y are variables
            let value = if self.match_token(&TokenKind::Colon) {
                self.parse_expr(Precedence::None)?
            } else {
                Expr::Ident(field_name.clone(), self.prev_span())
            };
            fields.push((field_name, value));
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        let end_span = self.current_span();
        self.expect(TokenKind::RBrace)?;
        Ok(Expr::StructInit {
            name,
            fields,
            base,
            span: self.merge_spans(start_span, end_span),
        })
    }

    // === Expression parsing (Pratt / precedence climbing) ===

    pub(super) fn parse_expr(&mut self, min_prec: Precedence) -> Result<Expr, PipelineError> {
        let mut left = self.parse_prefix()?;

        while !self.is_at_end() {
            let prec = Precedence::of_binary(self.peek_kind());
            if prec <= min_prec {
                break;
            }

            left = self.parse_infix(left, prec)?;
        }

        Ok(left)
    }

    pub(super) fn parse_prefix(&mut self) -> Result<Expr, PipelineError> {
        match self.peek_kind().clone() {
            // Literals
            TokenKind::IntLiteral(n, suffix) => {
                let span = self.current_span();
                self.advance();
                Ok(Expr::IntLiteral(n, suffix, span))
            }
            TokenKind::FloatLiteral(n, suffix) => {
                let span = self.current_span();
                self.advance();
                Ok(Expr::FloatLiteral(n, suffix, span))
            }
            TokenKind::True => {
                let span = self.current_span();
                self.advance();
                Ok(Expr::BoolLiteral(true, span))
            }
            TokenKind::False => {
                let span = self.current_span();
                self.advance();
                Ok(Expr::BoolLiteral(false, span))
            }
            TokenKind::StringLiteral(s) => {
                let span = self.current_span();
                self.advance();
                Ok(Expr::StringLiteral(s, span))
            }
            TokenKind::FStringLiteral(raw) => {
                let span = self.current_span();
                self.advance();
                let parts = self.parse_fstring_parts(&raw, span)?;
                Ok(Expr::FString { parts, span })
            }
            TokenKind::CharLiteral(c) => {
                let span = self.current_span();
                self.advance();
                Ok(Expr::CharLiteral(c, span))
            }

            // `self` keyword — could be self value or `self::path`
            TokenKind::SelfLower => {
                let span = self.current_span();
                self.advance();

                if self.check(&TokenKind::ColonColon) {
                    self.advance();
                    return self.finish_keyword_path("self", span);
                }

                Ok(Expr::SelfRef(span))
            }

            // `super` keyword — `super::path` in expression position
            TokenKind::Super => {
                let span = self.current_span();
                self.advance();
                self.expect(TokenKind::ColonColon)?;
                self.finish_keyword_path("super", span)
            }

            // `crate` keyword — `crate::path` in expression position
            TokenKind::Crate => {
                let span = self.current_span();
                self.advance();
                self.expect(TokenKind::ColonColon)?;
                self.finish_keyword_path("crate", span)
            }

            // Identifiers (could be followed by `::` for path, `{` for struct init)
            TokenKind::Ident(_) | TokenKind::SelfUpper => {
                let span = self.current_span();
                let name = if self.check(&TokenKind::SelfUpper) {
                    self.advance();
                    "Self".to_string()
                } else {
                    self.expect_ident()?
                };

                // `name!(...)` or `name![...]` — macro syntax is no longer supported.
                // Oxy uses regular function calls instead.
                if self.check(&TokenKind::Bang) {
                    return Err(PipelineError::Parser {
                        message: format!(
                            "macro syntax `!` is no longer supported. Use `{name}(...)` \
                             instead of `{name}!(...)`",
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }

                // Check for path: `Name::...`
                if self.check(&TokenKind::ColonColon) {
                    self.advance();

                    let mut turbofish = None;
                    // Turbofish: `foo::<T>(args)`, `Foo::<T> { ... }`, `Vec::<T>::new()`, `Foo::<T>`
                    if self.check(&TokenKind::Lt) {
                        turbofish = Some(self.parse_turbofish()?);
                        // `foo::<T>(args)` — call with turbofish
                        if self.check(&TokenKind::LParen) {
                            self.advance();
                            let args = self.parse_arg_list()?;
                            let end_span = self.current_span();
                            self.expect(TokenKind::RParen)?;
                            return Ok(Expr::Call {
                                callee: Box::new(Expr::Ident(name.clone(), span)),
                                turbofish,
                                args,
                                span: self.merge_spans(span, end_span),
                            });
                        }
                        // `Foo::<T> { field: val }` — struct init with turbofish
                        if self.check(&TokenKind::LBrace) && !self.ctx.no_struct_literal {
                            return self.parse_struct_init(name, span);
                        }
                        // No `::` after turbofish → `Foo::<T>` (unit/tuple struct)
                        if !self.check(&TokenKind::ColonColon) {
                            return Ok(Expr::Ident(name, span));
                        }
                        // Otherwise, `::` follows → fall through to path loop
                    }

                    let mut segments = vec![name];
                    if turbofish.is_none() {
                        segments.push(self.expect_path_segment()?);
                    }

                    self.parse_path_segments(&mut segments)?;

                    // Check if followed by `(` → PathCall
                    if self.check(&TokenKind::LParen) {
                        self.advance();
                        let args = self.parse_arg_list()?;
                        let end_span = self.current_span();
                        self.expect(TokenKind::RParen)?;
                        return Ok(Expr::PathCall {
                            path: segments,
                            turbofish,
                            args,
                            span: self.merge_spans(span, end_span),
                        });
                    }

                    // Check if followed by `{` → struct init with path (e.g. module::Struct { })
                    // Skip in no-struct-literal contexts (if/while/for headers).
                    if self.check(&TokenKind::LBrace) && !self.ctx.no_struct_literal {
                        let full_name = segments.join("::");
                        return self.parse_struct_init(full_name, span);
                    }
                    let end_span = self.prev_span();
                    return Ok(Expr::Path {
                        segments,
                        span: self.merge_spans(span, end_span),
                    });
                }

                // Check for struct init: `Name { field: value, ... }`
                // Only if name starts with uppercase (convention for type names)
                // AND we're not in a no-struct-literal context like an `if`,
                // `while`, or `for` header — otherwise
                // `if score < MAX_SIZE { ... }` would treat `MAX_SIZE { ... }`
                // as a struct literal and consume the if-body.
                if self.check(&TokenKind::LBrace)
                    && name.starts_with(|c: char| c.is_uppercase())
                    && !self.ctx.no_struct_literal
                {
                    return self.parse_struct_init(name, span);
                }

                Ok(Expr::Ident(name, span))
            }

            // Grouped expression `(expr)` or tuple `(a, b, c)`
            TokenKind::LParen => {
                let start_span = self.current_span();
                self.advance();

                // Empty tuple: `()`
                if self.check(&TokenKind::RParen) {
                    let end_span = self.current_span();
                    self.advance();
                    return Ok(Expr::Tuple {
                        elements: Vec::new(),
                        span: self.merge_spans(start_span, end_span),
                    });
                }

                let first = self.parse_expr(Precedence::None)?;

                // Check for comma → tuple
                if self.check(&TokenKind::Comma) {
                    let mut elements = vec![first];
                    while self.match_token(&TokenKind::Comma) {
                        if self.check(&TokenKind::RParen) {
                            break; // trailing comma
                        }
                        elements.push(self.parse_expr(Precedence::None)?);
                    }
                    let end_span = self.current_span();
                    self.expect(TokenKind::RParen)?;
                    return Ok(Expr::Tuple {
                        elements,
                        span: self.merge_spans(start_span, end_span),
                    });
                }

                // Single expression → grouped
                let end_span = self.current_span();
                self.expect(TokenKind::RParen)?;
                Ok(Expr::Grouped(
                    Box::new(first),
                    self.merge_spans(start_span, end_span),
                ))
            }

            // Block expression: `{ ... }`
            TokenKind::LBrace => {
                let block = self.parse_block()?;
                Ok(Expr::Block(block))
            }

            // Array literal: `[1, 2, 3]`
            TokenKind::LBracket => {
                let start_span = self.current_span();
                self.advance();
                if self.check(&TokenKind::RBracket) {
                    let end_span = self.current_span();
                    self.advance(); // ]
                    return Ok(Expr::Array {
                        elements: vec![],
                        span: self.merge_spans(start_span, end_span),
                    });
                }
                let first = self.parse_expr(Precedence::None)?;
                // Check for `[expr; N]` repeat expression
                if self.match_token(&TokenKind::Semicolon) {
                    let count = self.parse_expr(Precedence::None)?;
                    let end_span = self.current_span();
                    self.expect(TokenKind::RBracket)?;
                    return Ok(Expr::Repeat {
                        value: Box::new(first),
                        count: Box::new(count),
                        span: self.merge_spans(start_span, end_span),
                    });
                }
                let mut elements = vec![first];
                loop {
                    // Match a comma — if none, we're done
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                    // Allow trailing comma
                    if self.check(&TokenKind::RBracket) {
                        break;
                    }
                    elements.push(self.parse_expr(Precedence::None)?);
                }
                let end_span = self.current_span();
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::Array {
                    elements,
                    span: self.merge_spans(start_span, end_span),
                })
            }

            // If expression
            TokenKind::If => self.parse_if_expr(),

            // Match expression
            TokenKind::Match => self.parse_match_expr(),

            // Loop as expression (for `let x = loop { break val; };`)
            TokenKind::Loop => {
                let start_span = self.current_span();
                self.advance();
                let body = self.parse_block()?;
                let end_span = body.span;
                // Wrap as a block expression containing a Loop statement
                Ok(Expr::Block(Block {
                    stmts: vec![Stmt::Loop {
                        label: None,
                        body,
                        span: self.merge_spans(start_span, end_span),
                    }],
                    span: self.merge_spans(start_span, end_span),
                }))
            }

            // Return as expression (for `match x { 0 => return 42, _ => 0 }`)
            TokenKind::Return => {
                let span = self.current_span();
                self.advance();
                // Parse optional return value
                let value = if self.is_at_end()
                    || matches!(
                        self.peek_kind(),
                        TokenKind::Semicolon
                            | TokenKind::RBrace
                            | TokenKind::Comma
                            | TokenKind::RParen
                            | TokenKind::RBracket
                    ) {
                    None
                } else {
                    Some(Box::new(self.parse_expr(Precedence::None)?))
                };
                Ok(Expr::Return {
                    value,
                    span: self.merge_spans(span, self.current_span()),
                })
            }

            // Unary operators
            TokenKind::Minus => {
                let span = self.current_span();
                self.advance();
                let expr = self.parse_expr(Precedence::Unary)?;
                let end_span = expr.span();
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                    span: self.merge_spans(span, end_span),
                })
            }
            TokenKind::Bang => {
                let span = self.current_span();
                self.advance();
                let expr = self.parse_expr(Precedence::Unary)?;
                let end_span = expr.span();
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                    span: self.merge_spans(span, end_span),
                })
            }
            TokenKind::Tilde => {
                let span = self.current_span();
                self.advance();
                let expr = self.parse_expr(Precedence::Unary)?;
                let end_span = expr.span();
                Ok(Expr::UnaryOp {
                    op: UnaryOp::BitNot,
                    expr: Box::new(expr),
                    span: self.merge_spans(span, end_span),
                })
            }
            TokenKind::Amp => Err(self.error(
                "the `&` prefix operator is not supported in Oxy. Just pass the \
                 value directly — there's no borrow checker. (For bitwise AND on \
                 two values, use `a & b` as a binary operator.) See CLAUDE.md."
                    .to_string(),
            )),

            // Closure: `|params| expr` or `|params| { body }`
            TokenKind::Pipe => self.parse_closure(false),

            // Closure with no params: `|| expr` or `|| { body }`
            TokenKind::PipePipe => self.parse_empty_closure(false),

            // Prefix range: `..end` or `..=end`
            TokenKind::DotDot | TokenKind::DotDotEq => {
                let span = self.current_span();
                let inclusive = matches!(self.peek_kind(), TokenKind::DotDotEq);
                self.advance();
                let end = if self.check(&TokenKind::RBracket)
                    || self.check(&TokenKind::RParen)
                    || self.check(&TokenKind::Semicolon)
                {
                    None
                } else {
                    Some(Box::new(self.parse_expr(Precedence::Range)?))
                };
                let end_span = end.as_ref().map(|e| e.span()).unwrap_or(span);
                Ok(Expr::Range {
                    start: None,
                    end,
                    inclusive,
                    span: self.merge_spans(span, end_span),
                })
            }

            // `async` closure: `async || expr`, `async |params| expr`, `async { ... }`
            TokenKind::Async => {
                let start_span = self.current_span();
                self.advance(); // consume `async`
                if self.check(&TokenKind::PipePipe) {
                    self.parse_empty_closure(true)
                } else if self.check(&TokenKind::Pipe) {
                    self.parse_closure(true)
                } else if self.check(&TokenKind::LBrace) {
                    let block = self.parse_block()?;
                    let end_span = block.span;
                    Ok(Expr::AsyncBlock {
                        body: block,
                        span: self.merge_spans(start_span, end_span),
                    })
                } else {
                    Err(self.error("expected `||`, `|`, or `{` after `async`".into()))
                }
            }

            other => Err(self.error(format!(
                "expected expression, found {}",
                other.description()
            ))),
        }
    }

    fn parse_infix(&mut self, left: Expr, prec: Precedence) -> Result<Expr, PipelineError> {
        let op_span = self.current_span();
        let op_kind = self.peek_kind().clone();

        // Assignment operators
        if matches!(
            op_kind,
            TokenKind::Eq
                | TokenKind::PlusEq
                | TokenKind::MinusEq
                | TokenKind::StarEq
                | TokenKind::SlashEq
                | TokenKind::PercentEq
        ) {
            self.advance();
            // Right-associative: parse with same precedence
            let right = self.parse_expr(Precedence::None)?;
            let span = self.merge_spans(left.span(), right.span());

            return if op_kind == TokenKind::Eq {
                Ok(Expr::Assign {
                    target: Box::new(left),
                    value: Box::new(right),
                    span,
                })
            } else {
                let bin_op = match op_kind {
                    TokenKind::PlusEq => BinOp::Add,
                    TokenKind::MinusEq => BinOp::Sub,
                    TokenKind::StarEq => BinOp::Mul,
                    TokenKind::SlashEq => BinOp::Div,
                    TokenKind::PercentEq => BinOp::Mod,
                    _ => unreachable!(),
                };
                Ok(Expr::CompoundAssign {
                    target: Box::new(left),
                    op: bin_op,
                    value: Box::new(right),
                    span,
                })
            };
        }

        // Range operators: `..` and `..=`
        if matches!(op_kind, TokenKind::DotDot | TokenKind::DotDotEq) {
            let inclusive = op_kind == TokenKind::DotDotEq;
            self.advance();
            // Check if end is omitted (e.g., `start..` followed by `]` or `)` or `{` or `;`)
            let end = if self.check(&TokenKind::RBracket)
                || self.check(&TokenKind::RParen)
                || self.check(&TokenKind::LBrace)
                || self.check(&TokenKind::Semicolon)
                || self.check(&TokenKind::Comma)
            {
                None
            } else {
                Some(Box::new(self.parse_expr(prec)?))
            };
            let end_span = end
                .as_ref()
                .map(|e| e.span())
                .unwrap_or_else(|| left.span());
            let span = self.merge_spans(left.span(), end_span);
            return Ok(Expr::Range {
                start: Some(Box::new(left)),
                end,
                inclusive,
                span,
            });
        }

        // Pipeline operator: `x |> f(args)` desugars to `f(x, args)`
        if op_kind == TokenKind::PipeArrow {
            self.advance();
            let right = self.parse_expr(prec)?;
            let span = self.merge_spans(left.span(), right.span());
            return Self::desugar_pipeline(left, right, span);
        }

        // Binary operators
        if let Some(bin_op) = Self::token_to_binop(&op_kind) {
            self.advance();
            let right = self.parse_expr(prec)?;
            let span = self.merge_spans(left.span(), right.span());
            return Ok(Expr::BinaryOp {
                left: Box::new(left),
                op: bin_op,
                right: Box::new(right),
                span,
            });
        }

        // Function call: `expr(...)`
        if op_kind == TokenKind::LParen {
            self.advance();
            let args = self.parse_arg_list()?;
            let end_span = self.current_span();
            self.expect(TokenKind::RParen)?;
            return Ok(Expr::Call {
                callee: Box::new(left),
                turbofish: None,
                args,
                span: self.merge_spans(op_span, end_span),
            });
        }

        // Index: `expr[index]`
        if op_kind == TokenKind::LBracket {
            self.advance();
            let index = self.parse_expr(Precedence::None)?;
            let end_span = self.current_span();
            self.expect(TokenKind::RBracket)?;
            return Ok(Expr::Index {
                object: Box::new(left),
                index: Box::new(index),
                span: self.merge_spans(op_span, end_span),
            });
        }

        // Dot: `.method()`, `.field`, `.0`, `.await`
        if op_kind == TokenKind::Dot {
            self.advance();

            // Check for `.await`
            if self.check(&TokenKind::Await) {
                let end_span = self.current_span();
                self.advance();
                return Ok(Expr::Await {
                    expr: Box::new(left),
                    span: self.merge_spans(op_span, end_span),
                });
            }

            // Check for tuple index: `.0`, `.1` etc.
            if let TokenKind::IntLiteral(n, _) = self.peek_kind() {
                let n = *n;
                let end_span = self.current_span();
                self.advance();
                return Ok(Expr::FieldAccess {
                    object: Box::new(left),
                    field: n.to_string(),
                    span: self.merge_spans(op_span, end_span),
                });
            }

            let name = self.expect_ident()?;

            // Check for turbofish: `.method::<Type>(args)`
            let turbofish = if self.check(&TokenKind::ColonColon) {
                self.advance();
                if self.check(&TokenKind::Lt) {
                    Some(self.parse_turbofish()?)
                } else {
                    // It's a `::` without `<` — error or path, bail
                    return Err(self.error("expected identifier after `::` in method chain".into()));
                }
            } else {
                None
            };

            // Check for method call: `.name(...)`
            if self.check(&TokenKind::LParen) {
                self.advance();
                let args = self.parse_arg_list()?;
                let end_span = self.current_span();
                self.expect(TokenKind::RParen)?;
                return Ok(Expr::MethodCall {
                    object: Box::new(left),
                    method: name,
                    turbofish,
                    args,
                    span: self.merge_spans(op_span, end_span),
                });
            }

            // Otherwise field access: `.name`
            let end_span = self.current_span();
            return Ok(Expr::FieldAccess {
                object: Box::new(left),
                field: name,
                span: self.merge_spans(op_span, end_span),
            });
        }

        // Try operator: `expr?`
        if op_kind == TokenKind::Question {
            self.advance();
            return Ok(Expr::Try {
                expr: Box::new(left),
                span: op_span,
            });
        }

        // expr as Type
        if op_kind == TokenKind::As {
            self.advance();
            let type_name = self.expect_ident()?;
            return Ok(Expr::As {
                expr: Box::new(left),
                type_name,
                span: self.merge_spans(op_span, self.current_span()),
            });
        }

        Err(self.error(format!(
            "unexpected token in expression: {}",
            op_kind.description()
        )))
    }

    fn parse_if_expr(&mut self) -> Result<Expr, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::If)?;

        // Check for `if let` pattern
        if self.check(&TokenKind::Let) {
            return self.parse_if_let_expr(start_span);
        }

        let condition = self.with_no_struct_literal(|p| p.parse_expr(Precedence::None))?;
        let then_block = self.parse_block()?;

        let else_block = if self.match_token(&TokenKind::Else) {
            if self.check(&TokenKind::If) {
                // `else if` chain
                Some(Box::new(self.parse_if_expr()?))
            } else {
                // `else { ... }`
                let block = self.parse_block()?;
                Some(Box::new(Expr::Block(block)))
            }
        } else {
            None
        };

        let end_span = else_block
            .as_ref()
            .map(|e| e.span())
            .unwrap_or(then_block.span);

        Ok(Expr::If {
            condition: Box::new(condition),
            then_block,
            else_block,
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_if_let_expr(&mut self, start_span: Span) -> Result<Expr, PipelineError> {
        self.expect(TokenKind::Let)?;
        let pattern = self.parse_pattern()?;
        self.expect(TokenKind::Eq)?;
        // Stop at `&&` so it can separate scrutinee from optional guard condition.
        let expr = self.with_no_struct_literal(|p| p.parse_expr(Precedence::And))?;
        // Optional `&& guard_condition` — the bound pattern variables are in scope.
        let guard = if self.match_token(&TokenKind::AmpAmp) {
            Some(Box::new(self.with_no_struct_literal(|p| {
                p.parse_expr(Precedence::None)
            })?))
        } else {
            None
        };
        let then_block = self.parse_block()?;

        let else_block = if self.match_token(&TokenKind::Else) {
            if self.check(&TokenKind::If) {
                Some(Box::new(self.parse_if_expr()?))
            } else {
                let block = self.parse_block()?;
                Some(Box::new(Expr::Block(block)))
            }
        } else {
            None
        };

        let end_span = else_block
            .as_ref()
            .map(|e| e.span())
            .unwrap_or(then_block.span);

        Ok(Expr::IfLet {
            pattern: Box::new(pattern),
            expr: Box::new(expr),
            guard,
            then_block,
            else_block,
            span: self.merge_spans(start_span, end_span),
        })
    }

    /// Parse a closure expression: `|params| expr` or `|params| { body }`
    fn parse_closure(&mut self, is_async: bool) -> Result<Expr, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Pipe)?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::Pipe) {
            loop {
                let pspan = self.current_span();
                let name = if self.match_token(&TokenKind::Underscore) {
                    "_".to_string()
                } else {
                    self.expect_ident()?
                };
                let type_ann = if self.match_token(&TokenKind::Colon) {
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };
                params.push(ClosureParam {
                    name,
                    type_ann,
                    span: pspan,
                });
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::Pipe)?;

        // Optional return type: `-> Type`
        let return_type = if self.match_token(&TokenKind::Arrow) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        // Body: either a block `{ ... }` or a single expression
        let body = if self.check(&TokenKind::LBrace) {
            let block = self.parse_block()?;
            Expr::Block(block)
        } else {
            self.parse_expr(Precedence::None)?
        };

        let end_span = body.span();
        Ok(Expr::Closure {
            params,
            return_type,
            body: Box::new(body),
            span: self.merge_spans(start_span, end_span),
            is_async,
        })
    }

    /// Parse a closure with no params: `|| expr` or `|| { body }`
    fn parse_empty_closure(&mut self, is_async: bool) -> Result<Expr, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::PipePipe)?;

        // Optional return type
        let return_type = if self.match_token(&TokenKind::Arrow) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let body = if self.check(&TokenKind::LBrace) {
            let block = self.parse_block()?;
            Expr::Block(block)
        } else {
            self.parse_expr(Precedence::None)?
        };

        let end_span = body.span();
        Ok(Expr::Closure {
            params: Vec::new(),
            return_type,
            body: Box::new(body),
            span: self.merge_spans(start_span, end_span),
            is_async,
        })
    }

    fn parse_match_expr(&mut self) -> Result<Expr, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Match)?;

        let expr = self.parse_expr(Precedence::None)?;
        self.expect(TokenKind::LBrace)?;

        let mut arms = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let arm_span = self.current_span();
            let pattern = self.parse_pattern()?;

            // Parse optional guard: `pattern if condition =>`
            let guard = if self.match_token(&TokenKind::If) {
                Some(Box::new(self.parse_expr(Precedence::None)?))
            } else {
                None
            };

            self.expect(TokenKind::FatArrow)?;

            // Arm body can be a block or a single expression
            let body = if self.check(&TokenKind::LBrace) {
                let block = self.parse_block()?;
                Expr::Block(block)
            } else {
                self.parse_expr(Precedence::None)?
            };

            let end_span = body.span();
            arms.push(MatchArm {
                pattern,
                guard,
                body,
                span: self.merge_spans(arm_span, end_span),
            });

            // Comma is optional after block bodies, required after expressions
            self.match_token(&TokenKind::Comma);
        }

        let end_span = self.current_span();
        self.expect(TokenKind::RBrace)?;

        Ok(Expr::Match {
            expr: Box::new(expr),
            arms,
            span: self.merge_spans(start_span, end_span),
        })
    }

    pub(super) fn parse_arg_list(&mut self) -> Result<Vec<Expr>, PipelineError> {
        self.parse_comma_separated(&[TokenKind::RParen, TokenKind::RBracket], |s| {
            s.parse_expr(Precedence::None)
        })
    }

    /// Parse turbofish: `::<Type1, Type2, ...>`.  Assumes `::` has just been consumed.
    /// Uses angle-bracket depth tracking to handle nested types like `Vec<i64>`.
    pub(super) fn parse_turbofish(&mut self) -> Result<Vec<TypeAnnotation>, PipelineError> {
        let mut depth: u32 = 1;
        let mut current_type = String::new();
        let mut type_start = self.current_span();
        let mut types: Vec<TypeAnnotation> = Vec::new();
        self.expect(TokenKind::Lt)?;

        loop {
            match self.peek_kind().clone() {
                TokenKind::Lt => {
                    self.advance();
                    depth += 1;
                    current_type.push_str("< ");
                }
                TokenKind::Gt => {
                    self.advance();
                    depth -= 1;
                    if depth == 0 {
                        if !current_type.trim().is_empty() {
                            types.push(TypeAnnotation::Named {
                                name: current_type.trim().to_string(),
                                generic_args: Vec::new(),
                                span: type_start,
                            });
                        }
                        break;
                    }
                    current_type.push_str("> ");
                }
                TokenKind::Shr => {
                    // `>>` = two `>` tokens. Process the first.
                    self.advance();
                    depth -= 1;
                    if depth == 0 {
                        if !current_type.trim().is_empty() {
                            types.push(TypeAnnotation::Named {
                                name: current_type.trim().to_string(),
                                generic_args: Vec::new(),
                                span: type_start,
                            });
                        }
                        break;
                    }
                    // There's a second `>` implied by Shr. Process it too.
                    depth -= 1;
                    if depth == 0 {
                        if !current_type.trim().is_empty() {
                            types.push(TypeAnnotation::Named {
                                name: current_type.trim().to_string(),
                                generic_args: Vec::new(),
                                span: type_start,
                            });
                        }
                        break;
                    }
                    current_type.push_str(">> ");
                }
                TokenKind::Comma => {
                    if depth == 1 {
                        self.advance();
                        if !current_type.trim().is_empty() {
                            types.push(TypeAnnotation::Named {
                                name: current_type.trim().to_string(),
                                generic_args: Vec::new(),
                                span: type_start,
                            });
                            current_type.clear();
                        }
                        type_start = self.current_span();
                    } else {
                        self.advance();
                        current_type.push_str(", ");
                    }
                }
                TokenKind::Ident(_) => {
                    let name = self.expect_ident()?;
                    if current_type.trim().is_empty() {
                        type_start = self.prev_span();
                    }
                    current_type.push_str(&name);
                    current_type.push(' ');
                }
                TokenKind::Eof => {
                    return Err(self.error("unterminated generic arguments".into()));
                }
                _ => {
                    current_type.push_str(&format!("{} ", self.peek_kind().description()));
                    self.advance();
                }
            }
        }
        Ok(types)
    }

    /// Parse `::`-separated path segments into `segments`, skipping any mid-path
    /// turbofish (`::<T>`). Caller must already have consumed the leading `::`
    /// (if any) and pushed the first segment into `segments`.
    fn parse_path_segments(&mut self, segments: &mut Vec<String>) -> Result<(), PipelineError> {
        while self.check(&TokenKind::ColonColon) {
            self.advance();
            if self.check(&TokenKind::Lt) {
                let _ = self.parse_turbofish()?;
                continue;
            }
            segments.push(self.expect_path_segment()?);
        }
        Ok(())
    }

    /// Finish parsing a keyword-rooted path (`self::...`, `super::...`, or
    /// `crate::...`). Caller must already have consumed the leading `::` after
    /// the keyword. Returns a `PathCall` when the path is followed by `(args)`,
    /// or `Ident(keyword, span)` otherwise.
    fn finish_keyword_path(&mut self, keyword: &str, span: Span) -> Result<Expr, PipelineError> {
        let mut segments = vec![keyword.to_string(), self.expect_path_segment()?];
        self.parse_path_segments(&mut segments)?;

        if self.check(&TokenKind::LParen) {
            self.advance();
            let args = self.parse_arg_list()?;
            let end_span = self.current_span();
            self.expect(TokenKind::RParen)?;
            return Ok(Expr::PathCall {
                path: segments,
                turbofish: None,
                args,
                span: self.merge_spans(span, end_span),
            });
        }
        Ok(Expr::Ident(keyword.to_string(), span))
    }

    fn token_to_binop(kind: &TokenKind) -> Option<BinOp> {
        match kind {
            TokenKind::Plus => Some(BinOp::Add),
            TokenKind::Minus => Some(BinOp::Sub),
            TokenKind::Star => Some(BinOp::Mul),
            TokenKind::Slash => Some(BinOp::Div),
            TokenKind::Percent => Some(BinOp::Mod),
            TokenKind::EqEq => Some(BinOp::Eq),
            TokenKind::BangEq => Some(BinOp::NotEq),
            TokenKind::Lt => Some(BinOp::Lt),
            TokenKind::Gt => Some(BinOp::Gt),
            TokenKind::LtEq => Some(BinOp::LtEq),
            TokenKind::GtEq => Some(BinOp::GtEq),
            TokenKind::AmpAmp => Some(BinOp::And),
            TokenKind::PipePipe => Some(BinOp::Or),
            TokenKind::Amp => Some(BinOp::BitAnd),
            TokenKind::Pipe => Some(BinOp::BitOr),
            TokenKind::Caret => Some(BinOp::BitXor),
            TokenKind::Shl => Some(BinOp::Shl),
            TokenKind::Shr => Some(BinOp::Shr),
            _ => None,
        }
    }

    /// Desugar `left |> right` into a function call.
    ///
    /// | Right side form        | Desugared to                     |
    /// |------------------------|----------------------------------|
    /// | `f(args...)`           | `f(left, args...)`               |
    /// | `y.method(args...)`    | `y.method(left, args...)`        |
    /// | `path::func(args...)`  | `path::func(left, args...)`      |
    /// | `f` (bare ident)       | `f(left)`                        |
    /// | `y.method` (field)     | `y.method(left)`                 |
    /// | `path::to::func` (path)| `path::to::func(left)`           |
    fn desugar_pipeline(left: Expr, right: Expr, span: Span) -> Result<Expr, PipelineError> {
        match right {
            Expr::Call {
                callee,
                turbofish,
                mut args,
                ..
            } => {
                args.insert(0, left);
                Ok(Expr::Call {
                    callee,
                    turbofish,
                    args,
                    span,
                })
            }
            Expr::MethodCall {
                object,
                method,
                turbofish,
                mut args,
                ..
            } => {
                args.insert(0, left);
                Ok(Expr::MethodCall {
                    object,
                    method,
                    turbofish,
                    args,
                    span,
                })
            }
            Expr::PathCall {
                path,
                turbofish,
                mut args,
                ..
            } => {
                args.insert(0, left);
                Ok(Expr::PathCall {
                    path,
                    turbofish,
                    args,
                    span,
                })
            }
            Expr::Ident(name, ident_span) => Ok(Expr::Call {
                callee: Box::new(Expr::Ident(name, ident_span)),
                turbofish: None,
                args: vec![left],
                span,
            }),
            Expr::FieldAccess { object, field, .. } => Ok(Expr::MethodCall {
                object,
                method: field,
                turbofish: None,
                args: vec![left],
                span,
            }),
            Expr::Path { segments, .. } => Ok(Expr::PathCall {
                path: segments,
                turbofish: None,
                args: vec![left],
                span,
            }),
            _ => Err(PipelineError::Parser {
                message: "right side of `|>` must be a function call, method call, \
                     or identifier"
                    .to_string(),
                line: span.line,
                column: span.column,
            }),
        }
    }
}
