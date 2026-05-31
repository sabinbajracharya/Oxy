//! Abstract Syntax Tree definitions for the Oxy language.
//!
//! Shared types live here. Item, expression, statement, and pattern types
//! are extracted into sub-modules:
//! - [`item`]: Item, FnDef, StructDef, EnumDef, ImplBlock, TraitDef, etc.
//! - [`stmt`]: Stmt, Block
//! - [`expr`]: Expr, BinOp, UnaryOp, MatchArm, FStringPart
//! - [`pattern`]: Pattern

use crate::lexer::Span;

pub mod expr;
pub mod item;
pub mod pattern;
pub mod stmt;

pub use expr::{BinOp, Expr, FStringPart, MatchArm, UnaryOp};
pub(crate) use item::base_type_name;
pub use item::{
    Attribute, ClosureParam, EnumDef, EnumVariant, EnumVariantKind, FnDef, ImplBlock,
    ImplTraitBlock, Item, ModuleDef, Param, StructDef, StructField, StructKind, TraitDef,
    TraitMethodSig, UseDef, UseTree,
};
pub use pattern::Pattern;
pub use stmt::{Block, Stmt};

/// Item visibility.
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    /// `pub` — visible everywhere.
    Pub,
    /// `pub(crate)` — visible within the current crate.
    PubCrate,
    /// `pub(super)` — visible in the parent module.
    PubSuper,
    /// Default — private to the current module.
    Private,
}

impl Visibility {
    /// True for `pub`, `pub(crate)`, `pub(super)` — item is publicly accessible.
    pub fn is_pub(&self) -> bool {
        matches!(
            self,
            Visibility::Pub | Visibility::PubCrate | Visibility::PubSuper
        )
    }
    /// True for everything except `Private` — item is visible outside its own module.
    pub fn is_visible(&self) -> bool {
        !matches!(self, Visibility::Private)
    }
    /// Display as the Rust-style keyword.
    pub fn as_str(&self) -> &str {
        match self {
            Visibility::Pub => "pub",
            Visibility::PubCrate => "pub(crate)",
            Visibility::PubSuper => "pub(super)",
            Visibility::Private => "",
        }
    }
}

/// A complete Oxy program — a sequence of top-level items.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
    pub span: Span,
}

/// A generic type parameter, e.g., `T` or `T: Display + Clone`
#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
    pub name: String,
    pub bounds: Vec<String>,
    pub span: Span,
}

/// A type annotation.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    /// Simple named type: `i64`, `String`, `bool`, `Vec<i64>`, etc.
    /// `generic_args` carries the inner types of `Vec<T>`, `HashMap<K, V>`,
    /// `Option<T>`, etc. Empty for unparameterized types.
    Named {
        name: String,
        generic_args: Vec<TypeAnnotation>,
        span: Span,
    },
    /// Fixed-size array type: `[T; N]`
    Array {
        inner: Box<TypeAnnotation>,
        size: usize,
        span: Span,
    },
}

impl TypeAnnotation {
    /// Extract the name for simple named types. Panics on compound types.
    pub fn name(&self) -> &str {
        match self {
            TypeAnnotation::Named { name, .. } => name,
            TypeAnnotation::Array { .. } => panic!("called name() on Array type annotation"),
        }
    }

    pub fn span(&self) -> Span {
        match self {
            TypeAnnotation::Named { span, .. } => *span,
            TypeAnnotation::Array { span, .. } => *span,
        }
    }
}

// === Pretty-printing for AST dump ===

impl Program {
    /// Pretty-print the AST for debugging.
    pub fn pretty_print(&self) -> String {
        let mut out = String::new();
        for item in &self.items {
            item.pretty_print(&mut out, 0);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::IntegerSuffix;

    #[test]
    fn test_binop_display() {
        assert_eq!(format!("{}", BinOp::Add), "+");
        assert_eq!(format!("{}", BinOp::Eq), "==");
        assert_eq!(format!("{}", BinOp::And), "&&");
    }

    #[test]
    fn test_unaryop_display() {
        assert_eq!(format!("{}", UnaryOp::Neg), "-");
        assert_eq!(format!("{}", UnaryOp::Not), "!");
        assert_eq!(format!("{}", UnaryOp::BitNot), "~");
    }

    #[test]
    fn test_expr_span() {
        let span = Span::new(0, 5, 1, 1);
        let expr = Expr::IntLiteral(42, IntegerSuffix::None, span);
        assert_eq!(expr.span(), span);
    }
}
