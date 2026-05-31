use super::*;

impl Parser {
    pub(super) fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, PipelineError> {
        let span = self.current_span();
        // Accept `Self` as a type
        if self.check(&TokenKind::SelfUpper) {
            self.advance();
            let generic_args = if self.check(&TokenKind::Lt) {
                self.parse_generic_args()?
            } else {
                Vec::new()
            };
            return Ok(TypeAnnotation::Named {
                name: "Self".to_string(),
                generic_args,
                span,
            });
        }
        // Accept `impl Trait` syntax (e.g., `impl Display`) — treat as the trait name
        if self.check(&TokenKind::Impl) {
            self.advance();
            let name = self.expect_ident()?;
            return Ok(TypeAnnotation::Named {
                name,
                generic_args: Vec::new(),
                span,
            });
        }
        // Reject `&` in type position — Oxy has no references.
        if self.check(&TokenKind::Amp) {
            return Err(self.error(
                "references are not supported in Oxy. Drop the `&` (use `T` instead \
                 of `&T`, `Vec<T>` instead of `&[T]`, `String` instead of `&str`). \
                 Oxy has no borrow checker — see CLAUDE.md."
                    .to_string(),
            ));
        }
        // Accept `fn(T1, T2) -> Ret` function type syntax
        if self.check(&TokenKind::Fn) {
            self.advance();
            self.expect(TokenKind::LParen)?;
            let mut param_types = Vec::new();
            if !self.check(&TokenKind::RParen) {
                loop {
                    param_types.push(self.parse_type_annotation()?.name().to_string());
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(TokenKind::RParen)?;
            let mut name = String::from("fn(");
            for (i, p) in param_types.iter().enumerate() {
                if i > 0 {
                    name.push_str(", ");
                }
                name.push_str(p);
            }
            name.push(')');
            if self.match_token(&TokenKind::Arrow) {
                let ret = self.parse_type_annotation()?;
                name.push_str(" -> ");
                name.push_str(ret.name());
            }
            return Ok(TypeAnnotation::Named {
                name,
                generic_args: Vec::new(),
                span,
            });
        }
        // Accept `()` as the Unit type, `(T, U, ...)` as a tuple type.
        if self.check(&TokenKind::LParen) {
            self.advance();
            if self.match_token(&TokenKind::RParen) {
                return Ok(TypeAnnotation::Named {
                    name: "()".to_string(),
                    generic_args: Vec::new(),
                    span,
                });
            }
            let mut elements: Vec<String> = Vec::new();
            loop {
                elements.push(self.parse_type_annotation()?.name().to_string());
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
                if self.check(&TokenKind::RParen) {
                    break;
                }
            }
            self.expect(TokenKind::RParen)?;
            // Encode as a synthetic named type so downstream code (type
            // checker, compiler) sees a single string. The runtime tuple
            // representation handles values; the annotation is mostly
            // descriptive.
            let name = format!("({})", elements.join(", "));
            return Ok(TypeAnnotation::Named {
                name,
                generic_args: Vec::new(),
                span,
            });
        }
        // Accept `[T; N]` array type
        if self.check(&TokenKind::LBracket) {
            self.advance();
            let inner = self.parse_type_annotation()?;
            self.expect(TokenKind::Semicolon)?;
            let size: usize = match self.peek_kind() {
                TokenKind::IntLiteral(n, _) => {
                    if *n < 0 {
                        return Err(PipelineError::Parser {
                            message: "array size must be non-negative".into(),
                            line: self.current_span().line,
                            column: self.current_span().column,
                        });
                    }
                    let val = *n as usize;
                    self.advance();
                    val
                }
                _ => {
                    return Err(PipelineError::Parser {
                        message: "expected integer literal for array size".into(),
                        line: self.current_span().line,
                        column: self.current_span().column,
                    });
                }
            };
            let end_span = self.current_span();
            self.expect(TokenKind::RBracket)?;
            return Ok(TypeAnnotation::Array {
                inner: Box::new(inner),
                size,
                span: self.merge_spans(span, end_span),
            });
        }
        let name = self.expect_ident()?;
        let generic_args = if self.check(&TokenKind::Lt) {
            self.parse_generic_args()?
        } else {
            Vec::new()
        };
        Ok(TypeAnnotation::Named {
            name,
            generic_args,
            span,
        })
    }

    /// Parse generic type arguments `<T, U, ...>` into a list of nested
    /// `TypeAnnotation`s. Handles nesting (`Vec<Vec<i64>>`) via the recursive
    /// `parse_type_annotation` call.
    pub(super) fn parse_generic_args(&mut self) -> Result<Vec<TypeAnnotation>, PipelineError> {
        self.expect(TokenKind::Lt)?;
        let mut args = Vec::new();
        if self.check(&TokenKind::Gt) {
            self.advance();
            return Ok(args);
        }
        loop {
            args.push(self.parse_type_annotation()?);
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        self.expect_gt_split_shr()?;
        Ok(args)
    }

    /// Close a `<...>` clause, splitting a leading `>>` (Shr) into two `>`s
    /// when needed. Lets `Vec<Option<int>>` parse without forcing users to
    /// space-separate `> >`.
    pub(super) fn expect_gt_split_shr(&mut self) -> Result<(), PipelineError> {
        if self.check(&TokenKind::Gt) {
            self.advance();
            Ok(())
        } else if self.check(&TokenKind::Shr) {
            // Replace the `>>` token in place with a single `>` so the
            // next outer generic-closing call sees the remaining `>`.
            self.tokens[self.pos].kind = TokenKind::Gt;
            Ok(())
        } else {
            Err(self.error(format!(
                "expected '>', found {}",
                self.peek_kind().description()
            )))
        }
    }

    pub(super) fn parse_generic_params(&mut self) -> Result<Vec<GenericParam>, PipelineError> {
        self.expect(TokenKind::Lt)?;
        let mut params = Vec::new();

        if self.check(&TokenKind::Gt) {
            self.advance();
            return Ok(params);
        }

        loop {
            let span = self.current_span();
            let name = self.expect_ident()?;
            let mut bounds = Vec::new();

            // Parse optional bounds: `T: Display + Clone`
            if self.match_token(&TokenKind::Colon) {
                loop {
                    let bound = self.expect_ident()?;
                    bounds.push(bound);
                    if !self.match_token(&TokenKind::Plus) {
                        break;
                    }
                }
            }

            params.push(GenericParam { name, bounds, span });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
            if self.check(&TokenKind::Gt) {
                break;
            }
        }

        self.expect_gt_split_shr()?;
        Ok(params)
    }

    /// Parse a type name optionally followed by generic arguments: `Ident` or `Ident<A, B>`.
    pub(super) fn parse_type_name_with_args(&mut self) -> Result<String, PipelineError> {
        let name = self.expect_ident()?;
        if self.check(&TokenKind::Lt) {
            self.advance(); // consume `<`
            let mut args = Vec::new();
            loop {
                args.push(self.parse_type_annotation()?);
                if self.check(&TokenKind::Gt) {
                    self.advance();
                    break;
                }
                self.expect(TokenKind::Comma)?;
            }
            let arg_names: Vec<String> = args.iter().map(|a| a.name().to_string()).collect();
            Ok(format!("{}<{}>", name, arg_names.join(", ")))
        } else {
            Ok(name)
        }
    }
}
