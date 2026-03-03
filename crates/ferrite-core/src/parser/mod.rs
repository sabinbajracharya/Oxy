//! Recursive descent parser with Pratt parsing for expressions.
//!
//! Parses a token stream into an AST. Operator precedence follows Rust's rules.

use crate::ast::*;
use crate::errors::FerriError;
use crate::lexer::{Span, Token, TokenKind};
use crate::types::{ERR_VARIANT, NONE_VARIANT, OK_VARIANT, OPTION_TYPE, RESULT_TYPE, SOME_VARIANT};

/// Parser for the Ferrite language.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

/// Operator precedence levels (lower number = lower precedence = binds less tightly).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
#[allow(dead_code)] // Variants like Call will be used in later phases
enum Precedence {
    None = 0,
    Assignment = 1, // = += -= etc.
    Range = 2,      // .. ..=
    Or = 3,         // ||
    And = 4,        // &&
    BitOr = 5,      // |
    BitXor = 6,     // ^
    BitAnd = 7,     // &
    Equality = 8,   // == !=
    Comparison = 9, // < > <= >=
    Shift = 10,     // << >>
    Term = 11,      // + -
    Factor = 12,    // * / %
    Unary = 13,     // - ! & *
    Call = 14,      // () .
}

impl Precedence {
    /// Get precedence for a binary operator token.
    fn of_binary(kind: &TokenKind) -> Self {
        match kind {
            TokenKind::PipePipe => Precedence::Or,
            TokenKind::AmpAmp => Precedence::And,
            TokenKind::Pipe => Precedence::BitOr,
            TokenKind::Caret => Precedence::BitXor,
            TokenKind::Amp => Precedence::BitAnd,
            TokenKind::EqEq | TokenKind::BangEq => Precedence::Equality,
            TokenKind::Lt | TokenKind::Gt | TokenKind::LtEq | TokenKind::GtEq => {
                Precedence::Comparison
            }
            TokenKind::Shl | TokenKind::Shr => Precedence::Shift,
            TokenKind::Plus | TokenKind::Minus => Precedence::Term,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Factor,
            TokenKind::LParen | TokenKind::Dot | TokenKind::LBracket | TokenKind::Question => {
                Precedence::Call
            }
            TokenKind::Eq
            | TokenKind::PlusEq
            | TokenKind::MinusEq
            | TokenKind::StarEq
            | TokenKind::SlashEq
            | TokenKind::PercentEq => Precedence::Assignment,
            TokenKind::DotDot | TokenKind::DotDotEq => Precedence::Range,
            _ => Precedence::None,
        }
    }
}

impl Parser {
    /// Create a new parser from a token stream.
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parse the tokens into a [`Program`].
    pub fn parse(mut self) -> Result<Program, FerriError> {
        let start_span = self.current_span();
        let mut items = Vec::new();

        while !self.is_at_end() {
            items.push(self.parse_item()?);
        }

        let end_span = if items.is_empty() {
            start_span
        } else {
            items.last().unwrap().span()
        };

        Ok(Program {
            items,
            span: self.merge_spans(start_span, end_span),
        })
    }

    // === Item parsing ===

    fn parse_item(&mut self) -> Result<Item, FerriError> {
        // Parse optional attributes: `#[name(arg1, arg2, ...)]`
        let mut attributes = Vec::new();
        while self.check(&TokenKind::Hash) {
            attributes.push(self.parse_attribute()?);
        }

        // Handle optional `pub` keyword
        let is_pub = self.check(&TokenKind::Pub);
        if is_pub {
            self.advance();
        }
        match self.peek_kind() {
            TokenKind::Fn => self.parse_fn_def(false, attributes, is_pub).map(Item::Function),
            TokenKind::Async => {
                self.advance(); // consume `async`
                self.parse_fn_def(true, attributes, is_pub).map(Item::Function)
            }
            TokenKind::Struct => self.parse_struct_def(attributes, is_pub).map(Item::Struct),
            TokenKind::Enum => self.parse_enum_def(attributes, is_pub).map(Item::Enum),
            TokenKind::Impl => self.parse_impl_or_impl_trait(),
            TokenKind::Trait => self.parse_trait_def(is_pub).map(Item::Trait),
            TokenKind::Mod => self.parse_module_def(is_pub).map(Item::Module),
            TokenKind::Use => self.parse_use_def().map(Item::Use),
            TokenKind::Type => self.parse_type_alias(),
            TokenKind::Const => self.parse_const_def(false),
            TokenKind::Static => self.parse_const_def(true),
            other => Err(self.error(format!(
                "expected item (e.g., 'fn', 'struct', 'enum', 'impl', 'trait', 'mod', 'use', 'const', 'type'), found {}",
                other.description()
            ))),
        }
    }

    /// Parse a single attribute: `#[name(arg1, arg2, ...)]` or `#[name]`.
    fn parse_attribute(&mut self) -> Result<Attribute, FerriError> {
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

    fn parse_module_def(&mut self, is_pub: bool) -> Result<ModuleDef, FerriError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Mod)?;
        let name = self.expect_ident()?;

        if self.match_token(&TokenKind::Semicolon) {
            // File-based module: `mod name;`
            let end_span = self.prev_span();
            Ok(ModuleDef {
                name,
                is_pub,
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
                is_pub,
                body: Some(items),
                span: self.merge_spans(start_span, end_span),
            })
        }
    }

    fn parse_use_def(&mut self) -> Result<UseDef, FerriError> {
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
                    span: self.merge_spans(start_span, end_span),
                });
            }

            // Check for group: `use path::{a, b, c};`
            if self.check(&TokenKind::LBrace) {
                self.advance();
                let mut names = Vec::new();
                loop {
                    names.push(self.expect_ident()?);
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
                    tree: UseTree::Group(names),
                    span: self.merge_spans(start_span, end_span),
                });
            }

            // Regular path segment
            path.push(self.expect_ident()?);
        }

        // Simple import: `use path::item;`
        let end_span = self.current_span();
        self.expect(TokenKind::Semicolon)?;
        Ok(UseDef {
            path,
            tree: UseTree::Simple,
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_type_alias(&mut self) -> Result<Item, FerriError> {
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

    fn parse_const_def(&mut self, is_static: bool) -> Result<Item, FerriError> {
        let start_span = self.current_span();
        if is_static {
            self.expect(TokenKind::Static)?;
        } else {
            self.expect(TokenKind::Const)?;
        }
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
            is_static,
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_fn_def(
        &mut self,
        is_async: bool,
        attributes: Vec<Attribute>,
        is_pub: bool,
    ) -> Result<FnDef, FerriError> {
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

        // Skip optional `where` clause (parse but ignore)
        if self.check(&TokenKind::Where) {
            self.advance();
            // Consume tokens until `{`
            while !self.check(&TokenKind::LBrace) && !self.is_at_end() {
                self.advance();
            }
        }

        let body = self.parse_block()?;

        Ok(FnDef {
            name,
            is_async,
            generic_params,
            params,
            return_type,
            body: body.clone(),
            attributes,
            is_pub,
            span: self.merge_spans(start_span, body.span),
        })
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>, FerriError> {
        let mut params = Vec::new();

        if self.check(&TokenKind::RParen) {
            return Ok(params);
        }

        loop {
            let start_span = self.current_span();

            // Accept optional & or &mut before param name (parse but ignore)
            if self.check(&TokenKind::Amp) {
                self.advance();
                if self.check(&TokenKind::Mut) {
                    self.advance();
                }
            }

            // Accept `self` as a parameter (for methods)
            if self.check(&TokenKind::SelfLower) || self.check(&TokenKind::Mut) {
                let is_mut = self.check(&TokenKind::Mut);
                if is_mut {
                    self.advance(); // consume `mut`
                }
                if self.check(&TokenKind::SelfLower) {
                    self.advance(); // consume `self`
                    params.push(Param {
                        name: "self".to_string(),
                        type_ann: TypeAnnotation {
                            name: "Self".to_string(),
                            span: start_span,
                        },
                        span: start_span,
                    });
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                    continue;
                }
            }

            let name = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;

            // Accept optional & or &mut in type position (parse but ignore)
            if self.check(&TokenKind::Amp) {
                self.advance();
                if self.check(&TokenKind::Mut) {
                    self.advance();
                }
            }

            let type_ann = self.parse_type_annotation()?;

            params.push(Param {
                span: self.merge_spans(start_span, type_ann.span),
                name,
                type_ann,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        Ok(params)
    }

    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, FerriError> {
        let span = self.current_span();
        // Accept `Self` as a type
        if self.check(&TokenKind::SelfUpper) {
            self.advance();
            // Skip generic type args if present: `Self<T>`
            if self.check(&TokenKind::Lt) {
                self.skip_generic_args();
            }
            return Ok(TypeAnnotation {
                name: "Self".to_string(),
                span,
            });
        }
        // Accept `impl Trait` syntax (e.g., `impl Display`) — treat as the trait name
        if self.check(&TokenKind::Impl) {
            self.advance();
            let name = self.expect_ident()?;
            return Ok(TypeAnnotation { name, span });
        }
        // Accept `&` or `&mut` before type
        if self.check(&TokenKind::Amp) {
            self.advance();
            if self.check(&TokenKind::Mut) {
                self.advance();
            }
        }
        let name = self.expect_ident()?;
        // Skip generic type args: `Vec<i64>`, `HashMap<K, V>`
        if self.check(&TokenKind::Lt) {
            self.skip_generic_args();
        }
        Ok(TypeAnnotation { name, span })
    }

    /// Skip generic type arguments `<...>` — handles nesting.
    fn skip_generic_args(&mut self) {
        if !self.match_token(&TokenKind::Lt) {
            return;
        }
        let mut depth = 1;
        while depth > 0 && !self.is_at_end() {
            if self.check(&TokenKind::Lt) {
                depth += 1;
            } else if self.check(&TokenKind::Gt) {
                depth -= 1;
            }
            self.advance();
        }
    }

    fn parse_generic_params(&mut self) -> Result<Vec<GenericParam>, FerriError> {
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

        self.expect(TokenKind::Gt)?;
        Ok(params)
    }

    // === Struct parsing ===

    fn parse_struct_def(
        &mut self,
        attributes: Vec<Attribute>,
        is_pub: bool,
    ) -> Result<StructDef, FerriError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Struct)?;
        let name = self.expect_ident()?;

        // Unit struct: `struct Name;`
        if self.match_token(&TokenKind::Semicolon) {
            return Ok(StructDef {
                name,
                attributes: attributes.clone(),
                kind: StructKind::Unit,
                is_pub,
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
                attributes: attributes.clone(),
                kind: StructKind::Tuple(types),
                is_pub,
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
                span: self.merge_spans(field_span, type_ann.span),
                name: field_name,
                type_ann,
                is_pub: field_pub,
            });
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        let end_span = self.current_span();
        self.expect(TokenKind::RBrace)?;

        Ok(StructDef {
            name,
            attributes,
            kind: StructKind::Named(fields),
            is_pub,
            span: self.merge_spans(start_span, end_span),
        })
    }

    // === Enum parsing ===

    fn parse_enum_def(
        &mut self,
        attributes: Vec<Attribute>,
        is_pub: bool,
    ) -> Result<EnumDef, FerriError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Enum)?;
        let name = self.expect_ident()?;
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
                        span: self.merge_spans(fspan, ftype.span),
                        name: fname,
                        type_ann: ftype,
                        is_pub: false,
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
            attributes,
            variants,
            is_pub,
            span: self.merge_spans(start_span, end_span),
        })
    }

    // === Impl block parsing ===

    /// Parse `impl Type { ... }` or `impl Trait for Type { ... }`
    fn parse_impl_or_impl_trait(&mut self) -> Result<Item, FerriError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Impl)?;
        let first_name = self.expect_ident()?;

        // Check for `impl Trait for Type { ... }`
        if self.check(&TokenKind::For) {
            self.advance();
            let type_name = self.expect_ident()?;
            self.expect(TokenKind::LBrace)?;

            let mut methods = Vec::new();
            while !self.check(&TokenKind::RBrace) {
                let is_pub_method = self.match_token(&TokenKind::Pub);
                methods.push(self.parse_fn_def(false, vec![], is_pub_method)?);
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
            let is_pub_method = self.match_token(&TokenKind::Pub);
            methods.push(self.parse_fn_def(false, vec![], is_pub_method)?);
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

    fn parse_trait_def(&mut self, is_pub: bool) -> Result<TraitDef, FerriError> {
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
                    is_pub: false,
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
            is_pub,
            span: self.merge_spans(start_span, end_span),
        })
    }

    // === Block parsing ===

    fn parse_block(&mut self) -> Result<Block, FerriError> {
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

    fn parse_stmt(&mut self) -> Result<Stmt, FerriError> {
        match self.peek_kind() {
            TokenKind::Let => self.parse_let_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::Loop => self.parse_loop_stmt(),
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::Break => self.parse_break_stmt(),
            TokenKind::Continue => self.parse_continue_stmt(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_let_stmt(&mut self) -> Result<Stmt, FerriError> {
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

        let name = self.expect_ident()?;

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

    fn parse_return_stmt(&mut self) -> Result<Stmt, FerriError> {
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

    fn parse_while_stmt(&mut self) -> Result<Stmt, FerriError> {
        let start_span = self.current_span();
        self.expect(TokenKind::While)?;

        // Check for `while let`
        if self.check(&TokenKind::Let) {
            self.advance();
            let pattern = self.parse_pattern()?;
            self.expect(TokenKind::Eq)?;
            let expr = self.parse_expr(Precedence::None)?;
            let body = self.parse_block()?;
            let end_span = body.span;
            return Ok(Stmt::WhileLet {
                pattern: Box::new(pattern),
                expr: Box::new(expr),
                body,
                span: self.merge_spans(start_span, end_span),
            });
        }

        let condition = self.parse_expr(Precedence::None)?;
        let body = self.parse_block()?;
        let end_span = body.span;

        Ok(Stmt::While {
            condition: Box::new(condition),
            body,
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_loop_stmt(&mut self) -> Result<Stmt, FerriError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Loop)?;

        let body = self.parse_block()?;
        let end_span = body.span;

        Ok(Stmt::Loop {
            body,
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, FerriError> {
        let start_span = self.current_span();
        self.expect(TokenKind::For)?;

        // Check for tuple destructuring: `for (a, b) in ...`
        if self.check(&TokenKind::LParen) {
            self.advance();
            let mut names = Vec::new();
            loop {
                names.push(self.expect_ident()?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
                if self.check(&TokenKind::RParen) {
                    break;
                }
            }
            self.expect(TokenKind::RParen)?;
            self.expect(TokenKind::In)?;
            let iterable = self.parse_expr(Precedence::None)?;
            let body = self.parse_block()?;
            let end_span = body.span;
            return Ok(Stmt::ForDestructure {
                names,
                iterable: Box::new(iterable),
                body,
                span: self.merge_spans(start_span, end_span),
            });
        }

        let name = self.expect_ident()?;
        self.expect(TokenKind::In)?;

        let iterable = self.parse_expr(Precedence::None)?;
        let body = self.parse_block()?;
        let end_span = body.span;

        Ok(Stmt::For {
            name,
            iterable: Box::new(iterable),
            body,
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_break_stmt(&mut self) -> Result<Stmt, FerriError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Break)?;

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
            value,
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_continue_stmt(&mut self) -> Result<Stmt, FerriError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Continue)?;

        let end_span = self.current_span();
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }

        Ok(Stmt::Continue {
            span: self.merge_spans(start_span, end_span),
        })
    }

    fn parse_expr_stmt(&mut self) -> Result<Stmt, FerriError> {
        let expr = self.parse_expr(Precedence::None)?;

        let has_semicolon = self.match_token(&TokenKind::Semicolon);

        Ok(Stmt::Expr {
            expr,
            has_semicolon,
        })
    }

    fn parse_struct_init(&mut self, name: String, start_span: Span) -> Result<Expr, FerriError> {
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            let field_name = self.expect_ident()?;
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
            span: self.merge_spans(start_span, end_span),
        })
    }

    // === Expression parsing (Pratt / precedence climbing) ===

    fn parse_expr(&mut self, min_prec: Precedence) -> Result<Expr, FerriError> {
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

    fn parse_prefix(&mut self) -> Result<Expr, FerriError> {
        match self.peek_kind().clone() {
            // Literals
            TokenKind::IntLiteral(n) => {
                let span = self.current_span();
                self.advance();
                Ok(Expr::IntLiteral(n, span))
            }
            TokenKind::FloatLiteral(n) => {
                let span = self.current_span();
                self.advance();
                Ok(Expr::FloatLiteral(n, span))
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

            // `self` keyword
            TokenKind::SelfLower => {
                let span = self.current_span();
                self.advance();
                Ok(Expr::SelfRef(span))
            }

            // Identifiers (could be followed by `!` for macro, `::` for path, `{` for struct init)
            TokenKind::Ident(_) | TokenKind::SelfUpper => {
                let span = self.current_span();
                let name = if self.check(&TokenKind::SelfUpper) {
                    self.advance();
                    "Self".to_string()
                } else {
                    self.expect_ident()?
                };

                // Check for macro call: `name!(...)` or `name![...]`
                if self.check(&TokenKind::Bang) {
                    self.advance(); // consume `!`
                    if self.check(&TokenKind::LBracket) {
                        // `name![...]`
                        self.advance();
                        let args = self.parse_arg_list()?;
                        let end_span = self.current_span();
                        self.expect(TokenKind::RBracket)?;
                        return Ok(Expr::MacroCall {
                            name,
                            args,
                            span: self.merge_spans(span, end_span),
                        });
                    }
                    self.expect(TokenKind::LParen)?;
                    let args = self.parse_arg_list()?;
                    let end_span = self.current_span();
                    self.expect(TokenKind::RParen)?;
                    return Ok(Expr::MacroCall {
                        name,
                        args,
                        span: self.merge_spans(span, end_span),
                    });
                }

                // Check for path: `Name::...`
                if self.check(&TokenKind::ColonColon) {
                    self.advance();
                    let mut segments = vec![name];
                    segments.push(self.expect_ident()?);

                    // Continue collecting path segments
                    while self.check(&TokenKind::ColonColon) {
                        self.advance();
                        segments.push(self.expect_ident()?);
                    }

                    // Check if followed by `(` → PathCall
                    if self.check(&TokenKind::LParen) {
                        self.advance();
                        let args = self.parse_arg_list()?;
                        let end_span = self.current_span();
                        self.expect(TokenKind::RParen)?;
                        return Ok(Expr::PathCall {
                            path: segments,
                            args,
                            span: self.merge_spans(span, end_span),
                        });
                    }

                    // Check if followed by `{` → struct init with path (e.g. module::Struct { })
                    // For now just return Path
                    let end_span = self.prev_span();
                    return Ok(Expr::Path {
                        segments,
                        span: self.merge_spans(span, end_span),
                    });
                }

                // Check for struct init: `Name { field: value, ... }`
                // Only if name starts with uppercase (convention for type names)
                if self.check(&TokenKind::LBrace) && name.starts_with(|c: char| c.is_uppercase()) {
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
                let mut elements = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    loop {
                        elements.push(self.parse_expr(Precedence::None)?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                        // Allow trailing comma
                        if self.check(&TokenKind::RBracket) {
                            break;
                        }
                    }
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
                        body,
                        span: self.merge_spans(start_span, end_span),
                    }],
                    span: self.merge_spans(start_span, end_span),
                }))
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
            TokenKind::Amp => {
                let span = self.current_span();
                self.advance();
                // Accept optional `mut` after `&`
                if self.check(&TokenKind::Mut) {
                    self.advance();
                }
                let expr = self.parse_expr(Precedence::Unary)?;
                let end_span = expr.span();
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Ref,
                    expr: Box::new(expr),
                    span: self.merge_spans(span, end_span),
                })
            }

            // Closure: `|params| expr` or `|params| { body }`
            TokenKind::Pipe => self.parse_closure(),

            // Closure with no params: `|| expr` or `|| { body }`
            TokenKind::PipePipe => self.parse_empty_closure(),

            // `move` closure: `move |params| expr` or `move || expr`
            TokenKind::Move => {
                self.advance(); // consume `move`
                if self.check(&TokenKind::PipePipe) {
                    self.parse_empty_closure()
                } else {
                    self.parse_closure()
                }
            }

            other => Err(self.error(format!(
                "expected expression, found {}",
                other.description()
            ))),
        }
    }

    fn parse_infix(&mut self, left: Expr, prec: Precedence) -> Result<Expr, FerriError> {
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
            let right = self.parse_expr(prec)?;
            let span = self.merge_spans(left.span(), right.span());
            return Ok(Expr::Range {
                start: Box::new(left),
                end: Box::new(right),
                inclusive,
                span,
            });
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
            if let TokenKind::IntLiteral(n) = self.peek_kind() {
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

            // Check for method call: `.name(...)`
            if self.check(&TokenKind::LParen) {
                self.advance();
                let args = self.parse_arg_list()?;
                let end_span = self.current_span();
                self.expect(TokenKind::RParen)?;
                return Ok(Expr::MethodCall {
                    object: Box::new(left),
                    method: name,
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

        Err(self.error(format!(
            "unexpected token in expression: {}",
            op_kind.description()
        )))
    }

    fn parse_if_expr(&mut self) -> Result<Expr, FerriError> {
        let start_span = self.current_span();
        self.expect(TokenKind::If)?;

        // Check for `if let` pattern
        if self.check(&TokenKind::Let) {
            return self.parse_if_let_expr(start_span);
        }

        let condition = self.parse_expr(Precedence::None)?;
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

    fn parse_if_let_expr(&mut self, start_span: Span) -> Result<Expr, FerriError> {
        self.expect(TokenKind::Let)?;
        let pattern = self.parse_pattern()?;
        self.expect(TokenKind::Eq)?;
        let expr = self.parse_expr(Precedence::None)?;
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
            then_block,
            else_block,
            span: self.merge_spans(start_span, end_span),
        })
    }

    /// Parse a closure expression: `|params| expr` or `|params| { body }`
    fn parse_closure(&mut self) -> Result<Expr, FerriError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Pipe)?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::Pipe) {
            loop {
                let pspan = self.current_span();
                let name = self.expect_ident()?;
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
        })
    }

    /// Parse a closure with no params: `|| expr` or `|| { body }`
    fn parse_empty_closure(&mut self) -> Result<Expr, FerriError> {
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
        })
    }

    fn parse_match_expr(&mut self) -> Result<Expr, FerriError> {
        let start_span = self.current_span();
        self.expect(TokenKind::Match)?;

        let expr = self.parse_expr(Precedence::None)?;
        self.expect(TokenKind::LBrace)?;

        let mut arms = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let arm_span = self.current_span();
            let pattern = self.parse_pattern()?;
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

    fn parse_pattern(&mut self) -> Result<Pattern, FerriError> {
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
    fn parse_struct_pattern(
        &mut self,
        name: String,
        start_span: Span,
    ) -> Result<Pattern, FerriError> {
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
    fn parse_single_pattern(&mut self) -> Result<Pattern, FerriError> {
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
            TokenKind::DotDot => {
                let span = self.current_span();
                self.advance();
                Ok(Pattern::Rest(span))
            }
            TokenKind::IntLiteral(_)
            | TokenKind::FloatLiteral(_)
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

    fn parse_arg_list(&mut self) -> Result<Vec<Expr>, FerriError> {
        let mut args = Vec::new();

        if self.check(&TokenKind::RParen) {
            return Ok(args);
        }

        loop {
            args.push(self.parse_expr(Precedence::None)?);
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        Ok(args)
    }

    // === Helpers ===

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

    fn peek_kind(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn current_span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn prev_span(&self) -> Span {
        if self.pos > 0 {
            self.tokens[self.pos - 1].span
        } else {
            self.tokens[0].span
        }
    }

    fn is_at_end(&self) -> bool {
        self.peek_kind() == &TokenKind::Eof
    }

    fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.pos];
        if !self.is_at_end() {
            self.pos += 1;
        }
        token
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind)
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<&Token, FerriError> {
        if self.check(&kind) {
            Ok(self.advance())
        } else {
            Err(self.error(format!(
                "expected {}, found {}",
                kind.description(),
                self.peek_kind().description()
            )))
        }
    }

    fn expect_ident(&mut self) -> Result<String, FerriError> {
        match self.peek_kind().clone() {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            // Also accept `Self` as an identifier in type position
            TokenKind::SelfUpper => {
                self.advance();
                Ok("Self".to_string())
            }
            other => Err(self.error(format!(
                "expected identifier, found {}",
                other.description()
            ))),
        }
    }

    fn error(&self, message: String) -> FerriError {
        let span = self.current_span();
        FerriError::Parser {
            message,
            line: span.line,
            column: span.column,
        }
    }

    fn merge_spans(&self, start: Span, end: Span) -> Span {
        Span::new(start.start, end.end, start.line, start.column)
    }

    /// Parse the raw content of an f-string into `FStringPart`s.
    fn parse_fstring_parts(&self, raw: &str, span: Span) -> Result<Vec<FStringPart>, FerriError> {
        let mut parts = Vec::new();
        let mut literal = String::new();
        let chars: Vec<char> = raw.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '{' {
                // Escaped brace `{{` → literal `{`
                if i + 1 < chars.len() && chars[i + 1] == '{' {
                    literal.push('{');
                    i += 2;
                    continue;
                }
                // Interpolation: collect until matching `}`
                if !literal.is_empty() {
                    parts.push(FStringPart::Literal(std::mem::take(&mut literal)));
                }
                i += 1; // skip `{`
                let mut depth = 1;
                let mut expr_text = String::new();
                while i < chars.len() && depth > 0 {
                    let c = chars[i];
                    if c == '{' {
                        depth += 1;
                        expr_text.push(c);
                    } else if c == '}' {
                        depth -= 1;
                        if depth > 0 {
                            expr_text.push(c);
                        }
                    } else {
                        expr_text.push(c);
                    }
                    i += 1;
                }
                if depth > 0 {
                    return Err(FerriError::Parser {
                        message: "unterminated interpolation in f-string".into(),
                        line: span.line,
                        column: span.column,
                    });
                }
                // Parse the expression text via a sub-parser
                let tokens =
                    crate::lexer::tokenize(&expr_text).map_err(|_| FerriError::Parser {
                        message: format!("failed to tokenize f-string expression: {expr_text}"),
                        line: span.line,
                        column: span.column,
                    })?;
                let mut sub_parser = Parser::new(tokens);
                let expr =
                    sub_parser
                        .parse_expr(Precedence::None)
                        .map_err(|_| FerriError::Parser {
                            message: format!("failed to parse f-string expression: {expr_text}"),
                            line: span.line,
                            column: span.column,
                        })?;
                parts.push(FStringPart::Expr(Box::new(expr)));
            } else if chars[i] == '}' {
                // Escaped brace `}}` → literal `}`
                if i + 1 < chars.len() && chars[i + 1] == '}' {
                    literal.push('}');
                    i += 2;
                    continue;
                }
                literal.push('}');
                i += 1;
            } else {
                literal.push(chars[i]);
                i += 1;
            }
        }

        if !literal.is_empty() {
            parts.push(FStringPart::Literal(literal));
        }

        Ok(parts)
    }
}

/// Convenience function to parse source code into an AST.
pub fn parse(source: &str) -> Result<Program, FerriError> {
    let tokens = crate::lexer::tokenize(source)?;
    Parser::new(tokens).parse()
}

#[cfg(test)]
#[allow(irrefutable_let_patterns)] // Item only has Function for now; more variants coming
mod tests {
    use super::*;

    /// Extract the function body statements from a single-function program.
    fn parse_fn_body(src: &str) -> Vec<Stmt> {
        let program = parse(src).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function item");
        };
        f.body.stmts.clone()
    }

    /// Extract a FnDef from the first item.
    fn parse_fn(src: &str) -> FnDef {
        let program = parse(src).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function item");
        };
        f.clone()
    }

    // === Let statements ===

    #[test]
    fn test_let_simple() {
        let stmts = parse_fn_body("fn main() { let x = 42; }");
        assert_eq!(stmts.len(), 1);
        let Stmt::Let {
            name,
            mutable,
            value,
            ..
        } = &stmts[0]
        else {
            panic!("expected let statement");
        };
        assert_eq!(name, "x");
        assert!(!mutable);
        assert!(value.is_some());
    }

    #[test]
    fn test_let_mut() {
        let stmts = parse_fn_body("fn main() { let mut x = 10; }");
        let Stmt::Let { name, mutable, .. } = &stmts[0] else {
            panic!("expected let statement");
        };
        assert_eq!(name, "x");
        assert!(mutable);
    }

    #[test]
    fn test_let_with_type() {
        let stmts = parse_fn_body("fn main() { let x: i64 = 42; }");
        let Stmt::Let { type_ann, .. } = &stmts[0] else {
            panic!("expected let statement");
        };
        assert_eq!(type_ann.as_ref().unwrap().name, "i64");
    }

    // === Functions ===

    #[test]
    fn test_fn_no_params() {
        let f = parse_fn("fn main() {}");
        assert_eq!(f.name, "main");
        assert!(f.params.is_empty());
        assert!(f.return_type.is_none());
    }

    #[test]
    fn test_fn_with_params_and_return() {
        let f = parse_fn("fn add(a: i64, b: i64) -> i64 { a }");
        assert_eq!(f.name, "add");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].name, "a");
        assert_eq!(f.params[0].type_ann.name, "i64");
        assert_eq!(f.params[1].name, "b");
        assert_eq!(f.return_type.as_ref().unwrap().name, "i64");
    }

    #[test]
    fn test_multiple_functions() {
        let program = parse("fn foo() {} fn bar() {}").unwrap();
        assert_eq!(program.items.len(), 2);
    }

    // === Expressions ===

    #[test]
    fn test_arithmetic_precedence() {
        let stmts = parse_fn_body("fn main() { 1 + 2 * 3; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        // Should be Add(1, Mul(2, 3))
        let Expr::BinaryOp {
            op, left, right, ..
        } = expr
        else {
            panic!("expected BinaryOp");
        };
        assert_eq!(*op, BinOp::Add);
        assert!(matches!(**left, Expr::IntLiteral(1, _)));
        let Expr::BinaryOp { op: inner_op, .. } = right.as_ref() else {
            panic!("expected Mul on right");
        };
        assert_eq!(*inner_op, BinOp::Mul);
    }

    #[test]
    fn test_grouping() {
        let stmts = parse_fn_body("fn main() { (1 + 2) * 3; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::BinaryOp { op, .. } = expr else {
            panic!("expected Mul at top");
        };
        assert_eq!(*op, BinOp::Mul);
    }

    #[test]
    fn test_unary_negation() {
        let stmts = parse_fn_body("fn main() { -42; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::UnaryOp {
            op, expr: inner, ..
        } = expr
        else {
            panic!("expected UnaryOp");
        };
        assert_eq!(*op, UnaryOp::Neg);
        assert!(matches!(**inner, Expr::IntLiteral(42, _)));
    }

    #[test]
    fn test_unary_not() {
        let stmts = parse_fn_body("fn main() { !true; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::UnaryOp { op, .. } = expr else {
            panic!("expected UnaryOp");
        };
        assert_eq!(*op, UnaryOp::Not);
    }

    #[test]
    fn test_comparison_operators() {
        let stmts = parse_fn_body("fn main() { 1 < 2; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::BinaryOp { op, .. } = expr else {
            panic!("expected BinaryOp");
        };
        assert_eq!(*op, BinOp::Lt);
    }

    #[test]
    fn test_logical_operators() {
        let stmts = parse_fn_body("fn main() { true && false || true; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        // || has lower precedence than &&, so: Or(And(true, false), true)
        let Expr::BinaryOp { op, .. } = expr else {
            panic!("expected Or at top");
        };
        assert_eq!(*op, BinOp::Or);
    }

    // === Function calls ===

    #[test]
    fn test_function_call() {
        let stmts = parse_fn_body("fn main() { foo(1, 2); }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::Call { callee, args, .. } = expr else {
            panic!("expected Call");
        };
        assert!(matches!(callee.as_ref(), Expr::Ident(name, _) if name == "foo"));
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_function_call_no_args() {
        let stmts = parse_fn_body("fn main() { foo(); }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::Call { args, .. } = expr else {
            panic!("expected Call");
        };
        assert!(args.is_empty());
    }

    // === Macro calls ===

    #[test]
    fn test_println_macro() {
        let stmts = parse_fn_body(r#"fn main() { println!("hello {}", x); }"#);
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::MacroCall { name, args, .. } = expr else {
            panic!("expected MacroCall");
        };
        assert_eq!(name, "println");
        assert_eq!(args.len(), 2);
    }

    // === If expressions ===

    #[test]
    fn test_if_expr() {
        let stmts = parse_fn_body("fn main() { if true { 1; } }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::If { else_block, .. } = expr else {
            panic!("expected If");
        };
        assert!(else_block.is_none());
    }

    #[test]
    fn test_if_else_expr() {
        let stmts = parse_fn_body("fn main() { if true { 1; } else { 2; } }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::If { else_block, .. } = expr else {
            panic!("expected If");
        };
        assert!(else_block.is_some());
    }

    #[test]
    fn test_if_else_if() {
        let stmts = parse_fn_body("fn main() { if true { 1; } else if false { 2; } else { 3; } }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::If { else_block, .. } = expr else {
            panic!("expected If");
        };
        assert!(matches!(else_block.as_deref(), Some(Expr::If { .. })));
    }

    // === Block expressions ===

    #[test]
    fn test_block_as_value() {
        let stmts = parse_fn_body("fn main() { let x = { 42 }; }");
        let Stmt::Let {
            value: Some(expr), ..
        } = &stmts[0]
        else {
            panic!("expected let with block value");
        };
        assert!(matches!(expr, Expr::Block(_)));
    }

    // === Assignment ===

    #[test]
    fn test_assignment() {
        let stmts = parse_fn_body("fn main() { x = 42; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        assert!(matches!(expr, Expr::Assign { .. }));
    }

    #[test]
    fn test_compound_assignment() {
        let stmts = parse_fn_body("fn main() { x += 1; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::CompoundAssign { op, .. } = expr else {
            panic!("expected compound assignment");
        };
        assert_eq!(*op, BinOp::Add);
    }

    // === Return ===

    #[test]
    fn test_return_value() {
        let stmts = parse_fn_body("fn main() { return 42; }");
        let Stmt::Return { value, .. } = &stmts[0] else {
            panic!("expected return");
        };
        assert!(value.is_some());
    }

    #[test]
    fn test_return_void() {
        let stmts = parse_fn_body("fn main() { return; }");
        let Stmt::Return { value, .. } = &stmts[0] else {
            panic!("expected return");
        };
        assert!(value.is_none());
    }

    // === Tail expressions ===

    #[test]
    fn test_tail_expression() {
        let stmts = parse_fn_body("fn add(a: i64, b: i64) -> i64 { a + b }");
        assert_eq!(stmts.len(), 1);
        let Stmt::Expr { has_semicolon, .. } = &stmts[0] else {
            panic!("expected expression statement");
        };
        assert!(!has_semicolon, "tail expression should not have semicolon");
    }

    // === Pretty print ===

    #[test]
    fn test_pretty_print() {
        let program = parse("fn main() { let x: i64 = 1 + 2; }").unwrap();
        let output = program.pretty_print();
        assert!(output.contains("fn main()"));
        assert!(output.contains("let x: i64 = (1 + 2);"));
    }

    // === Error cases ===

    #[test]
    fn test_missing_semicolon_in_let() {
        let result = parse("fn main() { let x = 42 }");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected ';'"));
    }

    #[test]
    fn test_missing_rbrace() {
        let result = parse("fn main() { let x = 42;");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_fn_name() {
        let result = parse("fn () {}");
        assert!(result.is_err());
    }

    // === Reference syntax (parsed but ignored) ===

    #[test]
    fn test_reference_in_param() {
        let f = parse_fn("fn foo(x: &i64) {}");
        assert_eq!(f.params[0].type_ann.name, "i64");
    }

    #[test]
    fn test_ref_expr() {
        let stmts = parse_fn_body("fn main() { &x; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        assert!(matches!(
            expr,
            Expr::UnaryOp {
                op: UnaryOp::Ref,
                ..
            }
        ));
    }

    // === Full program ===

    #[test]
    fn test_full_program() {
        let src = r#"
fn add(a: i64, b: i64) -> i64 {
    a + b
}

fn main() {
    let x: i64 = 10;
    let y: i64 = 20;
    let result = add(x, y);
    println!("Result: {}", result);
}
"#;
        let program = parse(src).unwrap();
        assert_eq!(program.items.len(), 2);

        let Item::Function(f0) = &program.items[0] else {
            panic!("expected function item");
        };
        assert_eq!(f0.name, "add");
        assert_eq!(f0.params.len(), 2);

        let Item::Function(f1) = &program.items[1] else {
            panic!("expected function item");
        };
        assert_eq!(f1.name, "main");
        assert_eq!(f1.body.stmts.len(), 4);
    }

    // === String and bool literals ===

    #[test]
    fn test_string_literal_expr() {
        let stmts = parse_fn_body(r#"fn main() { "hello"; }"#);
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!("expected expr stmt");
        };
        assert!(matches!(expr, Expr::StringLiteral(s, _) if s == "hello"));
    }

    #[test]
    fn test_bool_literal_expr() {
        let stmts = parse_fn_body("fn main() { true; false; }");
        assert_eq!(stmts.len(), 2);
    }

    // === Phase 5: Control Flow ===

    #[test]
    fn test_while_stmt() {
        let stmts = parse_fn_body("fn main() { while x > 0 { x -= 1; } }");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Stmt::While { .. }));
    }

    #[test]
    fn test_loop_stmt() {
        let stmts = parse_fn_body("fn main() { loop { break; } }");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Stmt::Loop { .. }));
    }

    #[test]
    fn test_for_stmt() {
        let stmts = parse_fn_body("fn main() { for i in 0..10 { println!(\"{}\", i); } }");
        assert_eq!(stmts.len(), 1);
        let Stmt::For { name, iterable, .. } = &stmts[0] else {
            panic!("expected for");
        };
        assert_eq!(name, "i");
        assert!(matches!(
            iterable.as_ref(),
            Expr::Range {
                inclusive: false,
                ..
            }
        ));
    }

    #[test]
    fn test_for_inclusive_range() {
        let stmts = parse_fn_body("fn main() { for i in 0..=10 { x; } }");
        let Stmt::For { iterable, .. } = &stmts[0] else {
            panic!();
        };
        assert!(matches!(
            iterable.as_ref(),
            Expr::Range {
                inclusive: true,
                ..
            }
        ));
    }

    #[test]
    fn test_break_stmt() {
        let stmts = parse_fn_body("fn main() { break; }");
        assert!(matches!(&stmts[0], Stmt::Break { value: None, .. }));
    }

    #[test]
    fn test_break_with_value() {
        let stmts = parse_fn_body("fn main() { break 42; }");
        assert!(matches!(&stmts[0], Stmt::Break { value: Some(_), .. }));
    }

    #[test]
    fn test_continue_stmt() {
        let stmts = parse_fn_body("fn main() { continue; }");
        assert!(matches!(&stmts[0], Stmt::Continue { .. }));
    }

    #[test]
    fn test_match_expr() {
        let stmts = parse_fn_body(r#"fn main() { match x { 1 => "one", _ => "other" }; }"#);
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Match { arms, .. } = expr else {
            panic!();
        };
        assert_eq!(arms.len(), 2);
        assert!(matches!(&arms[0].pattern, Pattern::Literal(_)));
        assert!(matches!(&arms[1].pattern, Pattern::Wildcard(_)));
    }

    #[test]
    fn test_match_with_blocks() {
        let stmts = parse_fn_body(
            r#"fn main() { match x { 1 => { println!("one"); } _ => { println!("other"); } }; }"#,
        );
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        assert!(matches!(expr, Expr::Match { .. }));
    }

    #[test]
    fn test_match_variable_pattern() {
        let stmts = parse_fn_body("fn main() { match x { n => n + 1 }; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Match { arms, .. } = expr else {
            panic!();
        };
        assert!(matches!(&arms[0].pattern, Pattern::Ident(name, _) if name == "n"));
    }

    #[test]
    fn test_range_expression() {
        let stmts = parse_fn_body("fn main() { 0..10; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        assert!(matches!(
            expr,
            Expr::Range {
                inclusive: false,
                ..
            }
        ));
    }

    #[test]
    fn test_range_inclusive_expression() {
        let stmts = parse_fn_body("fn main() { 0..=10; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        assert!(matches!(
            expr,
            Expr::Range {
                inclusive: true,
                ..
            }
        ));
    }

    // === Phase 6: Collections & Strings ===

    #[test]
    fn test_array_literal() {
        let stmts = parse_fn_body("fn main() { [1, 2, 3]; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Array { elements, .. } = expr else {
            panic!("expected Array");
        };
        assert_eq!(elements.len(), 3);
    }

    #[test]
    fn test_empty_array() {
        let stmts = parse_fn_body("fn main() { []; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Array { elements, .. } = expr else {
            panic!("expected Array");
        };
        assert_eq!(elements.len(), 0);
    }

    #[test]
    fn test_index_expr() {
        let stmts = parse_fn_body("fn main() { v[0]; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        assert!(matches!(expr, Expr::Index { .. }));
    }

    #[test]
    fn test_method_call() {
        let stmts = parse_fn_body("fn main() { v.push(1); }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::MethodCall { method, args, .. } = expr else {
            panic!("expected MethodCall");
        };
        assert_eq!(method, "push");
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn test_field_access() {
        let stmts = parse_fn_body("fn main() { t.0; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::FieldAccess { field, .. } = expr else {
            panic!("expected FieldAccess");
        };
        assert_eq!(field, "0");
    }

    #[test]
    fn test_tuple_literal() {
        let stmts = parse_fn_body("fn main() { (1, 2, 3); }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Tuple { elements, .. } = expr else {
            panic!("expected Tuple, got {expr:?}");
        };
        assert_eq!(elements.len(), 3);
    }

    #[test]
    fn test_single_element_tuple() {
        let stmts = parse_fn_body("fn main() { (42,); }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Tuple { elements, .. } = expr else {
            panic!("expected Tuple");
        };
        assert_eq!(elements.len(), 1);
    }

    #[test]
    fn test_grouped_expr_not_tuple() {
        let stmts = parse_fn_body("fn main() { (42); }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        assert!(matches!(expr, Expr::Grouped(_, _)));
    }

    #[test]
    fn test_empty_tuple() {
        let stmts = parse_fn_body("fn main() { (); }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Tuple { elements, .. } = expr else {
            panic!("expected Tuple");
        };
        assert_eq!(elements.len(), 0);
    }

    #[test]
    fn test_vec_macro_brackets() {
        let stmts = parse_fn_body("fn main() { vec![1, 2, 3]; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::MacroCall { name, args, .. } = expr else {
            panic!("expected MacroCall");
        };
        assert_eq!(name, "vec");
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn test_chained_method_calls() {
        let stmts = parse_fn_body(r#"fn main() { s.trim().to_uppercase(); }"#);
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::MethodCall { method, object, .. } = expr else {
            panic!("expected MethodCall");
        };
        assert_eq!(method, "to_uppercase");
        assert!(matches!(object.as_ref(), Expr::MethodCall { method, .. } if method == "trim"));
    }

    #[test]
    fn test_chained_index() {
        let stmts = parse_fn_body("fn main() { v[0][1]; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Index { object, .. } = expr else {
            panic!("expected Index");
        };
        assert!(matches!(object.as_ref(), Expr::Index { .. }));
    }

    // === Phase 7: Struct/Enum/Impl parsing ===

    #[test]
    fn test_struct_def() {
        let program = parse(
            r#"
struct Point {
    x: f64,
    y: f64,
}
fn main() {}
"#,
        )
        .unwrap();
        let Item::Struct(s) = &program.items[0] else {
            panic!("expected struct");
        };
        assert_eq!(s.name, "Point");
        assert!(matches!(&s.kind, StructKind::Named(fields) if fields.len() == 2));
    }

    #[test]
    fn test_enum_def() {
        let program = parse(
            r#"
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
    Point,
}
fn main() {}
"#,
        )
        .unwrap();
        let Item::Enum(e) = &program.items[0] else {
            panic!("expected enum");
        };
        assert_eq!(e.name, "Shape");
        assert_eq!(e.variants.len(), 3);
        assert!(matches!(&e.variants[0].kind, EnumVariantKind::Tuple(t) if t.len() == 1));
        assert!(matches!(&e.variants[2].kind, EnumVariantKind::Unit));
    }

    #[test]
    fn test_impl_block() {
        let program = parse(
            r#"
struct Point { x: f64, y: f64 }

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }
}
fn main() {}
"#,
        )
        .unwrap();
        let Item::Impl(i) = &program.items[1] else {
            panic!("expected impl");
        };
        assert_eq!(i.type_name, "Point");
        assert_eq!(i.methods.len(), 1);
        assert_eq!(i.methods[0].name, "new");
    }

    #[test]
    fn test_struct_init_expr() {
        let stmts = parse_fn_body("fn main() { let p = Point { x: 1.0, y: 2.0 }; }");
        let Stmt::Let {
            value: Some(expr), ..
        } = &stmts[0]
        else {
            panic!("expected let with value");
        };
        assert!(matches!(expr, Expr::StructInit { name, .. } if name == "Point"));
    }

    #[test]
    fn test_path_call_expr() {
        let stmts = parse_fn_body("fn main() { Point::new(1.0, 2.0); }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        assert!(matches!(expr, Expr::PathCall { path, .. } if path == &["Point", "new"]));
    }

    #[test]
    fn test_path_expr() {
        let stmts = parse_fn_body("fn main() { Color::Red; }");
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        assert!(matches!(expr, Expr::Path { segments, .. } if segments == &["Color", "Red"]));
    }

    #[test]
    fn test_self_method_param() {
        let program = parse(
            r#"
impl Foo {
    fn bar(&self) -> i64 {
        42
    }
}
fn main() {}
"#,
        )
        .unwrap();
        let Item::Impl(i) = &program.items[0] else {
            panic!("expected impl");
        };
        assert_eq!(i.methods[0].params[0].name, "self");
    }

    #[test]
    fn test_enum_variant_pattern() {
        let stmts = parse_fn_body(
            r#"fn main() {
    match x {
        Shape::Circle(r) => r,
        _ => 0.0,
    };
}"#,
        );
        let Stmt::Expr { expr, .. } = &stmts[0] else {
            panic!();
        };
        let Expr::Match { arms, .. } = expr else {
            panic!("expected match");
        };
        assert!(matches!(
            &arms[0].pattern,
            Pattern::EnumVariant {
                enum_name,
                variant,
                ..
            } if enum_name == "Shape" && variant == "Circle"
        ));
    }

    #[test]
    fn test_unit_struct_def() {
        let program = parse("struct Marker;\nfn main() {}").unwrap();
        let Item::Struct(s) = &program.items[0] else {
            panic!("expected struct");
        };
        assert_eq!(s.name, "Marker");
        assert!(matches!(s.kind, StructKind::Unit));
    }

    #[test]
    fn test_tuple_struct_def() {
        let program = parse("struct Pair(i64, i64);\nfn main() {}").unwrap();
        let Item::Struct(s) = &program.items[0] else {
            panic!("expected struct");
        };
        assert_eq!(s.name, "Pair");
        assert!(matches!(&s.kind, StructKind::Tuple(t) if t.len() == 2));
    }

    // === Phase 8: Traits & Generics ===

    #[test]
    fn test_trait_def() {
        let program = parse("trait Greet { fn greet(&self) -> String; }\nfn main() {}").unwrap();
        let Item::Trait(t) = &program.items[0] else {
            panic!("expected trait");
        };
        assert_eq!(t.name, "Greet");
        assert_eq!(t.methods.len(), 1);
        assert_eq!(t.methods[0].name, "greet");
    }

    #[test]
    fn test_trait_with_default_method() {
        let program = parse(
            r#"trait Foo { fn bar(&self) -> i64 { 42 } }
fn main() {}"#,
        )
        .unwrap();
        let Item::Trait(t) = &program.items[0] else {
            panic!("expected trait");
        };
        assert_eq!(t.name, "Foo");
        assert_eq!(t.methods.len(), 0);
        assert_eq!(t.default_methods.len(), 1);
        assert_eq!(t.default_methods[0].name, "bar");
    }

    #[test]
    fn test_impl_trait_for_type() {
        let program = parse(
            r#"trait Greet { fn greet(&self) -> String; }
struct Person { name: String }
impl Greet for Person { fn greet(&self) -> String { self.name } }
fn main() {}"#,
        )
        .unwrap();
        assert!(matches!(&program.items[0], Item::Trait(_)));
        let Item::ImplTrait(i) = &program.items[2] else {
            panic!("expected impl trait block");
        };
        assert_eq!(i.trait_name, "Greet");
        assert_eq!(i.type_name, "Person");
        assert_eq!(i.methods.len(), 1);
    }

    #[test]
    fn test_generic_fn_def() {
        let program = parse("fn identity<T>(x: T) -> T { x }\nfn main() {}").unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        assert_eq!(f.name, "identity");
        assert_eq!(f.generic_params.len(), 1);
        assert_eq!(f.generic_params[0].name, "T");
        assert!(f.generic_params[0].bounds.is_empty());
    }

    #[test]
    fn test_generic_fn_with_bounds() {
        let program =
            parse("fn print_val<T: Display>(x: T) { println!(\"{}\", x); }\nfn main() {}").unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        assert_eq!(f.generic_params.len(), 1);
        assert_eq!(f.generic_params[0].name, "T");
        assert_eq!(f.generic_params[0].bounds, vec!["Display"]);
    }

    #[test]
    fn test_generic_fn_multiple_params() {
        let program = parse("fn foo<A, B: Clone + Debug>(a: A, b: B) { }\nfn main() {}").unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        assert_eq!(f.generic_params.len(), 2);
        assert_eq!(f.generic_params[0].name, "A");
        assert!(f.generic_params[0].bounds.is_empty());
        assert_eq!(f.generic_params[1].name, "B");
        assert_eq!(f.generic_params[1].bounds, vec!["Clone", "Debug"]);
    }

    #[test]
    fn test_impl_trait_for_add() {
        let program = parse(
            r#"struct Vec2 { x: f64, y: f64 }
impl Add for Vec2 { fn add(&self, other: &Vec2) -> Vec2 { Vec2 { x: 0.0, y: 0.0 } } }
fn main() {}"#,
        )
        .unwrap();
        let Item::ImplTrait(i) = &program.items[1] else {
            panic!("expected impl trait block");
        };
        assert_eq!(i.trait_name, "Add");
        assert_eq!(i.type_name, "Vec2");
    }

    // === Phase 9: Error Handling ===

    #[test]
    fn test_if_let_expr() {
        let program =
            parse(r#"fn main() { if let Some(x) = foo() { println!("{}", x); } }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Expr { expr, .. } = &f.body.stmts[0] else {
            panic!("expected expr stmt");
        };
        assert!(matches!(expr, Expr::IfLet { .. }));
    }

    #[test]
    fn test_if_let_with_else() {
        let program = parse(r#"fn main() { if let Some(x) = foo() { x } else { 0 } }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Expr { expr, .. } = &f.body.stmts[0] else {
            panic!("expected expr stmt");
        };
        let Expr::IfLet { else_block, .. } = expr else {
            panic!("expected if let");
        };
        assert!(else_block.is_some());
    }

    #[test]
    fn test_while_let_stmt() {
        let program =
            parse(r#"fn main() { while let Some(x) = v.pop() { println!("{}", x); } }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        assert!(matches!(&f.body.stmts[0], Stmt::WhileLet { .. }));
    }

    #[test]
    fn test_try_operator() {
        let program = parse(r#"fn main() { let x = foo()?; }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Let {
            value: Some(expr), ..
        } = &f.body.stmts[0]
        else {
            panic!("expected let with value");
        };
        assert!(matches!(expr, Expr::Try { .. }));
    }

    #[test]
    fn test_some_none_pattern() {
        let program = parse(r#"fn main() { match x { Some(v) => v, None => 0, } }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Expr { expr, .. } = &f.body.stmts[0] else {
            panic!("expected expr");
        };
        let Expr::Match { arms, .. } = expr else {
            panic!("expected match");
        };
        assert!(matches!(
            &arms[0].pattern,
            Pattern::EnumVariant {
                enum_name,
                variant,
                ..
            } if enum_name == "Option" && variant == "Some"
        ));
        assert!(matches!(
            &arms[1].pattern,
            Pattern::EnumVariant {
                enum_name,
                variant,
                ..
            } if enum_name == "Option" && variant == "None"
        ));
    }

    #[test]
    fn test_ok_err_pattern() {
        let program = parse(r#"fn main() { match x { Ok(v) => v, Err(e) => 0, } }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Expr { expr, .. } = &f.body.stmts[0] else {
            panic!("expected expr");
        };
        let Expr::Match { arms, .. } = expr else {
            panic!("expected match");
        };
        assert!(matches!(
            &arms[0].pattern,
            Pattern::EnumVariant {
                enum_name,
                variant,
                ..
            } if enum_name == "Result" && variant == "Ok"
        ));
        assert!(matches!(
            &arms[1].pattern,
            Pattern::EnumVariant {
                enum_name,
                variant,
                ..
            } if enum_name == "Result" && variant == "Err"
        ));
    }

    // === Phase 10: Closures ===

    #[test]
    fn test_closure_with_params() {
        let program = parse(r#"fn main() { let f = |x, y| x + y; }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Let {
            value: Some(expr), ..
        } = &f.body.stmts[0]
        else {
            panic!("expected let");
        };
        let Expr::Closure { params, .. } = expr else {
            panic!("expected closure");
        };
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "x");
        assert_eq!(params[1].name, "y");
    }

    #[test]
    fn test_closure_with_types() {
        let program = parse(r#"fn main() { let f = |x: i64| -> i64 { x * 2 }; }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Let {
            value: Some(expr), ..
        } = &f.body.stmts[0]
        else {
            panic!("expected let");
        };
        let Expr::Closure {
            params,
            return_type,
            ..
        } = expr
        else {
            panic!("expected closure");
        };
        assert_eq!(params.len(), 1);
        assert!(params[0].type_ann.is_some());
        assert!(return_type.is_some());
    }

    #[test]
    fn test_empty_closure() {
        let program = parse(r#"fn main() { let f = || 42; }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Let {
            value: Some(expr), ..
        } = &f.body.stmts[0]
        else {
            panic!("expected let");
        };
        let Expr::Closure { params, .. } = expr else {
            panic!("expected closure");
        };
        assert!(params.is_empty());
    }

    #[test]
    fn test_move_closure() {
        let program = parse(r#"fn main() { let f = move |x| x; }"#).unwrap();
        let Item::Function(f) = &program.items[0] else {
            panic!("expected function");
        };
        let Stmt::Let {
            value: Some(expr), ..
        } = &f.body.stmts[0]
        else {
            panic!("expected let");
        };
        assert!(matches!(expr, Expr::Closure { .. }));
    }

    // === Phase 11: Modules & Use Statements ===

    #[test]
    fn test_inline_module() {
        let program =
            parse("mod math { fn add(a: i64, b: i64) -> i64 { a + b } } fn main() {}").unwrap();
        assert!(
            matches!(&program.items[0], Item::Module(m) if m.name == "math" && m.body.is_some())
        );
    }

    #[test]
    fn test_file_module() {
        let program = parse("mod utils; fn main() {}").unwrap();
        let Item::Module(m) = &program.items[0] else {
            panic!("expected module");
        };
        assert_eq!(m.name, "utils");
        assert!(m.body.is_none()); // file-based
    }

    #[test]
    fn test_use_simple() {
        let program = parse("use math::add; fn main() {}").unwrap();
        let Item::Use(u) = &program.items[0] else {
            panic!("expected use");
        };
        assert_eq!(u.path, vec!["math", "add"]);
        assert!(matches!(u.tree, UseTree::Simple));
    }

    #[test]
    fn test_use_glob() {
        let program = parse("use math::*; fn main() {}").unwrap();
        let Item::Use(u) = &program.items[0] else {
            panic!("expected use");
        };
        assert_eq!(u.path, vec!["math"]);
        assert!(matches!(u.tree, UseTree::Glob));
    }

    #[test]
    fn test_use_group() {
        let program = parse("use math::{add, sub}; fn main() {}").unwrap();
        let Item::Use(u) = &program.items[0] else {
            panic!("expected use");
        };
        assert_eq!(u.path, vec!["math"]);
        let UseTree::Group(names) = &u.tree else {
            panic!("expected group");
        };
        assert_eq!(names, &["add", "sub"]);
    }

    #[test]
    fn test_pub_module() {
        let program =
            parse("pub mod math { fn add(a: i64, b: i64) -> i64 { a + b } } fn main() {}").unwrap();
        let Item::Module(m) = &program.items[0] else {
            panic!("expected module");
        };
        assert!(m.is_pub);
    }

    #[test]
    fn test_pub_fn() {
        let program = parse("pub fn helper() -> i64 { 42 } fn main() {}").unwrap();
        assert!(matches!(&program.items[0], Item::Function(_)));
    }

    #[test]
    fn test_type_alias() {
        let program = parse("type Meters = f64; fn main() {}").unwrap();
        let Item::TypeAlias { name, target, .. } = &program.items[0] else {
            panic!("expected type alias");
        };
        assert_eq!(name, "Meters");
        assert_eq!(target.name, "f64");
    }

    #[test]
    fn test_const_def() {
        let program = parse("const MAX: i64 = 100; fn main() {}").unwrap();
        let Item::Const {
            name,
            type_ann,
            is_static,
            ..
        } = &program.items[0]
        else {
            panic!("expected const");
        };
        assert_eq!(name, "MAX");
        assert!(!is_static);
        assert_eq!(type_ann.as_ref().unwrap().name, "i64");
    }

    #[test]
    fn test_static_def() {
        let program = parse("static COUNT: i64 = 0; fn main() {}").unwrap();
        let Item::Const {
            name, is_static, ..
        } = &program.items[0]
        else {
            panic!("expected static");
        };
        assert_eq!(name, "COUNT");
        assert!(is_static);
    }

    #[test]
    fn test_for_destructure() {
        let body = parse_fn_body("fn main() { for (k, v) in items { println!(\"{}\", k); } }");
        assert!(matches!(&body[0], Stmt::ForDestructure { .. }));
        if let Stmt::ForDestructure { names, .. } = &body[0] {
            assert_eq!(names, &["k", "v"]);
        }
    }
}
