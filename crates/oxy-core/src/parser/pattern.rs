use super::*;

impl Parser {
    pub(super) fn parse_pattern(&mut self) -> Result<Pattern, PipelineError> {
        let first = self.parse_single_pattern()?;

        // Check for or-pattern: `A | B | C`
        if self.check(&TokenKind::Pipe) {
            let span = first.span();
            let mut alternatives = vec![first];
            while self.match_token(&TokenKind::Pipe) {
                alternatives.push(self.parse_single_pattern()?);
            }
            Ok(Pattern::Or(alternatives, span))
        } else {
            Ok(first)
        }
    }

    /// Parse a struct pattern after name and `{` have been identified.
    pub(super) fn parse_struct_pattern(
        &mut self,
        name: String,
        start_span: Span,
    ) -> Result<Pattern, PipelineError> {
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.check(&TokenKind::DotDot) {
                // `..` rest pattern — ignore remaining fields
                self.advance();
                break;
            }
            let field_name = self.expect_ident()?;
            let pat = if self.match_token(&TokenKind::Colon) {
                self.parse_pattern()?
            } else {
                // Shorthand: `{ x }` means `{ x: x }`
                Pattern::Ident(field_name.clone(), self.prev_span())
            };
            fields.push((field_name, pat));
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        let end_span = self.current_span();
        self.expect(TokenKind::RBrace)?;
        Ok(Pattern::Struct {
            name,
            fields,
            span: self.merge_spans(start_span, end_span),
        })
    }

    /// Parse a single pattern (without or-pattern `|` handling).
    fn parse_single_pattern(&mut self) -> Result<Pattern, PipelineError> {
        match self.peek_kind().clone() {
            TokenKind::Underscore => {
                let span = self.current_span();
                self.advance();
                Ok(Pattern::Wildcard(span))
            }
            // Tuple pattern: `(x, y, z)`
            TokenKind::LParen => {
                let span = self.current_span();
                self.advance();
                let mut pats = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        pats.push(self.parse_pattern()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                        if self.check(&TokenKind::RParen) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RParen)?;
                Ok(Pattern::Tuple(pats, span))
            }
            // Slice pattern: `[a, b, ..]`
            TokenKind::LBracket => {
                let span = self.current_span();
                self.advance();
                let mut pats = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    loop {
                        if self.check(&TokenKind::DotDot) {
                            let rest_span = self.current_span();
                            self.advance();
                            pats.push(Pattern::Rest(rest_span));
                        } else {
                            pats.push(self.parse_pattern()?);
                        }
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                        if self.check(&TokenKind::RBracket) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Pattern::Slice(pats, span))
            }
            // Rest pattern standalone: `..`
            TokenKind::DotDot | TokenKind::DotDotEq => {
                let inclusive = self.check(&TokenKind::DotDotEq);
                let span = self.current_span();
                self.advance();
                if let TokenKind::IntLiteral(end, _) = self.peek_kind().clone() {
                    self.advance();
                    return Ok(Pattern::Range {
                        start: None,
                        end: Some(end),
                        inclusive,
                        span,
                    });
                }
                if inclusive {
                    return Ok(Pattern::Wildcard(span));
                }
                Ok(Pattern::Rest(span))
            }
            TokenKind::IntLiteral(n, suffix) => {
                let val = n;
                let start_span = self.current_span();
                self.advance(); // consume the int literal
                if self.check(&TokenKind::DotDot) || self.check(&TokenKind::DotDotEq) {
                    let inclusive = self.check(&TokenKind::DotDotEq);
                    self.advance();
                    let end = if let TokenKind::IntLiteral(m, _) = self.peek_kind().clone() {
                        self.advance();
                        Some(m)
                    } else {
                        None
                    };
                    return Ok(Pattern::Range {
                        start: Some(val),
                        end,
                        inclusive,
                        span: start_span,
                    });
                }
                Ok(Pattern::Literal(Expr::IntLiteral(val, suffix, start_span)))
            }
            TokenKind::FloatLiteral(_, _)
            | TokenKind::True
            | TokenKind::False
            | TokenKind::StringLiteral(_)
            | TokenKind::CharLiteral(_) => {
                let expr = self.parse_prefix()?;
                Ok(Pattern::Literal(expr))
            }
            TokenKind::Minus => {
                // Negative numeric literal
                let expr = self.parse_prefix()?;
                Ok(Pattern::Literal(expr))
            }
            TokenKind::Ident(_) | TokenKind::SelfUpper => {
                let span = self.current_span();
                let name = if self.check(&TokenKind::SelfUpper) {
                    self.advance();
                    "Self".to_string()
                } else {
                    self.expect_ident()?
                };

                // Handle shorthand patterns: Some(x), None, Ok(x), Err(e)
                match name.as_str() {
                    NONE_VARIANT => {
                        return Ok(Pattern::EnumVariant {
                            enum_name: OPTION_TYPE.to_string(),
                            variant: NONE_VARIANT.to_string(),
                            fields: vec![],
                            span,
                        });
                    }
                    SOME_VARIANT | OK_VARIANT | ERR_VARIANT => {
                        let enum_name = if name == SOME_VARIANT {
                            OPTION_TYPE
                        } else {
                            RESULT_TYPE
                        }
                        .to_string();
                        let mut fields = Vec::new();
                        if self.match_token(&TokenKind::LParen) {
                            if !self.check(&TokenKind::RParen) {
                                loop {
                                    fields.push(self.parse_pattern()?);
                                    if !self.match_token(&TokenKind::Comma) {
                                        break;
                                    }
                                    if self.check(&TokenKind::RParen) {
                                        break;
                                    }
                                }
                            }
                            self.expect(TokenKind::RParen)?;
                        }
                        let end_span = self.prev_span();
                        return Ok(Pattern::EnumVariant {
                            enum_name,
                            variant: name,
                            fields,
                            span: self.merge_spans(span, end_span),
                        });
                    }
                    _ => {}
                }

                // Check for path pattern: `Name::Variant` or `Name::Variant(x, y)`
                if self.check(&TokenKind::ColonColon) {
                    self.advance();
                    let variant = self.expect_ident()?;
                    let mut fields = Vec::new();

                    if self.match_token(&TokenKind::LParen) {
                        // Tuple variant destructuring: `Shape::Circle(r)`
                        if !self.check(&TokenKind::RParen) {
                            loop {
                                fields.push(self.parse_pattern()?);
                                if !self.match_token(&TokenKind::Comma) {
                                    break;
                                }
                                if self.check(&TokenKind::RParen) {
                                    break;
                                }
                            }
                        }
                        self.expect(TokenKind::RParen)?;
                    } else if self.match_token(&TokenKind::LBrace) {
                        // Struct variant destructuring: `Message::Move { x, y }`
                        if !self.check(&TokenKind::RBrace) {
                            loop {
                                let fname = self.expect_ident()?;
                                if self.match_token(&TokenKind::Colon) {
                                    // `field: pattern`
                                    fields.push(self.parse_pattern()?);
                                } else {
                                    // Shorthand: `x` means `x: x`
                                    fields.push(Pattern::Ident(fname.clone(), self.prev_span()));
                                }
                                if !self.match_token(&TokenKind::Comma) {
                                    break;
                                }
                                if self.check(&TokenKind::RBrace) {
                                    break;
                                }
                            }
                        }
                        self.expect(TokenKind::RBrace)?;
                    }

                    let end_span = self.prev_span();
                    return Ok(Pattern::EnumVariant {
                        enum_name: name,
                        variant,
                        fields,
                        span: self.merge_spans(span, end_span),
                    });
                }

                // Check for struct pattern: `Name { x, y }`
                if self.check(&TokenKind::LBrace) {
                    return self.parse_struct_pattern(name, span);
                }

                Ok(Pattern::Ident(name, span))
            }
            other => Err(self.error(format!("expected pattern, found {}", other.description()))),
        }
    }
}
