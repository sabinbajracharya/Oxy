//! Expression types: Expr, BinOp, UnaryOp, MatchArm, FStringPart.
//!
//! Extracted from [`super`] to keep mod.rs under ~300 lines.

use super::item::ClosureParam;
use super::pattern::Pattern;
use super::{Block, TypeAnnotation};
use crate::lexer::Span;

/// A part of an f-string: either a literal segment or an interpolated expression.
#[derive(Debug, Clone, PartialEq)]
pub enum FStringPart {
    /// A literal text segment.
    Literal(String),
    /// An interpolated expression.
    Expr(Box<Expr>),
}

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Integer literal: `42`
    IntLiteral(i64, crate::lexer::IntegerSuffix, Span),
    /// Float literal: `3.14`
    FloatLiteral(f64, crate::lexer::FloatSuffix, Span),
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
    /// Function call: `foo(a, b)` or `foo::<T>(a, b)`
    Call {
        callee: Box<Expr>,
        turbofish: Option<Vec<TypeAnnotation>>,
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
    /// Range expression: `start..end`, `start..=end`, `start..`, `..end`, `..`
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
        span: Span,
    },
    /// Array repeat: `[val; N]`
    Repeat {
        value: Box<Expr>,
        count: Box<Expr>,
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
    /// Method call: `expr.method(args)` or `expr.method::<T>(args)`
    MethodCall {
        object: Box<Expr>,
        method: String,
        turbofish: Option<Vec<TypeAnnotation>>,
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
    /// Struct instantiation: `Point { x: 1.0, y: 2.0 }` or with update `Point { x: 1, ..base }`
    StructInit {
        name: String,
        fields: Vec<(String, Expr)>,
        /// Optional `..base` expression — remaining fields copied from here.
        base: Option<Box<Expr>>,
        span: Span,
    },
    /// Path expression: `Type::method(args)` or `Type::method::<T>(args)`
    PathCall {
        path: Vec<String>,
        turbofish: Option<Vec<TypeAnnotation>>,
        args: Vec<Expr>,
        span: Span,
    },
    /// Path without call: `Type::Variant` — unit enum variant access
    Path { segments: Vec<String>, span: Span },
    /// `self` keyword in methods
    SelfRef(Span),
    /// `if val` / `if var` expression: `if let Some(x) = expr { ... } else { ... }`
    IfLet {
        pattern: Box<Pattern>,
        expr: Box<Expr>,
        /// Optional `&& condition` guard after the scrutinee.
        guard: Option<Box<Expr>>,
        then_block: Block,
        else_block: Option<Box<Expr>>,
        mutable: bool,
        span: Span,
    },
    /// Type cast: `expr as Type`
    As {
        expr: Box<Expr>,
        type_name: String,
        span: Span,
    },
    /// Try operator: `expr?` — unwraps Ok/Some or returns Err/None early
    Try { expr: Box<Expr>, span: Span },
    /// Closure expression: `|params| expr` or `|params| { body }`
    Closure {
        params: Vec<ClosureParam>,
        return_type: Option<TypeAnnotation>,
        body: Box<Expr>,
        span: Span,
        is_async: bool,
    },
    /// Async block: `async { ... }` — evaluates to a Future<T>.
    AsyncBlock { body: Block, span: Span },
    /// Await expression: `expr.await`
    Await { expr: Box<Expr>, span: Span },
    /// F-string expression: `f"Hello {name}!"`
    FString { parts: Vec<FStringPart>, span: Span },
    /// Return expression (diverges): `return expr` or `return`
    Return {
        value: Option<Box<Expr>>,
        span: Span,
    },
}

impl Expr {
    /// Returns the span of this expression.
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLiteral(.., s)
            | Expr::FloatLiteral(.., s)
            | Expr::BoolLiteral(_, s)
            | Expr::StringLiteral(_, s)
            | Expr::CharLiteral(_, s)
            | Expr::Ident(_, s)
            | Expr::BinaryOp { span: s, .. }
            | Expr::UnaryOp { span: s, .. }
            | Expr::Call { span: s, .. }
            | Expr::If { span: s, .. }
            | Expr::Assign { span: s, .. }
            | Expr::CompoundAssign { span: s, .. }
            | Expr::Grouped(_, s)
            | Expr::Match { span: s, .. }
            | Expr::Range { span: s, .. }
            | Expr::Repeat { span: s, .. }
            | Expr::Array { span: s, .. }
            | Expr::Index { span: s, .. }
            | Expr::MethodCall { span: s, .. }
            | Expr::FieldAccess { span: s, .. }
            | Expr::Tuple { span: s, .. }
            | Expr::StructInit { span: s, .. }
            | Expr::PathCall { span: s, .. }
            | Expr::Path { span: s, .. }
            | Expr::SelfRef(s)
            | Expr::As { span: s, .. }
            | Expr::IfLet { span: s, .. }
            | Expr::Try { span: s, .. }
            | Expr::Closure { span: s, .. }
            | Expr::AsyncBlock { span: s, .. }
            | Expr::Await { span: s, .. }
            | Expr::FString { span: s, .. }
            | Expr::Return { span: s, .. } => *s,
            Expr::Block(block) => block.span,
        }
    }
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    /// Addition: `+`
    Add,
    /// Subtraction: `-`
    Sub,
    /// Multiplication: `*`
    Mul,
    /// Division: `/`
    Div,
    /// Modulo: `%`
    Mod,
    /// Equality: `==`
    Eq,
    /// Inequality: `!=`
    NotEq,
    /// Less than: `<`
    Lt,
    /// Greater than: `>`
    Gt,
    /// Less than or equal: `<=`
    LtEq,
    /// Greater than or equal: `>=`
    GtEq,
    /// Logical and: `&&`
    And,
    /// Logical or: `||`
    Or,
    /// Bitwise and: `&`
    BitAnd,
    /// Bitwise or: `|`
    BitOr,
    /// Bitwise xor: `^`
    BitXor,
    /// Left shift: `<<`
    Shl,
    /// Right shift: `>>`
    Shr,
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
    /// Arithmetic negation: `-`
    Neg,
    /// Logical not: `!`
    Not,
    /// Bitwise not: `~`
    BitNot,
}

impl std::fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryOp::Neg => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
            UnaryOp::BitNot => write!(f, "~"),
        }
    }
}

/// A match arm: `pattern => expr` or `pattern if guard => expr`
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Box<Expr>>,
    pub body: Expr,
    pub span: Span,
}
impl Expr {
    pub(crate) fn pretty_print(&self, out: &mut String, indent: usize) {
        match self {
            Expr::IntLiteral(n, _, _) => out.push_str(&n.to_string()),
            Expr::FloatLiteral(n, _, _) => out.push_str(&n.to_string()),
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
                if let Some(s) = start {
                    s.pretty_print(out, 0);
                }
                if *inclusive {
                    out.push_str("..=");
                } else {
                    out.push_str("..");
                }
                if let Some(e) = end {
                    e.pretty_print(out, 0);
                }
            }
            Expr::Repeat { value, count, .. } => {
                out.push('[');
                value.pretty_print(out, 0);
                out.push_str("; ");
                count.pretty_print(out, 0);
                out.push(']');
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
            Expr::IfLet {
                pattern,
                expr,
                then_block,
                else_block,
                mutable,
                ..
            } => {
                let pad = "  ".repeat(indent);
                out.push_str(&format!("{pad}if {} ", if *mutable { "var" } else { "val" }));
                pattern.pretty_print(out);
                out.push_str(" = ");
                expr.pretty_print(out, 0);
                out.push_str(" {\n");
                for s in &then_block.stmts {
                    s.pretty_print(out, indent + 1);
                }
                out.push_str(&format!("{pad}}}"));
                if let Some(else_expr) = else_block {
                    out.push_str(" else ");
                    else_expr.pretty_print(out, indent);
                }
                out.push('\n');
            }
            Expr::Try { expr, .. } => {
                expr.pretty_print(out, indent);
                out.push('?');
            }
            Expr::Closure {
                params,
                return_type,
                body,
                is_async,
                ..
            } => {
                if *is_async {
                    out.push_str("async ");
                }
                out.push('|');
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&p.name);
                    if let Some(ty) = &p.type_ann {
                        out.push_str(&format!(": {ty:?}"));
                    }
                }
                out.push('|');
                if let Some(ret) = return_type {
                    out.push_str(&format!(" -> {ret:?}"));
                }
                out.push(' ');
                body.pretty_print(out, indent);
            }
            Expr::AsyncBlock { body, .. } => {
                out.push_str("async {\n");
                for stmt in &body.stmts {
                    stmt.pretty_print(out, indent + 1);
                }
                out.push_str(&"  ".repeat(indent));
                out.push('}');
            }
            Expr::Await { expr, .. } => {
                expr.pretty_print(out, indent);
                out.push_str(".await");
            }
            Expr::FString { parts, .. } => {
                out.push_str("f\"");
                for part in parts {
                    match part {
                        FStringPart::Literal(s) => out.push_str(s),
                        FStringPart::Expr(expr) => {
                            out.push('{');
                            expr.pretty_print(out, 0);
                            out.push('}');
                        }
                    }
                }
                out.push('"');
            }
            Expr::Return { value, .. } => {
                out.push_str("return");
                if let Some(expr) = value {
                    out.push(' ');
                    expr.pretty_print(out, indent);
                }
            }
            Expr::As {
                expr, type_name, ..
            } => {
                expr.pretty_print(out, indent);
                out.push_str(" as ");
                out.push_str(type_name);
            }
        }
    }
}
