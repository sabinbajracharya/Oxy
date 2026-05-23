//! Compiler helper functions: closure capture analysis, const evaluation,
//! literal validation, narrowing casts, and builtin path detection.
//!
//! ```text
//! helpers.rs  ── free pub(crate) functions, no Compiler access needed
//!   used by: mod.rs (re-exported), expr.rs (type checking + casts)
//! ```

use std::collections::HashSet;

use crate::ast::*;
use crate::errors::FerriError;
use crate::lexer::{FloatSuffix, IntegerSuffix};
use crate::types::{FloatWidth, IntegerWidth};
use crate::vm::OpCode;

use super::Compiler;

/// Find all free variables in an expression (variables used but not defined in `params`).
/// Pre-scan: find names of free variables in all closures in the function body.
pub(crate) fn find_captured_mutable(
    block: &crate::ast::Block,
    params: &[String],
) -> HashSet<String> {
    let mut captured = HashSet::new();
    collect_closure_free_vars_in_block(block, params, &mut captured);
    captured
}

pub(crate) fn collect_closure_free_vars_in_block(
    block: &crate::ast::Block,
    params: &[String],
    out: &mut HashSet<String>,
) {
    for stmt in &block.stmts {
        match stmt {
            crate::ast::Stmt::Expr { expr, .. } => collect_closure_free_vars(expr, params, out),
            crate::ast::Stmt::Let { value, .. } => {
                if let Some(v) = value {
                    collect_closure_free_vars(v, params, out);
                }
            }
            crate::ast::Stmt::While {
                condition, body, ..
            } => {
                collect_closure_free_vars(condition, params, out);
                collect_closure_free_vars_in_block(body, params, out);
            }
            crate::ast::Stmt::Loop { body, .. } => {
                collect_closure_free_vars_in_block(body, params, out)
            }
            _ => {}
        }
    }
}

pub(crate) fn collect_closure_free_vars(
    expr: &crate::ast::Expr,
    params: &[String],
    out: &mut HashSet<String>,
) {
    match expr {
        crate::ast::Expr::Closure {
            params: inner_params,
            body,
            ..
        } => {
            let mut cp = params.to_vec();
            for p in inner_params {
                cp.push(p.name.clone());
            }
            for v in find_free_vars(body, &cp) {
                out.insert(v);
            }
        }
        crate::ast::Expr::BinaryOp { left, right, .. } => {
            collect_closure_free_vars(left, params, out);
            collect_closure_free_vars(right, params, out);
        }
        crate::ast::Expr::Call { callee, args, .. } => {
            collect_closure_free_vars(callee, params, out);
            for a in args {
                collect_closure_free_vars(a, params, out);
            }
        }
        crate::ast::Expr::MethodCall { object, args, .. } => {
            collect_closure_free_vars(object, params, out);
            for a in args {
                collect_closure_free_vars(a, params, out);
            }
        }
        crate::ast::Expr::UnaryOp { expr: inner, .. } => {
            collect_closure_free_vars(inner, params, out)
        }
        crate::ast::Expr::Assign { target, value, .. } => {
            collect_closure_free_vars(target, params, out);
            collect_closure_free_vars(value, params, out);
        }
        _ => {}
    }
}

pub(crate) fn find_free_vars(expr: &crate::ast::Expr, params: &[String]) -> Vec<String> {
    let mut vars = Vec::new();
    collect_free_vars(expr, params, &mut vars);
    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    vars.retain(|v| seen.insert(v.clone()));
    vars
}

pub(crate) fn collect_free_vars(
    expr: &crate::ast::Expr,
    params: &[String],
    vars: &mut Vec<String>,
) {
    match expr {
        crate::ast::Expr::Ident(name, _) => {
            if !params.contains(name) {
                vars.push(name.clone());
            }
        }
        crate::ast::Expr::BinaryOp { left, right, .. } => {
            collect_free_vars(left, params, vars);
            collect_free_vars(right, params, vars);
        }
        crate::ast::Expr::UnaryOp { expr: inner, .. } => {
            collect_free_vars(inner, params, vars);
        }
        crate::ast::Expr::Call { callee, args, .. } => {
            collect_free_vars(callee, params, vars);
            for arg in args {
                collect_free_vars(arg, params, vars);
            }
        }
        crate::ast::Expr::Block(block) => {
            for stmt in &block.stmts {
                collect_free_vars_in_stmt(stmt, params, vars);
            }
        }
        crate::ast::Expr::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            collect_free_vars(condition, params, vars);
            for stmt in &then_block.stmts {
                collect_free_vars_in_stmt(stmt, params, vars);
            }
            if let Some(else_expr) = else_block {
                collect_free_vars(else_expr, params, vars);
            }
        }
        crate::ast::Expr::Index { object, index, .. } => {
            collect_free_vars(object, params, vars);
            collect_free_vars(index, params, vars);
        }
        crate::ast::Expr::MethodCall { object, args, .. } => {
            collect_free_vars(object, params, vars);
            for arg in args {
                collect_free_vars(arg, params, vars);
            }
        }
        crate::ast::Expr::Assign { target, value, .. } => {
            collect_free_vars(target, params, vars);
            collect_free_vars(value, params, vars);
        }
        crate::ast::Expr::CompoundAssign { target, value, .. } => {
            collect_free_vars(target, params, vars);
            collect_free_vars(value, params, vars);
        }
        crate::ast::Expr::MacroCall { args, .. } => {
            for arg in args {
                collect_free_vars(arg, params, vars);
            }
        }
        crate::ast::Expr::Closure {
            params: inner_params,
            body,
            ..
        } => {
            let mut new_params = params.to_vec();
            for p in inner_params {
                new_params.push(p.name.clone());
            }
            collect_free_vars(body, &new_params, vars);
        }
        _ => {} // Skip other expression types for now
    }
}

pub(crate) fn collect_free_vars_in_stmt(
    stmt: &crate::ast::Stmt,
    params: &[String],
    vars: &mut Vec<String>,
) {
    match stmt {
        crate::ast::Stmt::Expr { expr, .. } => collect_free_vars(expr, params, vars),
        crate::ast::Stmt::Let { value, .. } => {
            if let Some(val) = value {
                collect_free_vars(val, params, vars);
            }
        }
        crate::ast::Stmt::While {
            condition, body, ..
        } => {
            collect_free_vars(condition, params, vars);
            for s in &body.stmts {
                collect_free_vars_in_stmt(s, params, vars);
            }
        }
        crate::ast::Stmt::Loop { body, .. } => {
            for s in &body.stmts {
                collect_free_vars_in_stmt(s, params, vars);
            }
        }
        _ => {}
    }
}

/// Evaluate a simple constant expression at compile time.
pub(crate) fn try_eval_const(expr: &crate::ast::Expr) -> Option<crate::types::Value> {
    match expr {
        crate::ast::Expr::IntLiteral(n, IntegerSuffix::None, _) => {
            Some(crate::types::Value::I64(*n))
        }
        crate::ast::Expr::FloatLiteral(n, FloatSuffix::None, _) => {
            Some(crate::types::Value::F64(*n))
        }
        crate::ast::Expr::BoolLiteral(b, _) => Some(crate::types::Value::Bool(*b)),
        crate::ast::Expr::StringLiteral(s, _) => Some(crate::types::Value::String(s.clone())),
        crate::ast::Expr::CharLiteral(c, _) => Some(crate::types::Value::Char(*c)),
        crate::ast::Expr::UnaryOp {
            op: crate::ast::UnaryOp::Neg,
            expr: inner,
            ..
        } => match try_eval_const(inner) {
            Some(crate::types::Value::I64(n)) => Some(crate::types::Value::I64(-n)),
            Some(crate::types::Value::F64(n)) => Some(crate::types::Value::F64(-n)),
            _ => None,
        },
        _ => None,
    }
}

/// Known built-in paths that the VM can dispatch natively.
/// Validate that an integer literal value fits in the target width (for suffixed literals).
/// Note: the lexer stores u64 values > i64::MAX as negative i64 via wrapping `as i64`,
/// so we reinterpret bits as u64 for unsigned width checks.
pub(crate) fn validate_int_literal(
    n: i64,
    width: &IntegerWidth,
    span: crate::lexer::Span,
) -> Result<(), FerriError> {
    let fits = match width {
        IntegerWidth::I8 => (i8::MIN as i64..=i8::MAX as i64).contains(&n),
        IntegerWidth::I16 => (i16::MIN as i64..=i16::MAX as i64).contains(&n),
        IntegerWidth::I32 => (i32::MIN as i64..=i32::MAX as i64).contains(&n),
        IntegerWidth::I64 => true,
        // For unsigned widths: reinterpret the bits as u64 to handle
        // values > i64::MAX that the lexer stored via wrapping as i64.
        IntegerWidth::U8 => (n as u64) <= u8::MAX as u64,
        IntegerWidth::U16 => (n as u64) <= u16::MAX as u64,
        IntegerWidth::U32 => (n as u64) <= u32::MAX as u64,
        IntegerWidth::U64 => true,
    };
    if !fits {
        return Err(FerriError::Runtime {
            message: format!("literal out of range for `{}`", width_to_str(width)),
            line: span.line,
            column: span.column,
        });
    }
    Ok(())
}

pub(crate) fn width_to_str(w: &IntegerWidth) -> &str {
    match w {
        IntegerWidth::I8 => "i8",
        IntegerWidth::I16 => "i16",
        IntegerWidth::I32 => "i32",
        IntegerWidth::I64 => "i64",
        IntegerWidth::U8 => "u8",
        IntegerWidth::U16 => "u16",
        IntegerWidth::U32 => "u32",
        IntegerWidth::U64 => "u64",
    }
}

/// Check that a constant integer literal fits in the target integer type's range.
/// Returns an error if the literal value is outside the type's bounds (matches Rust).
pub(crate) fn check_literal_fits_type(
    expr: &Expr,
    type_name: &str,
    span: crate::lexer::Span,
) -> Result<(), FerriError> {
    let (min, max): (i128, i128) = match type_name {
        "i8" => (i8::MIN as i128, i8::MAX as i128),
        "i16" => (i16::MIN as i128, i16::MAX as i128),
        "i32" => (i32::MIN as i128, i32::MAX as i128),
        "i64" | "isize" => (i64::MIN as i128, i64::MAX as i128),
        "u8" => (0, u8::MAX as i128),
        "u16" => (0, u16::MAX as i128),
        "u32" => (0, u32::MAX as i128),
        "u64" | "usize" => (0, u64::MAX as i128),
        _ => return Ok(()),
    };

    match expr {
        Expr::IntLiteral(n, suffix, _) => {
            if *suffix != IntegerSuffix::None {
                return Ok(()); // suffixed literal: validated separately
            }
            let val = *n as i128;
            if val < min || val > max {
                return Err(FerriError::Runtime {
                    message: format!(
                        "literal out of range for `{type_name}`: value {val} is outside the range {min}..={max}"
                    ),
                    line: span.line,
                    column: span.column,
                });
            }
        }
        Expr::UnaryOp {
            op: crate::ast::UnaryOp::Neg,
            expr: inner,
            ..
        } => {
            if let Expr::IntLiteral(n, suffix, _) = inner.as_ref() {
                if *suffix != IntegerSuffix::None {
                    return Ok(());
                }
                let val = -(*n as i128);
                if val < min {
                    return Err(FerriError::Runtime {
                        message: format!(
                            "literal out of range for `{type_name}`: value {val} is less than minimum {min}"
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
                if min == 0 && val < 0 {
                    return Err(FerriError::Runtime {
                        message: format!(
                            "literal out of range for `{type_name}`: value {val} cannot be negative"
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Emit a narrowing cast if the type annotation specifies an integer or float width.
pub(crate) fn emit_narrowing_cast(compiler: &mut Compiler, type_name: &str) {
    let op = match type_name {
        "i8" => Some(OpCode::CastInt(IntegerWidth::I8)),
        "i16" => Some(OpCode::CastInt(IntegerWidth::I16)),
        "i32" => Some(OpCode::CastInt(IntegerWidth::I32)),
        "i64" | "isize" => Some(OpCode::CastInt(IntegerWidth::I64)),
        "u8" => Some(OpCode::CastInt(IntegerWidth::U8)),
        "u16" => Some(OpCode::CastInt(IntegerWidth::U16)),
        "u32" => Some(OpCode::CastInt(IntegerWidth::U32)),
        "u64" | "usize" => Some(OpCode::CastInt(IntegerWidth::U64)),
        "f32" => Some(OpCode::CastFloat(FloatWidth::F32)),
        "f64" => Some(OpCode::CastFloat(FloatWidth::F64)),
        _ => None,
    };
    if let Some(o) = op {
        compiler.emit(o);
    }
}

pub(crate) fn is_builtin_path(path: &[String]) -> bool {
    let segs: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
    let module = segs.first().copied().unwrap_or("");
    // Handle std::module::function paths (3+ segments starting with "std")
    let effective_module = if segs.len() >= 3 && module == "std" {
        segs.get(1).copied().unwrap_or("")
    } else {
        module
    };
    matches!(
        segs.as_slice(),
        // math
        ["math", "sqrt"]
            | ["math", "abs"]
            | ["math", "sin"]
            | ["math", "cos"]
            | ["math", "tan"]
            | ["math", "asin"]
            | ["math", "acos"]
            | ["math", "atan"]
            | ["math", "pow"]
            | ["math", "floor"]
            | ["math", "ceil"]
            | ["math", "round"]
            | ["math", "min"]
            | ["math", "max"]
            | ["math", "log"]
            | ["math", "log2"]
            | ["math", "log10"]
            | ["math", "gcd"]
            | ["math", "lcm"]
            // json
            | ["json", "parse"]
            | ["json", "to_string"]
            | ["json", "serialize"]
            | ["json", "deserialize"]
            | ["json", "to_string_pretty"]
            | ["json", "from_str"]
            | ["json", "from_struct"]
            // constructors
            | ["String", "from"]
            | ["HashMap", "new"]
            | ["HashSet", "new"]
            | ["BTreeMap", "new"]
            | ["BTreeSet", "new"]
            | ["BinaryHeap", "new"]
            | ["VecDeque", "new"]
            | ["ListNode", "new"]
            | ["TreeNode", "new"]
            | ["char", "from_code"]
            | ["int", "parse"]

            | ["float", "parse"]
    ) || matches!(
        effective_module,
        "fs" | "env" | "process" | "regex" | "net" | "time" | "rand" | "http"
    ) || segs.as_slice() == ["std", "env", "args"]
}

/// Substitute generic type param names with concrete types in an expression tree.
/// Used by monomorphization to replace `T` with `i64` etc. in path calls and annotations.
/// Resolve `self`, `super`, `crate` across ALL segments of a use path.
pub(crate) fn resolve_use_path(path: &[String], module_stack: &[String]) -> Vec<String> {
    let mut context: Vec<String> = module_stack.to_vec();
    let mut i = 0;
    let mut had_special = false;
    while i < path.len() {
        match path[i].as_str() {
            "self" => {
                had_special = true;
                i += 1;
            }
            "super" => {
                had_special = true;
                context.pop();
                i += 1;
            }
            "crate" => {
                had_special = true;
                context.clear();
                i += 1;
            }
            _ => break,
        }
    }
    if had_special {
        let mut resolved = context;
        resolved.extend_from_slice(&path[i..]);
        resolved
    } else {
        path.to_vec()
    }
}
pub(crate) fn substitute_type_params(expr: &mut crate::ast::Expr, subst: &[(String, String)]) {
    match expr {
        crate::ast::Expr::PathCall { path, args, .. } => {
            if let Some(concrete) = subst.iter().find(|(p, _)| **p == path[0]).map(|(_, c)| c) {
                path[0] = concrete.clone();
            }
            for arg in args {
                substitute_type_params(arg, subst);
            }
        }
        crate::ast::Expr::Call { callee, args, .. } => {
            substitute_type_params(callee, subst);
            for arg in args {
                substitute_type_params(arg, subst);
            }
        }
        crate::ast::Expr::MethodCall { object, args, .. } => {
            substitute_type_params(object, subst);
            for arg in args {
                substitute_type_params(arg, subst);
            }
        }
        crate::ast::Expr::BinaryOp { left, right, .. } => {
            substitute_type_params(left, subst);
            substitute_type_params(right, subst);
        }
        crate::ast::Expr::UnaryOp { expr: inner, .. } => {
            substitute_type_params(inner, subst);
        }
        crate::ast::Expr::Block(block) => {
            for stmt in &mut block.stmts {
                match stmt {
                    crate::ast::Stmt::Expr { expr, .. } => substitute_type_params(expr, subst),
                    crate::ast::Stmt::Let { value, .. } => {
                        if let Some(val) = value {
                            substitute_type_params(val, subst);
                        }
                    }
                    crate::ast::Stmt::Return { value, .. } => {
                        if let Some(val) = value {
                            substitute_type_params(val, subst);
                        }
                    }
                    _ => {}
                }
            }
        }
        crate::ast::Expr::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            substitute_type_params(condition, subst);
            for stmt in &mut then_block.stmts {
                if let crate::ast::Stmt::Expr { expr, .. } = stmt {
                    substitute_type_params(expr, subst);
                }
            }
            if let Some(else_expr) = else_block {
                substitute_type_params(else_expr, subst);
            }
        }
        crate::ast::Expr::Assign { target, value, .. } => {
            substitute_type_params(target, subst);
            substitute_type_params(value, subst);
        }
        crate::ast::Expr::CompoundAssign { target, value, .. } => {
            substitute_type_params(target, subst);
            substitute_type_params(value, subst);
        }
        crate::ast::Expr::Match { expr, arms, .. } => {
            substitute_type_params(expr, subst);
            for arm in arms {
                substitute_type_params(&mut arm.body, subst);
            }
        }
        crate::ast::Expr::Array { elements, .. } => {
            for elem in elements {
                substitute_type_params(elem, subst);
            }
        }
        crate::ast::Expr::StructInit { fields, .. } => {
            for (_, field_expr) in fields {
                substitute_type_params(field_expr, subst);
            }
        }
        crate::ast::Expr::FieldAccess { object, .. } => {
            substitute_type_params(object, subst);
        }
        crate::ast::Expr::Index { object, index, .. } => {
            substitute_type_params(object, subst);
            substitute_type_params(index, subst);
        }
        crate::ast::Expr::MacroCall { args, .. } => {
            for arg in args {
                substitute_type_params(arg, subst);
            }
        }
        _ => {}
    }
}
