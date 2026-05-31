use super::*;

impl Parser {
    pub(super) fn parse_item(&mut self) -> Result<Item, PipelineError> {
        // Parse optional attributes: `#[name(arg1, arg2, ...)]`
        let mut attributes = Vec::new();
        while self.check(&TokenKind::Hash) {
            attributes.push(self.parse_attribute()?);
        }

        // Handle optional `pub` keyword
        let visibility = if self.match_token(&TokenKind::Pub) {
            Visibility::Pub
        } else {
            Visibility::Private
        };
        match self.peek_kind() {
            TokenKind::Fn => self.parse_fn_def(false, attributes, visibility).map(Item::Function),
            TokenKind::Async => {
                self.advance(); // consume `async`
                self.parse_fn_def(true, attributes, visibility).map(Item::Function)
            }
            TokenKind::Struct => self.parse_struct_def(attributes, visibility).map(Item::Struct),
            TokenKind::Enum => self.parse_enum_def(attributes, visibility).map(Item::Enum),
            TokenKind::Impl => self.parse_impl_or_impl_trait(),
            TokenKind::Trait => self.parse_trait_def(visibility).map(Item::Trait),
            TokenKind::Mod => self.parse_module_def(visibility).map(Item::Module),
            TokenKind::Use => self.parse_use_def(visibility).map(Item::Use),
            TokenKind::Type => self.parse_type_alias(),
            TokenKind::Const => self.parse_const_def(),
            other => Err(self.error(format!(
                "expected item (e.g., 'fn', 'struct', 'enum', 'impl', 'trait', 'mod', 'use', 'const', 'type'), found {}",
                other.description()
            ))),
        }
    }

    /// Parse a single attribute: `#[name(arg1, arg2, ...)]` or `#[name]`.
    fn parse_attribute(&mut self) -> Result<Attribute, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Hash)?;
        self.expect(TokenKind::LBracket)?;
        let name = self.expect_ident()?;
        let mut args = Vec::new();
        if self.match_token(&TokenKind::LParen) {
            // Parse comma-separated arguments (identifiers or strings)
            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                match self.peek_kind().clone() {
                    TokenKind::Ident(s) => {
                        args.push(s);
                        self.advance();
                    }
                    TokenKind::StringLiteral(s) => {
                        args.push(s);
                        self.advance();
                    }
                    _ => {
                        // Skip unknown tokens inside attribute args
                        self.advance();
                    }
                }
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen)?;
        }
        let end_span = self.current_span();
        self.expect(TokenKind::RBracket)?;
        Ok(Attribute {
            name,
            args,
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_module_def(&mut self, visibility: Visibility) -> Result<ModuleDef, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Mod)?;
        let name = self.expect_ident()?;

        if self.match_token(&TokenKind::Semicolon) {
            // File-based module: `mod name;`
            let end_span = self.prev_span();
            Ok(ModuleDef {
                name,
                visibility: visibility.clone(),
                body: None,
                span: self.merge_spans(start_span, end_span),
            })
        } else {
            // Inline module: `mod name { items }`
            self.expect(TokenKind::LBrace)?;
            let mut items = Vec::new();
            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                items.push(self.parse_item()?);
            }
            let end_span = self.current_span();
            self.expect(TokenKind::RBrace)?;
            Ok(ModuleDef {
                name,
                visibility: visibility.clone(),
                body: Some(items),
                span: self.merge_spans(start_span, end_span),
            })
        }
    }

    pub(super) fn parse_use_def(
        &mut self,
        visibility: Visibility,
    ) -> Result<UseDef, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Use)?;

        let mut path = Vec::new();

        // Handle special prefixes: `crate`, `self`, `super`
        if self.check(&TokenKind::Crate) {
            path.push("crate".to_string());
            self.advance();
        } else if self.check(&TokenKind::SelfLower) {
            path.push("self".to_string());
            self.advance();
        } else if self.check(&TokenKind::Super) {
            path.push("super".to_string());
            self.advance();
        } else {
            path.push(self.expect_ident()?);
        }

        // Parse path segments: `::segment`
        while self.check(&TokenKind::ColonColon) {
            self.advance();

            // Check for glob: `use path::*;`
            if self.check(&TokenKind::Star) {
                self.advance();
                let end_span = self.current_span();
                self.expect(TokenKind::Semicolon)?;
                return Ok(UseDef {
                    path,
                    tree: UseTree::Glob,
                    visibility: visibility.clone(),
                    span: self.merge_spans(start_span, end_span),
                });
            }

            // Check for group: `use path::{a, b, c};`
            if self.check(&TokenKind::LBrace) {
                self.advance();
                let mut items = Vec::new();
                loop {
                    let name = self.expect_ident()?;
                    let alias = if self.match_token(&TokenKind::As) {
                        Some(self.expect_ident()?)
                    } else {
                        None
                    };
                    items.push((name, alias));
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                    if self.check(&TokenKind::RBrace) {
                        break; // trailing comma
                    }
                }
                let end_span = self.current_span();
                self.expect(TokenKind::RBrace)?;
                self.expect(TokenKind::Semicolon)?;
                return Ok(UseDef {
                    path,
                    tree: UseTree::Group(items),
                    visibility: visibility.clone(),
                    span: self.merge_spans(start_span, end_span),
                });
            }

            // Regular path segment (including self/super/crate as identifiers)
            if self.check(&TokenKind::Crate) {
                path.push("crate".to_string());
                self.advance();
            } else if self.check(&TokenKind::SelfLower) {
                path.push("self".to_string());
                self.advance();
            } else if self.check(&TokenKind::Super) {
                path.push("super".to_string());
                self.advance();
            } else {
                path.push(self.expect_ident()?);
            }
        }

        // Simple import: `use path::item;` or `use path::item as alias;`
        let alias = if self.match_token(&TokenKind::As) {
            Some(self.expect_ident()?)
        } else {
            None
        };
        let end_span = self.current_span();
        self.expect(TokenKind::Semicolon)?;
        Ok(UseDef {
            path,
            tree: UseTree::Simple(alias),
            visibility,
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_type_alias(&mut self) -> Result<Item, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Type)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::Eq)?;
        let target = self.parse_type_annotation()?;
        let end_span = self.current_span();
        self.expect(TokenKind::Semicolon)?;
        Ok(Item::TypeAlias {
            name,
            target,
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_const_def(&mut self) -> Result<Item, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Const)?;
        let name = self.expect_ident()?;
        let type_ann = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr(Precedence::None)?;
        let end_span = self.current_span();
        self.expect(TokenKind::Semicolon)?;
        Ok(Item::Const {
            name,
            type_ann,
            value,
            span: self.merge_spans(start_span, end_span),
        })
    }

    pub(super) fn parse_fn_def(
        &mut self,
        is_async: bool,
        attributes: Vec<Attribute>,
        visibility: Visibility,
    ) -> Result<FnDef, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Fn)?;

        let name = self.expect_ident()?;

        // Optional generic parameters: `<T, U: Bound>`
        let generic_params = if self.check(&TokenKind::Lt) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };

        self.expect(TokenKind::LParen)?;

        let params = self.parse_param_list()?;
        self.expect(TokenKind::RParen)?;

        let return_type = if self.check(&TokenKind::Arrow) {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        // Single-line function body: `fn name(params) [-> Type] = expr`
        if self.check(&TokenKind::Eq) {
            self.advance(); // consume `=`
            self.ctx.fn_name_stack.push(name.clone());
            let expr = self.parse_expr(Precedence::None)?;
            self.ctx.fn_name_stack.pop();

            let expr_span = expr.span();
            let body = Block {
                stmts: vec![Stmt::Expr {
                    expr,
                    has_semicolon: false,
                }],
                span: expr_span,
            };

            return Ok(FnDef {
                name,
                is_async,
                generic_params,
                params,
                return_type,
                body,
                attributes,
                visibility: visibility.clone(),
                span: self.merge_spans(start_span, expr_span),
            });
        }

        self.ctx.fn_name_stack.push(name.clone());
        let body = self.parse_block()?;
        self.ctx.fn_name_stack.pop();

        Ok(FnDef {
            name,
            is_async,
            generic_params,
            params,
            return_type,
            body: body.clone(),
            attributes,
            visibility: visibility.clone(),
            span: self.merge_spans(start_span, body.span),
        })
    }

    pub(super) fn parse_param_list(&mut self) -> Result<Vec<Param>, PipelineError> {
        let mut params = Vec::new();

        if self.check(&TokenKind::RParen) {
            return Ok(params);
        }

        loop {
            let start_span = self.current_span();

            // Reject `&` at the start of a parameter. Oxy has no borrow checker,
            // so `&self` / `&mut self` / `&T` are noise that mislead readers.
            if self.check(&TokenKind::Amp) {
                return Err(self.error(
                    "references are not supported in Oxy. Use `self` for methods \
                     and drop `&` from parameter types (e.g., `name: String` \
                     instead of `name: &str`). \
                     Oxy has no borrow checker — see CLAUDE.md."
                        .to_string(),
                ));
            }

            // `self` parameter (for methods).
            if self.check(&TokenKind::SelfLower) {
                self.advance();
                params.push(Param {
                    name: "self".to_string(),
                    type_ann: TypeAnnotation::Named {
                        name: "Self".to_string(),
                        generic_args: Vec::new(),
                        span: start_span,
                    },
                    span: start_span,
                });
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
                continue;
            }

            let name = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;

            let type_ann = self.parse_type_annotation()?;

            params.push(Param {
                span: self.merge_spans(start_span, type_ann.span()),
                name,
                type_ann,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        Ok(params)
    }

    // === Struct parsing ===

    fn parse_struct_def(
        &mut self,
        attributes: Vec<Attribute>,
        visibility: Visibility,
    ) -> Result<StructDef, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Struct)?;
        let name = self.expect_ident()?;

        // Parse optional generic params: struct Name<T, U: Bound>
        let generic_params = if self.check(&TokenKind::Lt) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };

        // Unit struct: `struct Name;`
        if self.match_token(&TokenKind::Semicolon) {
            return Ok(StructDef {
                name,
                generic_params,
                attributes: attributes.clone(),
                kind: StructKind::Unit,
                visibility: visibility.clone(),
                span: self.merge_spans(start_span, self.prev_span()),
            });
        }

        // Tuple struct: `struct Name(Type, ...);`
        if self.check(&TokenKind::LParen) {
            self.advance();
            let mut types = Vec::new();
            if !self.check(&TokenKind::RParen) {
                loop {
                    types.push(self.parse_type_annotation()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                    if self.check(&TokenKind::RParen) {
                        break;
                    }
                }
            }
            let end_span = self.current_span();
            self.expect(TokenKind::RParen)?;
            self.expect(TokenKind::Semicolon)?;
            return Ok(StructDef {
                name,
                generic_params,
                attributes: attributes.clone(),
                kind: StructKind::Tuple(types),
                visibility: visibility.clone(),
                span: self.merge_spans(start_span, end_span),
            });
        }

        // Named struct: `struct Name { field: Type, ... }`
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            let field_pub = self.match_token(&TokenKind::Pub);
            let field_span = self.current_span();
            let field_name = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let type_ann = self.parse_type_annotation()?;
            fields.push(StructField {
                span: self.merge_spans(field_span, type_ann.span()),
                name: field_name,
                type_ann,
                visibility: if field_pub {
                    Visibility::Pub
                } else {
                    Visibility::Private
                },
            });
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        let end_span = self.current_span();
        self.expect(TokenKind::RBrace)?;

        Ok(StructDef {
            name,
            generic_params,
            attributes,
            kind: StructKind::Named(fields),
            visibility: visibility.clone(),
            span: self.merge_spans(start_span, end_span),
        })
    }

    // === Enum parsing ===

    fn parse_enum_def(
        &mut self,
        attributes: Vec<Attribute>,
        visibility: Visibility,
    ) -> Result<EnumDef, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Enum)?;
        let name = self.expect_ident()?;

        // Parse optional generic params: enum Name<T, U: Bound>
        let generic_params = if self.check(&TokenKind::Lt) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };

        self.expect(TokenKind::LBrace)?;

        let mut variants = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            let var_span = self.current_span();
            let var_name = self.expect_ident()?;

            let kind = if self.check(&TokenKind::LParen) {
                // Tuple variant: `Variant(Type, ...)`
                self.advance();
                let mut types = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        types.push(self.parse_type_annotation()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                        if self.check(&TokenKind::RParen) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RParen)?;
                EnumVariantKind::Tuple(types)
            } else if self.check(&TokenKind::LBrace) {
                // Struct variant: `Variant { field: Type, ... }`
                self.advance();
                let mut fields = Vec::new();
                while !self.check(&TokenKind::RBrace) {
                    let fspan = self.current_span();
                    let fname = self.expect_ident()?;
                    self.expect(TokenKind::Colon)?;
                    let ftype = self.parse_type_annotation()?;
                    fields.push(StructField {
                        span: self.merge_spans(fspan, ftype.span()),
                        name: fname,
                        type_ann: ftype,
                        visibility: Visibility::Private,
                    });
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::RBrace)?;
                EnumVariantKind::Struct(fields)
            } else {
                // Unit variant: `Variant`
                EnumVariantKind::Unit
            };

            variants.push(EnumVariant {
                span: self.merge_spans(var_span, self.prev_span()),
                name: var_name,
                kind,
            });
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        let end_span = self.current_span();
        self.expect(TokenKind::RBrace)?;

        Ok(EnumDef {
            name,
            generic_params,
            attributes,
            variants,
            visibility: visibility.clone(),
            span: self.merge_spans(start_span, end_span),
        })
    }

    // === Impl block parsing ===

    /// Prepend impl-level generic params to a method's own generic_params.
    /// Skip duplicates (an `impl<T> Foo<T> { fn bar<T>(...) }` shouldn't
    /// register T twice — the method's local T shadows the impl's T).
    fn merge_impl_generics(method: &mut FnDef, impl_generics: &[GenericParam]) {
        if impl_generics.is_empty() {
            return;
        }
        let mut merged: Vec<GenericParam> =
            Vec::with_capacity(impl_generics.len() + method.generic_params.len());
        for gp in impl_generics {
            if !method.generic_params.iter().any(|m| m.name == gp.name) {
                merged.push(gp.clone());
            }
        }
        merged.append(&mut method.generic_params);
        method.generic_params = merged;
    }

    /// Parse `impl Type { ... }` or `impl Trait for Type { ... }`
    fn parse_impl_or_impl_trait(&mut self) -> Result<Item, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Impl)?;
        // Optional generic parameters on the impl block itself, e.g.
        // `impl<T> Foo<T> { ... }`. Captured so we can prepend them to
        // each method's per-method generic_params below — that piggybacks
        // on the existing per-method generic machinery (monomorphization,
        // T-resolution in bodies, turbofish) without needing a new
        // generic_params field on ImplBlock.
        let impl_generics: Vec<GenericParam> = if self.check(&TokenKind::Lt) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };
        let first_name = self.parse_type_name_with_args()?;

        // Check for `impl Trait for Type { ... }`
        if self.check(&TokenKind::For) {
            self.advance();
            let type_name = self.parse_type_name_with_args()?;
            self.expect(TokenKind::LBrace)?;

            let mut methods = Vec::new();
            while !self.check(&TokenKind::RBrace) {
                let method_vis = if self.match_token(&TokenKind::Pub) {
                    Visibility::Pub
                } else {
                    Visibility::Private
                };
                let is_async = self.match_token(&TokenKind::Async);
                let mut m = self.parse_fn_def(is_async, vec![], method_vis)?;
                Self::merge_impl_generics(&mut m, &impl_generics);
                methods.push(m);
            }
            let end_span = self.current_span();
            self.expect(TokenKind::RBrace)?;

            return Ok(Item::ImplTrait(ImplTraitBlock {
                trait_name: first_name,
                type_name,
                methods,
                span: self.merge_spans(start_span, end_span),
            }));
        }

        // Regular `impl Type { ... }`
        self.expect(TokenKind::LBrace)?;

        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            let method_vis = if self.match_token(&TokenKind::Pub) {
                Visibility::Pub
            } else {
                Visibility::Private
            };
            let is_async = self.match_token(&TokenKind::Async);
            let mut m = self.parse_fn_def(is_async, vec![], method_vis)?;
            Self::merge_impl_generics(&mut m, &impl_generics);
            methods.push(m);
        }
        let end_span = self.current_span();
        self.expect(TokenKind::RBrace)?;

        Ok(Item::Impl(ImplBlock {
            type_name: first_name,
            methods,
            span: self.merge_spans(start_span, end_span),
        }))
    }

    // === Trait parsing ===

    fn parse_trait_def(&mut self, visibility: Visibility) -> Result<TraitDef, PipelineError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Trait)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;

        let mut methods = Vec::new();
        let mut default_methods = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            let sig_start = self.current_span();
            self.expect(TokenKind::Fn)?;
            let method_name = self.expect_ident()?;

            // Optional generic params on trait method
            let _generic_params = if self.check(&TokenKind::Lt) {
                self.parse_generic_params()?
            } else {
                Vec::new()
            };

            self.expect(TokenKind::LParen)?;
            let params = self.parse_param_list()?;
            self.expect(TokenKind::RParen)?;

            let return_type = if self.check(&TokenKind::Arrow) {
                self.advance();
                Some(self.parse_type_annotation()?)
            } else {
                None
            };

            // If followed by `{`, it's a default method implementation
            if self.check(&TokenKind::LBrace) {
                let body = self.parse_block()?;
                default_methods.push(FnDef {
                    name: method_name,
                    is_async: false,
                    generic_params: Vec::new(),
                    params,
                    return_type,
                    body: body.clone(),
                    attributes: vec![],
                    visibility: Visibility::Private,
                    span: self.merge_spans(sig_start, body.span),
                });
            } else {
                // Method signature only — ends with `;`
                let end_span = self.current_span();
                self.expect(TokenKind::Semicolon)?;
                methods.push(TraitMethodSig {
                    name: method_name,
                    params,
                    return_type,
                    span: self.merge_spans(sig_start, end_span),
                });
            }
        }
        let end_span = self.current_span();
        self.expect(TokenKind::RBrace)?;

        Ok(TraitDef {
            name,
            methods,
            default_methods,
            visibility: visibility.clone(),
            span: self.merge_spans(start_span, end_span),
        })
    }
}
