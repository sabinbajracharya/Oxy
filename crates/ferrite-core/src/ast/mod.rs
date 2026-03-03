//! Abstract Syntax Tree definitions for the Ferrite language.

use crate::lexer::Span;

/// A complete Ferrite program — a sequence of top-level items.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
    pub span: Span,
}

/// A top-level item (function, struct, enum, etc.).
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Function(FnDef),
    Struct(StructDef),
    Enum(EnumDef),
    Impl(ImplBlock),
    Trait(TraitDef),
    ImplTrait(ImplTraitBlock),
}

impl Item {
    pub fn span(&self) -> Span {
        match self {
            Item::Function(f) => f.span,
            Item::Struct(s) => s.span,
            Item::Enum(e) => e.span,
            Item::Impl(i) => i.span,
            Item::Trait(t) => t.span,
            Item::ImplTrait(i) => i.span,
        }
    }
}

/// A function definition.
#[derive(Debug, Clone, PartialEq)]
pub struct FnDef {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub span: Span,
}

/// A function parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub span: Span,
}

/// A struct definition: `struct Name { field: Type, ... }` or `struct Name(Type, ...);`
#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub kind: StructKind,
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
    pub span: Span,
}

/// An enum definition: `enum Name { Variant, Variant(Type), Variant { field: Type } }`
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariant>,
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

/// An impl block: `impl Name { fn ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct ImplBlock {
    pub type_name: String,
    pub methods: Vec<FnDef>,
    pub span: Span,
}

/// A trait definition: `trait Name { fn method(&self) -> Type; }`
#[derive(Debug, Clone, PartialEq)]
pub struct TraitDef {
    pub name: String,
    pub methods: Vec<TraitMethodSig>,
    pub default_methods: Vec<FnDef>,
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

/// A generic type parameter, e.g., `T` or `T: Display + Clone`
#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
    pub name: String,
    pub bounds: Vec<String>,
    pub span: Span,
}

/// A type annotation (simple for now — just a name like `i64`, `String`, `bool`).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeAnnotation {
    pub name: String,
    pub span: Span,
}

/// A block: `{ stmts }` — the last expression (without semicolon) is the block's value.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

/// A statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `let [mut] name [: type] [= expr];`
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
    /// `while cond { body }`
    While {
        condition: Box<Expr>,
        body: Block,
        span: Span,
    },
    /// `loop { body }`
    Loop { body: Block, span: Span },
    /// `for name in iterable { body }`
    For {
        name: String,
        iterable: Box<Expr>,
        body: Block,
        span: Span,
    },
    /// `break [expr];`
    Break {
        value: Option<Box<Expr>>,
        span: Span,
    },
    /// `continue;`
    Continue { span: Span },
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. }
            | Stmt::Return { span, .. }
            | Stmt::While { span, .. }
            | Stmt::Loop { span, .. }
            | Stmt::For { span, .. }
            | Stmt::Break { span, .. }
            | Stmt::Continue { span, .. } => *span,
            Stmt::Expr { expr, .. } => expr.span(),
        }
    }
}

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Integer literal: `42`
    IntLiteral(i64, Span),
    /// Float literal: `3.14`
    FloatLiteral(f64, Span),
    /// Boolean literal: `true` / `false`
    BoolLiteral(bool, Span),
    /// String literal: `"hello"`
    StringLiteral(String, Span),
    /// Character literal: `'a'`
    CharLiteral(char, Span),
    /// Variable reference: `x`
    Ident(String, Span),
    /// Binary operation: `a + b`
    BinaryOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
        span: Span,
    },
    /// Unary operation: `-x`, `!x`
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    /// Function call: `foo(a, b)`
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    /// Macro-style call: `println!("hello {}", x)`
    MacroCall {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    /// Block expression: `{ stmts }`
    Block(Block),
    /// If expression: `if cond { ... } [else { ... }]`
    If {
        condition: Box<Expr>,
        then_block: Block,
        else_block: Option<Box<Expr>>,
        span: Span,
    },
    /// Assignment: `x = expr`
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },
    /// Compound assignment: `x += expr`
    CompoundAssign {
        target: Box<Expr>,
        op: BinOp,
        value: Box<Expr>,
        span: Span,
    },
    /// Grouped expression: `(expr)` or tuple: `(a, b, c)`
    Grouped(Box<Expr>, Span),
    /// Match expression: `match expr { arms }`
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    /// Range expression: `start..end` or `start..=end`
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
        span: Span,
    },
    /// Array literal: `[1, 2, 3]`
    Array { elements: Vec<Expr>, span: Span },
    /// Index expression: `expr[index]`
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    /// Method call: `expr.method(args)`
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
        span: Span,
    },
    /// Field/tuple index access: `expr.0`, `expr.field`
    FieldAccess {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    /// Tuple literal: `(a, b, c)`
    Tuple { elements: Vec<Expr>, span: Span },
    /// Struct instantiation: `Point { x: 1.0, y: 2.0 }`
    StructInit {
        name: String,
        fields: Vec<(String, Expr)>,
        span: Span,
    },
    /// Path expression: `Type::method(args)` — associated function or enum variant constructor
    PathCall {
        path: Vec<String>,
        args: Vec<Expr>,
        span: Span,
    },
    /// Path without call: `Type::Variant` — unit enum variant access
    Path { segments: Vec<String>, span: Span },
    /// `self` keyword in methods
    SelfRef(Span),
}

impl Expr {
    /// Returns the span of this expression.
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLiteral(_, s)
            | Expr::FloatLiteral(_, s)
            | Expr::BoolLiteral(_, s)
            | Expr::StringLiteral(_, s)
            | Expr::CharLiteral(_, s)
            | Expr::Ident(_, s)
            | Expr::BinaryOp { span: s, .. }
            | Expr::UnaryOp { span: s, .. }
            | Expr::Call { span: s, .. }
            | Expr::MacroCall { span: s, .. }
            | Expr::If { span: s, .. }
            | Expr::Assign { span: s, .. }
            | Expr::CompoundAssign { span: s, .. }
            | Expr::Grouped(_, s)
            | Expr::Match { span: s, .. }
            | Expr::Range { span: s, .. }
            | Expr::Array { span: s, .. }
            | Expr::Index { span: s, .. }
            | Expr::MethodCall { span: s, .. }
            | Expr::FieldAccess { span: s, .. }
            | Expr::Tuple { span: s, .. }
            | Expr::StructInit { span: s, .. }
            | Expr::PathCall { span: s, .. }
            | Expr::Path { span: s, .. }
            | Expr::SelfRef(s) => *s,
            Expr::Block(block) => block.span,
        }
    }
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,    // +
    Sub,    // -
    Mul,    // *
    Div,    // /
    Mod,    // %
    Eq,     // ==
    NotEq,  // !=
    Lt,     // <
    Gt,     // >
    LtEq,   // <=
    GtEq,   // >=
    And,    // &&
    Or,     // ||
    BitAnd, // &
    BitOr,  // |
    BitXor, // ^
    Shl,    // <<
    Shr,    // >>
}

impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Mod => write!(f, "%"),
            BinOp::Eq => write!(f, "=="),
            BinOp::NotEq => write!(f, "!="),
            BinOp::Lt => write!(f, "<"),
            BinOp::Gt => write!(f, ">"),
            BinOp::LtEq => write!(f, "<="),
            BinOp::GtEq => write!(f, ">="),
            BinOp::And => write!(f, "&&"),
            BinOp::Or => write!(f, "||"),
            BinOp::BitAnd => write!(f, "&"),
            BinOp::BitOr => write!(f, "|"),
            BinOp::BitXor => write!(f, "^"),
            BinOp::Shl => write!(f, "<<"),
            BinOp::Shr => write!(f, ">>"),
        }
    }
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,   // -
    Not,   // !
    Ref,   // & (parsed but semantically ignored)
    Deref, // * (parsed but semantically ignored)
}

/// A match arm: `pattern => expr`
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
    pub span: Span,
}

/// A pattern for match arms.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Literal value: `42`, `"hello"`, `true`
    Literal(Expr),
    /// Wildcard: `_`
    Wildcard(Span),
    /// Variable binding: `x`
    Ident(String, Span),
    /// Enum variant pattern: `Shape::Circle(r)` or `Option::Some(x)`
    EnumVariant {
        enum_name: String,
        variant: String,
        fields: Vec<Pattern>,
        span: Span,
    },
    /// Struct pattern: `Point { x, y }`
    Struct {
        name: String,
        fields: Vec<(String, Pattern)>,
        span: Span,
    },
}

impl Pattern {
    fn pretty_print(&self, out: &mut String) {
        match self {
            Pattern::Literal(e) => e.pretty_print(out, 0),
            Pattern::Wildcard(_) => out.push('_'),
            Pattern::Ident(name, _) => out.push_str(name),
            Pattern::EnumVariant {
                enum_name,
                variant,
                fields,
                ..
            } => {
                out.push_str(&format!("{enum_name}::{variant}"));
                if !fields.is_empty() {
                    out.push('(');
                    for (i, f) in fields.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        f.pretty_print(out);
                    }
                    out.push(')');
                }
            }
            Pattern::Struct { name, fields, .. } => {
                out.push_str(name);
                out.push_str(" { ");
                for (i, (fname, pat)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(fname);
                    out.push_str(": ");
                    pat.pretty_print(out);
                }
                out.push_str(" }");
            }
        }
    }
}

impl std::fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryOp::Neg => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
            UnaryOp::Ref => write!(f, "&"),
            UnaryOp::Deref => write!(f, "*"),
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

impl Item {
    fn pretty_print(&self, out: &mut String, indent: usize) {
        match self {
            Item::Function(f) => f.pretty_print(out, indent),
            Item::Struct(s) => {
                let pad = "  ".repeat(indent);
                out.push_str(&format!("{pad}struct {}", s.name));
                match &s.kind {
                    StructKind::Named(fields) => {
                        out.push_str(" {\n");
                        for f in fields {
                            out.push_str(&format!("{pad}  {}: {},\n", f.name, f.type_ann.name));
                        }
                        out.push_str(&format!("{pad}}}\n"));
                    }
                    StructKind::Tuple(types) => {
                        out.push('(');
                        for (i, t) in types.iter().enumerate() {
                            if i > 0 {
                                out.push_str(", ");
                            }
                            out.push_str(&t.name);
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
                                out.push_str(&t.name);
                            }
                            out.push(')');
                        }
                        EnumVariantKind::Struct(fields) => {
                            out.push_str(" { ");
                            for (i, f) in fields.iter().enumerate() {
                                if i > 0 {
                                    out.push_str(", ");
                                }
                                out.push_str(&format!("{}: {}", f.name, f.type_ann.name));
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
                        out.push_str(&format!("{}: {}", p.name, p.type_ann.name));
                    }
                    out.push(')');
                    if let Some(ret) = &sig.return_type {
                        out.push_str(&format!(" -> {}", ret.name));
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
        }
    }
}

impl FnDef {
    fn pretty_print(&self, out: &mut String, indent: usize) {
        let pad = "  ".repeat(indent);
        out.push_str(&format!("{pad}fn {}", self.name));
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
            out.push_str(&format!("{}: {}", p.name, p.type_ann.name));
        }
        out.push(')');
        if let Some(ret) = &self.return_type {
            out.push_str(&format!(" -> {}", ret.name));
        }
        out.push_str(" {\n");
        for stmt in &self.body.stmts {
            stmt.pretty_print(out, indent + 1);
        }
        out.push_str(&format!("{pad}}}\n"));
    }
}

impl Stmt {
    fn pretty_print(&self, out: &mut String, indent: usize) {
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
                    "{pad}let {}{name}",
                    if *mutable { "mut " } else { "" }
                ));
                if let Some(t) = type_ann {
                    out.push_str(&format!(": {}", t.name));
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
                condition, body, ..
            } => {
                out.push_str(&format!("{pad}while "));
                condition.pretty_print(out, 0);
                out.push_str(" {\n");
                for stmt in &body.stmts {
                    stmt.pretty_print(out, indent + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            Stmt::Loop { body, .. } => {
                out.push_str(&format!("{pad}loop {{\n"));
                for stmt in &body.stmts {
                    stmt.pretty_print(out, indent + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            Stmt::For {
                name,
                iterable,
                body,
                ..
            } => {
                out.push_str(&format!("{pad}for {name} in "));
                iterable.pretty_print(out, 0);
                out.push_str(" {\n");
                for stmt in &body.stmts {
                    stmt.pretty_print(out, indent + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            Stmt::Break { value, .. } => {
                out.push_str(&format!("{pad}break"));
                if let Some(v) = value {
                    out.push(' ');
                    v.pretty_print(out, 0);
                }
                out.push_str(";\n");
            }
            Stmt::Continue { .. } => {
                out.push_str(&format!("{pad}continue;\n"));
            }
        }
    }
}

impl Expr {
    fn pretty_print(&self, out: &mut String, _indent: usize) {
        match self {
            Expr::IntLiteral(n, _) => out.push_str(&n.to_string()),
            Expr::FloatLiteral(n, _) => out.push_str(&n.to_string()),
            Expr::BoolLiteral(b, _) => out.push_str(&b.to_string()),
            Expr::StringLiteral(s, _) => out.push_str(&format!("\"{s}\"")),
            Expr::CharLiteral(c, _) => out.push_str(&format!("'{c}'")),
            Expr::Ident(name, _) => out.push_str(name),
            Expr::BinaryOp {
                left, op, right, ..
            } => {
                out.push('(');
                left.pretty_print(out, 0);
                out.push_str(&format!(" {op} "));
                right.pretty_print(out, 0);
                out.push(')');
            }
            Expr::UnaryOp { op, expr, .. } => {
                out.push_str(&format!("({op}"));
                expr.pretty_print(out, 0);
                out.push(')');
            }
            Expr::Call { callee, args, .. } => {
                callee.pretty_print(out, 0);
                out.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    arg.pretty_print(out, 0);
                }
                out.push(')');
            }
            Expr::MacroCall { name, args, .. } => {
                out.push_str(&format!("{name}!("));
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    arg.pretty_print(out, 0);
                }
                out.push(')');
            }
            Expr::Block(block) => {
                out.push_str("{\n");
                for stmt in &block.stmts {
                    stmt.pretty_print(out, 1);
                }
                out.push('}');
            }
            Expr::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                out.push_str("if ");
                condition.pretty_print(out, 0);
                out.push_str(" {\n");
                for stmt in &then_block.stmts {
                    stmt.pretty_print(out, 1);
                }
                out.push('}');
                if let Some(else_expr) = else_block {
                    out.push_str(" else ");
                    else_expr.pretty_print(out, 0);
                }
            }
            Expr::Assign { target, value, .. } => {
                target.pretty_print(out, 0);
                out.push_str(" = ");
                value.pretty_print(out, 0);
            }
            Expr::CompoundAssign {
                target, op, value, ..
            } => {
                target.pretty_print(out, 0);
                out.push_str(&format!(" {op}= "));
                value.pretty_print(out, 0);
            }
            Expr::Grouped(expr, _) => {
                out.push('(');
                expr.pretty_print(out, 0);
                out.push(')');
            }
            Expr::Match { expr, arms, .. } => {
                out.push_str("match ");
                expr.pretty_print(out, 0);
                out.push_str(" {\n");
                for arm in arms {
                    out.push_str("  ");
                    arm.pattern.pretty_print(out);
                    out.push_str(" => ");
                    arm.body.pretty_print(out, 0);
                    out.push_str(",\n");
                }
                out.push('}');
            }
            Expr::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                start.pretty_print(out, 0);
                if *inclusive {
                    out.push_str("..=");
                } else {
                    out.push_str("..");
                }
                end.pretty_print(out, 0);
            }
            Expr::Array { elements, .. } => {
                out.push('[');
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    elem.pretty_print(out, 0);
                }
                out.push(']');
            }
            Expr::Index { object, index, .. } => {
                object.pretty_print(out, 0);
                out.push('[');
                index.pretty_print(out, 0);
                out.push(']');
            }
            Expr::MethodCall {
                object,
                method,
                args,
                ..
            } => {
                object.pretty_print(out, 0);
                out.push('.');
                out.push_str(method);
                out.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    arg.pretty_print(out, 0);
                }
                out.push(')');
            }
            Expr::FieldAccess { object, field, .. } => {
                object.pretty_print(out, 0);
                out.push('.');
                out.push_str(field);
            }
            Expr::Tuple { elements, .. } => {
                out.push('(');
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    elem.pretty_print(out, 0);
                }
                if elements.len() == 1 {
                    out.push(',');
                }
                out.push(')');
            }
            Expr::StructInit { name, fields, .. } => {
                out.push_str(name);
                out.push_str(" { ");
                for (i, (fname, fval)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(fname);
                    out.push_str(": ");
                    fval.pretty_print(out, 0);
                }
                out.push_str(" }");
            }
            Expr::PathCall { path, args, .. } => {
                out.push_str(&path.join("::"));
                out.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    arg.pretty_print(out, 0);
                }
                out.push(')');
            }
            Expr::Path { segments, .. } => {
                out.push_str(&segments.join("::"));
            }
            Expr::SelfRef(_) => {
                out.push_str("self");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(format!("{}", UnaryOp::Ref), "&");
    }

    #[test]
    fn test_expr_span() {
        let span = Span::new(0, 5, 1, 1);
        let expr = Expr::IntLiteral(42, span);
        assert_eq!(expr.span(), span);
    }
}
