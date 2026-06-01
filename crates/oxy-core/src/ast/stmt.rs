//! Statement types: Block, Stmt, and their impl blocks.
//!
//! Extracted from [`super`] to keep mod.rs under ~300 lines.

use super::expr::Expr;
use super::item::{Item, UseDef, UseTree};
use super::pattern::Pattern;
use super::TypeAnnotation;
use crate::lexer::Span;

/// A block: `{ stmts }` — the last expression (without semicolon) is the block's value.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

/// A statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `val name [: type] [= expr];` or `var name [: type] [= expr];`
    Let {
        name: String,
        mutable: bool,
        type_ann: Option<TypeAnnotation>,
        value: Option<Expr>,
        span: Span,
    },
    /// An expression used as a statement. `has_semicolon` distinguishes
    /// `expr;` (statement, value discarded) from `expr` (tail expression, value returned).
    Expr { expr: Expr, has_semicolon: bool },
    /// `return [expr];`
    Return { value: Option<Expr>, span: Span },
    /// `[label:] while cond { body }`
    While {
        label: Option<String>,
        condition: Box<Expr>,
        body: Block,
        span: Span,
    },
    /// `[label:] loop { body }`
    Loop {
        label: Option<String>,
        body: Block,
        span: Span,
    },
    /// `[label:] for name in iterable { body }`
    For {
        label: Option<String>,
        name: String,
        iterable: Box<Expr>,
        body: Block,
        span: Span,
    },
    /// `break [label] [expr];`
    Break {
        label: Option<String>,
        value: Option<Box<Expr>>,
        span: Span,
    },
    /// `continue [label];`
    Continue { label: Option<String>, span: Span },
    /// `[label:] while let pattern = expr { body }` or `while var pattern = expr { body }`
    WhileLet {
        label: Option<String>,
        pattern: Box<Pattern>,
        expr: Box<Expr>,
        body: Block,
        mutable: bool,
        span: Span,
    },
    /// `[label:] for (a, b) in iterable { body }` — tuple destructuring
    ForDestructure {
        label: Option<String>,
        names: Vec<String>,
        iterable: Box<Expr>,
        body: Block,
        span: Span,
    },
    /// `val pattern = expr;` or `var pattern = expr;` — destructuring binding
    LetPattern {
        pattern: Box<Pattern>,
        mutable: bool,
        value: Expr,
        span: Span,
    },
    /// `use path::to::item;` — local import within a function body
    Use(UseDef),
    /// A nested item (fn, struct, enum, etc.) declared inside a function body.
    /// At compile time the item is hoisted to a synthetic qualified name based
    /// on the enclosing fn, and a local alias is added so calls inside the
    /// body resolve. Forward references within the body work because the
    /// prescan walks Stmt::Item before compiling expressions.
    Item(Box<Item>),
}

impl Stmt {
    /// Returns the source span of this statement.
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. }
            | Stmt::Return { span, .. }
            | Stmt::While { span, .. }
            | Stmt::Loop { span, .. }
            | Stmt::For { span, .. }
            | Stmt::Break { span, .. }
            | Stmt::Continue { span, .. }
            | Stmt::WhileLet { span, .. }
            | Stmt::ForDestructure { span, .. }
            | Stmt::LetPattern { span, .. } => *span,
            Stmt::Use(use_def) => use_def.span,
            Stmt::Item(item) => item.span(),
            Stmt::Expr { expr, .. } => expr.span(),
        }
    }
}
impl Stmt {
    pub(crate) fn pretty_print(&self, out: &mut String, indent: usize) {
        let pad = "  ".repeat(indent);
        match self {
            Stmt::Let {
                name,
                mutable,
                type_ann,
                value,
                ..
            } => {
                out.push_str(&format!(
                    "{pad}{} {name}",
                    if *mutable { "var" } else { "val" }
                ));
                if let Some(t) = type_ann {
                    out.push_str(&format!(": {}", t.name()));
                }
                if let Some(v) = value {
                    out.push_str(" = ");
                    v.pretty_print(out, 0);
                }
                out.push_str(";\n");
            }
            Stmt::Expr {
                expr,
                has_semicolon,
            } => {
                out.push_str(&pad);
                expr.pretty_print(out, 0);
                if *has_semicolon {
                    out.push(';');
                }
                out.push('\n');
            }
            Stmt::Return { value, .. } => {
                out.push_str(&format!("{pad}return"));
                if let Some(v) = value {
                    out.push(' ');
                    v.pretty_print(out, 0);
                }
                out.push_str(";\n");
            }
            Stmt::While {
                label,
                condition,
                body,
                ..
            } => {
                if let Some(l) = label {
                    out.push_str(&format!("{pad}'{l}: "));
                }
                out.push_str(&format!("{pad}while "));
                condition.pretty_print(out, 0);
                out.push_str(" {\n");
                for stmt in &body.stmts {
                    stmt.pretty_print(out, indent + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            Stmt::Loop { label, body, .. } => {
                if let Some(l) = label {
                    out.push_str(&format!("{pad}'{l}: "));
                }
                out.push_str(&format!("{pad}loop {{\n"));
                for stmt in &body.stmts {
                    stmt.pretty_print(out, indent + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            Stmt::For {
                label,
                name,
                iterable,
                body,
                ..
            } => {
                if let Some(l) = label {
                    out.push_str(&format!("{pad}'{l}: "));
                }
                out.push_str(&format!("{pad}for {name} in "));
                iterable.pretty_print(out, 0);
                out.push_str(" {\n");
                for stmt in &body.stmts {
                    stmt.pretty_print(out, indent + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            Stmt::Break { label, value, .. } => {
                out.push_str(&format!("{pad}break"));
                if let Some(l) = label {
                    out.push_str(&format!(" '{l}"));
                }
                if let Some(v) = value {
                    out.push(' ');
                    v.pretty_print(out, 0);
                }
                out.push_str(";\n");
            }
            Stmt::Continue { label, .. } => {
                out.push_str(&format!("{pad}continue"));
                if let Some(l) = label {
                    out.push_str(&format!(" '{l}"));
                }
                out.push_str(";\n");
            }
            Stmt::WhileLet {
                label,
                pattern,
                expr,
                body,
                mutable,
                ..
            } => {
                if let Some(l) = label {
                    out.push_str(&format!("{pad}'{l}: "));
                }
                out.push_str(&format!(
                    "{pad}while {} ",
                    if *mutable { "var" } else { "val" }
                ));
                pattern.pretty_print(out);
                out.push_str(" = ");
                expr.pretty_print(out, 0);
                out.push_str(" {\n");
                for s in &body.stmts {
                    s.pretty_print(out, indent + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            Stmt::ForDestructure {
                label,
                names,
                iterable,
                body,
                ..
            } => {
                if let Some(l) = label {
                    out.push_str(&format!("{pad}'{l}: "));
                }
                out.push_str(&format!("{pad}for ({}) in ", names.join(", ")));
                iterable.pretty_print(out, 0);
                out.push_str(" {\n");
                for s in &body.stmts {
                    s.pretty_print(out, indent + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            Stmt::LetPattern {
                pattern,
                mutable,
                value,
                ..
            } => {
                out.push_str(&format!("{pad}{} ", if *mutable { "var" } else { "val" }));
                pattern.pretty_print(out);
                out.push_str(" = ");
                value.pretty_print(out, 0);
                out.push_str(";\n");
            }
            Stmt::Use(use_def) => {
                let path = use_def.path.join("::");
                match &use_def.tree {
                    UseTree::Simple(alias) => {
                        if let Some(alias) = alias {
                            out.push_str(&format!("{pad}use {} as {};\n", path, alias));
                        } else {
                            out.push_str(&format!("{pad}use {};\n", path));
                        }
                    }
                    UseTree::Glob => {
                        out.push_str(&format!("{pad}use {}::*;\n", path));
                    }
                    UseTree::Group(items) => {
                        let names: Vec<String> = items
                            .iter()
                            .map(|(n, a)| {
                                if let Some(alias) = a {
                                    format!("{} as {}", n, alias)
                                } else {
                                    n.clone()
                                }
                            })
                            .collect();
                        out.push_str(&format!("{pad}use {}::{{{}}};\n", path, names.join(", ")));
                    }
                }
            }
            Stmt::Item(item) => {
                item.pretty_print(out, indent);
            }
        }
    }
}
