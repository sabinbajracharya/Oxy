//! Item types: function, struct, enum, impl, trait, module, and use definitions.
//!
//! Extracted from [`super`] to keep mod.rs under ~300 lines.

use super::expr::Expr;
use super::{Block, GenericParam, TypeAnnotation, Visibility};
use crate::lexer::Span;

/// A top-level item (function, struct, enum, etc.).
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// A function definition: `fn name(params) -> Type { body }`
    Function(FnDef),
    /// A struct definition: `struct Name { ... }`
    Struct(StructDef),
    /// An enum definition: `enum Name { ... }`
    Enum(EnumDef),
    /// An inherent impl block: `impl Type { ... }`
    Impl(ImplBlock),
    /// A trait definition: `trait Name { ... }`
    Trait(TraitDef),
    /// A trait implementation: `impl Trait for Type { ... }`
    ImplTrait(ImplTraitBlock),
    /// `mod name { items }` or `mod name;` (file-based)
    Module(ModuleDef),
    /// `use path::item;` or `use path::*;` or `use path::{a, b};`
    Use(UseDef),
    /// `type Name = Type;`
    TypeAlias {
        name: String,
        target: TypeAnnotation,
        span: Span,
    },
    /// `const NAME: Type = expr;`
    Const {
        name: String,
        type_ann: Option<TypeAnnotation>,
        value: Expr,
        span: Span,
    },
}

impl Item {
    /// Returns the source span of this item.
    pub fn span(&self) -> Span {
        match self {
            Item::Function(f) => f.span,
            Item::Struct(s) => s.span,
            Item::Enum(e) => e.span,
            Item::Impl(i) => i.span,
            Item::Trait(t) => t.span,
            Item::ImplTrait(i) => i.span,
            Item::Module(m) => m.span,
            Item::Use(u) => u.span,
            Item::TypeAlias { span, .. } => *span,
            Item::Const { span, .. } => *span,
        }
    }
}

/// A function definition.
#[derive(Debug, Clone, PartialEq)]
pub struct FnDef {
    pub name: String,
    pub is_async: bool,
    pub generic_params: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub span: Span,
}

/// A function parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_ann: TypeAnnotation,
    /// `true` if declared as `mut param: T` (or `mut self`). Mirrors `let mut x`.
    pub is_mut: bool,
    pub span: Span,
}

/// A closure parameter (type annotation is optional).
#[derive(Debug, Clone, PartialEq)]
pub struct ClosureParam {
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
    pub span: Span,
}

/// An attribute on an item, e.g. `#[derive(Debug, Clone)]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<String>,
    pub span: Span,
}

/// A struct definition: `struct Name { field: Type, ... }` or `struct Name(Type, ...);`
#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub attributes: Vec<Attribute>,
    pub kind: StructKind,
    pub visibility: Visibility,
    pub span: Span,
}

/// Whether a struct has named fields, is a tuple struct, or is a unit struct.
#[derive(Debug, Clone, PartialEq)]
pub enum StructKind {
    /// Named fields: `struct S { x: i64, y: i64 }`
    Named(Vec<StructField>),
    /// Tuple struct: `struct S(i64, i64);`
    Tuple(Vec<TypeAnnotation>),
    /// Unit struct: `struct S;`
    Unit,
}

/// A named struct field.
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub visibility: Visibility,
    pub span: Span,
}

/// An enum definition: `enum Name { Variant, Variant(Type), Variant { field: Type } }`
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub attributes: Vec<Attribute>,
    pub variants: Vec<EnumVariant>,
    pub visibility: Visibility,
    pub span: Span,
}

/// A single enum variant.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub kind: EnumVariantKind,
    pub span: Span,
}

/// The data a variant carries.
#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariantKind {
    /// Unit variant: `Variant`
    Unit,
    /// Tuple variant: `Variant(Type, ...)`
    Tuple(Vec<TypeAnnotation>),
    /// Struct variant: `Variant { field: Type, ... }`
    Struct(Vec<StructField>),
}

/// The base type name a `type_name` string refers to, with any generic-argument
/// suffix stripped (`Cell<T>` → `Cell`, `TwoBox<A, B>` → `TwoBox`).
///
/// Method dispatch keys use the base name *everywhere*: at runtime the method
/// resolver builds `Type::method` from a value's base struct name (which never
/// carries type args) and `oxy_path_call_builtin` builds it from `::`-joined
/// path segments. Registering an impl's methods under a generic-laden name like
/// `Cell<T>::make` therefore makes them unreachable. The impl's own generic
/// params are already merged into each method's `generic_params` by the parser,
/// so the `<…>` in `type_name` is redundant for dispatch.
pub(crate) fn base_type_name(type_name: &str) -> &str {
    match type_name.find('<') {
        Some(i) => &type_name[..i],
        None => type_name,
    }
}

/// An impl block: `impl Name { fn ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct ImplBlock {
    pub type_name: String,
    pub methods: Vec<FnDef>,
    pub span: Span,
}

impl ImplBlock {
    /// The base type these methods attach to (dispatch key), generics stripped.
    pub(crate) fn base_type_name(&self) -> &str {
        base_type_name(&self.type_name)
    }
}

/// A trait definition: `trait Name { fn method(&self) -> Type; }`
#[derive(Debug, Clone, PartialEq)]
pub struct TraitDef {
    pub name: String,
    pub methods: Vec<TraitMethodSig>,
    pub default_methods: Vec<FnDef>,
    pub visibility: Visibility,
    pub span: Span,
}

/// A trait method signature (no body): `fn method(&self, x: i64) -> i64;`
#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethodSig {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub span: Span,
}

/// An impl-trait block: `impl Trait for Type { fn ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct ImplTraitBlock {
    pub trait_name: String,
    pub type_name: String,
    pub methods: Vec<FnDef>,
    pub span: Span,
}

impl ImplTraitBlock {
    /// The base type these methods attach to (dispatch key), generics stripped.
    pub(crate) fn base_type_name(&self) -> &str {
        base_type_name(&self.type_name)
    }
}

/// An inline or file-based module definition.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDef {
    pub name: String,
    pub visibility: Visibility,
    /// `Some(items)` for inline modules, `None` for file-based (`mod name;`).
    pub body: Option<Vec<Item>>,
    pub span: Span,
}

/// A `use` declaration to import items from a module path.
#[derive(Debug, Clone, PartialEq)]
pub struct UseDef {
    /// Path segments: `["std", "collections"]` for `use std::collections::...`
    pub path: Vec<String>,
    /// What to import from the path.
    pub tree: UseTree,
    /// Whether this is a `pub use` (re-export) and at what visibility.
    pub visibility: Visibility,
    pub span: Span,
}

/// What to import from a use path.
#[derive(Debug, Clone, PartialEq)]
pub enum UseTree {
    /// Import a single item: `use path::item;` or `use path::item as alias;`
    Simple(Option<String>),
    /// Glob import: `use path::*;`
    Glob,
    /// Multiple imports: `use path::{a, b, c};` with optional `as` aliases
    Group(Vec<(String, Option<String>)>),
}
impl Item {
    pub(crate) fn pretty_print(&self, out: &mut String, indent: usize) {
        match self {
            Item::Function(f) => f.pretty_print(out, indent),
            Item::Struct(s) => {
                let pad = "  ".repeat(indent);
                out.push_str(&format!("{pad}struct {}", s.name));
                match &s.kind {
                    StructKind::Named(fields) => {
                        out.push_str(" {\n");
                        for f in fields {
                            out.push_str(&format!("{pad}  {}: {},\n", f.name, f.type_ann.name()));
                        }
                        out.push_str(&format!("{pad}}}\n"));
                    }
                    StructKind::Tuple(types) => {
                        out.push('(');
                        for (i, t) in types.iter().enumerate() {
                            if i > 0 {
                                out.push_str(", ");
                            }
                            out.push_str(t.name());
                        }
                        out.push_str(");\n");
                    }
                    StructKind::Unit => out.push_str(";\n"),
                }
            }
            Item::Enum(e) => {
                let pad = "  ".repeat(indent);
                out.push_str(&format!("{pad}enum {} {{\n", e.name));
                for v in &e.variants {
                    out.push_str(&format!("{pad}  {}", v.name));
                    match &v.kind {
                        EnumVariantKind::Unit => {}
                        EnumVariantKind::Tuple(types) => {
                            out.push('(');
                            for (i, t) in types.iter().enumerate() {
                                if i > 0 {
                                    out.push_str(", ");
                                }
                                out.push_str(t.name());
                            }
                            out.push(')');
                        }
                        EnumVariantKind::Struct(fields) => {
                            out.push_str(" { ");
                            for (i, f) in fields.iter().enumerate() {
                                if i > 0 {
                                    out.push_str(", ");
                                }
                                out.push_str(&format!("{}: {}", f.name, f.type_ann.name()));
                            }
                            out.push_str(" }");
                        }
                    }
                    out.push_str(",\n");
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            Item::Impl(i) => {
                let pad = "  ".repeat(indent);
                out.push_str(&format!("{pad}impl {} {{\n", i.type_name));
                for m in &i.methods {
                    m.pretty_print(out, indent + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            Item::Trait(t) => {
                let pad = "  ".repeat(indent);
                out.push_str(&format!("{pad}trait {} {{\n", t.name));
                for sig in &t.methods {
                    out.push_str(&format!("{pad}  fn {}(", sig.name));
                    for (i, p) in sig.params.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        out.push_str(&format!("{}: {}", p.name, p.type_ann.name()));
                    }
                    out.push(')');
                    if let Some(ret) = &sig.return_type {
                        out.push_str(&format!(" -> {}", ret.name()));
                    }
                    out.push_str(";\n");
                }
                for m in &t.default_methods {
                    m.pretty_print(out, indent + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            Item::ImplTrait(i) => {
                let pad = "  ".repeat(indent);
                out.push_str(&format!(
                    "{pad}impl {} for {} {{\n",
                    i.trait_name, i.type_name
                ));
                for m in &i.methods {
                    m.pretty_print(out, indent + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            Item::Module(m) => {
                let pad = "  ".repeat(indent);
                let pub_str = m.visibility.as_str();
                let pub_str = if pub_str.is_empty() {
                    String::new()
                } else {
                    format!("{pub_str} ")
                };
                if let Some(body) = &m.body {
                    out.push_str(&format!("{pub_str}{pad}mod {} {{\n", m.name));
                    for item in body {
                        item.pretty_print(out, indent + 1);
                    }
                    out.push_str(&format!("{pad}}}\n"));
                } else {
                    out.push_str(&format!("{pub_str}{pad}mod {};\n", m.name));
                }
            }
            Item::Use(u) => {
                let pad = "  ".repeat(indent);
                let pub_str = u.visibility.as_str();
                let pub_str = if pub_str.is_empty() {
                    String::new()
                } else {
                    format!("{pub_str} ")
                };
                out.push_str(&format!("{pad}{pub_str}use "));
                let path_str = u.path.join("::");
                match &u.tree {
                    UseTree::Simple(alias) => {
                        if let Some(a) = alias {
                            out.push_str(&format!("{path_str} as {a};\n"));
                        } else {
                            out.push_str(&format!("{path_str};\n"));
                        }
                    }
                    UseTree::Glob => out.push_str(&format!("{path_str}::*;\n")),
                    UseTree::Group(items) => {
                        let parts: Vec<String> = items
                            .iter()
                            .map(|(n, alias)| {
                                if let Some(a) = alias {
                                    format!("{n} as {a}")
                                } else {
                                    n.clone()
                                }
                            })
                            .collect();
                        out.push_str(&format!("{}::{{{}}};\n", path_str, parts.join(", ")));
                    }
                }
            }
            Item::TypeAlias { name, target, .. } => {
                let pad = "  ".repeat(indent);
                out.push_str(&format!("{pad}type {name} = {};\n", target.name()));
            }
            Item::Const { name, type_ann, .. } => {
                let pad = "  ".repeat(indent);
                if let Some(ta) = type_ann {
                    out.push_str(&format!("{pad}const {name}: {} = ...;\n", ta.name()));
                } else {
                    out.push_str(&format!("{pad}const {name} = ...;\n"));
                }
            }
        }
    }
}
impl FnDef {
    pub(crate) fn pretty_print(&self, out: &mut String, indent: usize) {
        let pad = "  ".repeat(indent);
        if self.is_async {
            out.push_str(&format!("{pad}async fn {}", self.name));
        } else {
            out.push_str(&format!("{pad}fn {}", self.name));
        }
        if !self.generic_params.is_empty() {
            out.push('<');
            for (i, gp) in self.generic_params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&gp.name);
                if !gp.bounds.is_empty() {
                    out.push_str(": ");
                    out.push_str(&gp.bounds.join(" + "));
                }
            }
            out.push('>');
        }
        out.push('(');
        for (i, p) in self.params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!("{}: {}", p.name, p.type_ann.name()));
        }
        out.push(')');
        if let Some(ret) = &self.return_type {
            out.push_str(&format!(" -> {}", ret.name()));
        }
        out.push_str(" {\n");
        for stmt in &self.body.stmts {
            stmt.pretty_print(out, indent + 1);
        }
        out.push_str(&format!("{pad}}}\n"));
    }
}
