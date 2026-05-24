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

/// Pre-scan: collect the names every closure in `block` captures from the
/// enclosing scope. Exhaustive over `Stmt`.
pub(crate) fn collect_closure_free_vars_in_block(
    block: &crate::ast::Block,
    params: &[String],
    out: &mut HashSet<String>,
) {
    for stmt in &block.stmts {
        collect_closure_free_vars_in_stmt(stmt, params, out);
    }
}

fn collect_closure_free_vars_in_stmt(
    stmt: &crate::ast::Stmt,
    params: &[String],
    out: &mut HashSet<String>,
) {
    use crate::ast::Stmt;
    match stmt {
        Stmt::Expr { expr, .. } => collect_closure_free_vars(expr, params, out),
        Stmt::Let { value, .. } => {
            if let Some(v) = value {
                collect_closure_free_vars(v, params, out);
            }
        }
        Stmt::Return { value, .. } => {
            if let Some(v) = value {
                collect_closure_free_vars(v, params, out);
            }
        }
        Stmt::While {
            condition, body, ..
        } => {
            collect_closure_free_vars(condition, params, out);
            collect_closure_free_vars_in_block(body, params, out);
        }
        Stmt::Loop { body, .. } => collect_closure_free_vars_in_block(body, params, out),
        Stmt::For { iterable, body, .. } | Stmt::ForDestructure { iterable, body, .. } => {
            collect_closure_free_vars(iterable, params, out);
            collect_closure_free_vars_in_block(body, params, out);
        }
        Stmt::WhileLet { expr, body, .. } => {
            collect_closure_free_vars(expr, params, out);
            collect_closure_free_vars_in_block(body, params, out);
        }
        Stmt::LetPattern { value, .. } => collect_closure_free_vars(value, params, out),
        Stmt::Break { value: Some(v), .. } => collect_closure_free_vars(v, params, out),
        Stmt::Break { value: None, .. } | Stmt::Continue { .. } | Stmt::Use(_) | Stmt::Item(_) => {}
    }
}

/// Exhaustive walk over `Expr`. The previous implementation covered only six
/// of ~30 variants behind a `_ => {}` wildcard, so closures inside `if`,
/// `match`, `for`, struct literals, etc. silently escaped the mutable-capture
/// pre-scan. Forcing exhaustiveness here makes future AST extensions trigger a
/// compile error until the new variant is given a recursion arm.
pub(crate) fn collect_closure_free_vars(
    expr: &crate::ast::Expr,
    params: &[String],
    out: &mut HashSet<String>,
) {
    use crate::ast::Expr;
    use crate::ast::FStringPart;
    match expr {
        Expr::Closure {
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
        Expr::BinaryOp { left, right, .. } => {
            collect_closure_free_vars(left, params, out);
            collect_closure_free_vars(right, params, out);
        }
        Expr::UnaryOp { expr: inner, .. } => collect_closure_free_vars(inner, params, out),
        Expr::Call { callee, args, .. } => {
            collect_closure_free_vars(callee, params, out);
            for a in args {
                collect_closure_free_vars(a, params, out);
            }
        }
        Expr::MethodCall { object, args, .. } => {
            collect_closure_free_vars(object, params, out);
            for a in args {
                collect_closure_free_vars(a, params, out);
            }
        }
        Expr::MacroCall { args, .. } => {
            for a in args {
                collect_closure_free_vars(a, params, out);
            }
        }
        Expr::PathCall { args, .. } => {
            for a in args {
                collect_closure_free_vars(a, params, out);
            }
        }
        Expr::Assign { target, value, .. } | Expr::CompoundAssign { target, value, .. } => {
            collect_closure_free_vars(target, params, out);
            collect_closure_free_vars(value, params, out);
        }
        Expr::Block(block) => collect_closure_free_vars_in_block(block, params, out),
        Expr::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            collect_closure_free_vars(condition, params, out);
            collect_closure_free_vars_in_block(then_block, params, out);
            if let Some(eb) = else_block {
                collect_closure_free_vars(eb, params, out);
            }
        }
        Expr::IfLet {
            expr: scrutinee,
            then_block,
            else_block,
            ..
        } => {
            collect_closure_free_vars(scrutinee, params, out);
            collect_closure_free_vars_in_block(then_block, params, out);
            if let Some(eb) = else_block {
                collect_closure_free_vars(eb, params, out);
            }
        }
        Expr::Match {
            expr: scrutinee,
            arms,
            ..
        } => {
            collect_closure_free_vars(scrutinee, params, out);
            for arm in arms {
                if let Some(g) = &arm.guard {
                    collect_closure_free_vars(g, params, out);
                }
                collect_closure_free_vars(&arm.body, params, out);
            }
        }
        Expr::Index { object, index, .. } => {
            collect_closure_free_vars(object, params, out);
            collect_closure_free_vars(index, params, out);
        }
        Expr::FieldAccess { object, .. } => collect_closure_free_vars(object, params, out),
        Expr::Tuple { elements, .. } | Expr::Array { elements, .. } => {
            for e in elements {
                collect_closure_free_vars(e, params, out);
            }
        }
        Expr::StructInit { fields, base, .. } => {
            for (_, v) in fields {
                collect_closure_free_vars(v, params, out);
            }
            if let Some(b) = base {
                collect_closure_free_vars(b, params, out);
            }
        }
        Expr::Grouped(inner, _) => collect_closure_free_vars(inner, params, out),
        Expr::Range { start, end, .. } => {
            if let Some(s) = start.as_deref() {
                collect_closure_free_vars(s, params, out);
            }
            if let Some(e) = end.as_deref() {
                collect_closure_free_vars(e, params, out);
            }
        }
        Expr::Repeat { value, count, .. } => {
            collect_closure_free_vars(value, params, out);
            collect_closure_free_vars(count, params, out);
        }
        Expr::As { expr: inner, .. }
        | Expr::Try { expr: inner, .. }
        | Expr::Await { expr: inner, .. } => collect_closure_free_vars(inner, params, out),
        Expr::FString { parts, .. } => {
            for part in parts {
                if let FStringPart::Expr(e) = part {
                    collect_closure_free_vars(e, params, out);
                }
            }
        }
        Expr::Return { value, .. } => {
            if let Some(v) = value {
                collect_closure_free_vars(v, params, out);
            }
        }
        // Terminals.
        Expr::Ident(..)
        | Expr::IntLiteral(..)
        | Expr::FloatLiteral(..)
        | Expr::BoolLiteral(..)
        | Expr::StringLiteral(..)
        | Expr::CharLiteral(..)
        | Expr::Path { .. }
        | Expr::SelfRef(_) => {}
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
        crate::ast::Expr::FieldAccess { object, .. } => {
            collect_free_vars(object, params, vars);
        }
        crate::ast::Expr::Tuple { elements, .. } | crate::ast::Expr::Array { elements, .. } => {
            for e in elements {
                collect_free_vars(e, params, vars);
            }
        }
        crate::ast::Expr::StructInit { fields, base, .. } => {
            for (_, v) in fields {
                collect_free_vars(v, params, vars);
            }
            if let Some(b) = base {
                collect_free_vars(b, params, vars);
            }
        }
        crate::ast::Expr::Grouped(inner, _) => {
            collect_free_vars(inner, params, vars);
        }
        crate::ast::Expr::Range { start, end, .. } => {
            if let Some(s) = start.as_deref() {
                collect_free_vars(s, params, vars);
            }
            if let Some(e) = end.as_deref() {
                collect_free_vars(e, params, vars);
            }
        }
        crate::ast::Expr::Repeat { value, count, .. } => {
            collect_free_vars(value, params, vars);
            collect_free_vars(count, params, vars);
        }
        crate::ast::Expr::Match { expr, arms, .. } => {
            collect_free_vars(expr, params, vars);
            for arm in arms {
                // Pattern bindings become new locals — extend params for this arm.
                let mut arm_params = params.to_vec();
                pattern_bindings(&arm.pattern, &mut arm_params);
                if let Some(g) = &arm.guard {
                    collect_free_vars(g, &arm_params, vars);
                }
                collect_free_vars(&arm.body, &arm_params, vars);
            }
        }
        crate::ast::Expr::IfLet {
            pattern,
            expr: scrutinee,
            then_block,
            else_block,
            ..
        } => {
            collect_free_vars(scrutinee, params, vars);
            let mut inner_params = params.to_vec();
            pattern_bindings(pattern, &mut inner_params);
            for s in &then_block.stmts {
                collect_free_vars_in_stmt(s, &inner_params, vars);
            }
            if let Some(eb) = else_block {
                collect_free_vars(eb, params, vars);
            }
        }
        crate::ast::Expr::PathCall { args, .. } => {
            for a in args {
                collect_free_vars(a, params, vars);
            }
        }
        crate::ast::Expr::As { expr: inner, .. }
        | crate::ast::Expr::Try { expr: inner, .. }
        | crate::ast::Expr::Await { expr: inner, .. } => {
            collect_free_vars(inner, params, vars);
        }
        crate::ast::Expr::FString { parts, .. } => {
            for part in parts {
                if let crate::ast::FStringPart::Expr(e) = part {
                    collect_free_vars(e, params, vars);
                }
            }
        }
        crate::ast::Expr::Return { value, .. } => {
            if let Some(v) = value {
                collect_free_vars(v, params, vars);
            }
        }
        // Path, SelfRef, literals — no free variables.
        crate::ast::Expr::Path { .. }
        | crate::ast::Expr::SelfRef(_)
        | crate::ast::Expr::IntLiteral(..)
        | crate::ast::Expr::FloatLiteral(..)
        | crate::ast::Expr::BoolLiteral(..)
        | crate::ast::Expr::StringLiteral(..)
        | crate::ast::Expr::CharLiteral(..) => {}
    }
}

/// Collect names that a pattern introduces as bindings. Used by closure
/// capture analysis so match-arm / if-let bound names don't get treated as
/// free variables when they're actually local to the arm. Exhaustive over
/// `Pattern`.
fn pattern_bindings(pattern: &crate::ast::Pattern, out: &mut Vec<String>) {
    use crate::ast::Pattern;
    match pattern {
        Pattern::Ident(name, _) => out.push(name.clone()),
        Pattern::EnumVariant { fields, .. }
        | Pattern::Tuple(fields, _)
        | Pattern::Slice(fields, _) => {
            for f in fields {
                pattern_bindings(f, out);
            }
        }
        Pattern::Struct { fields, .. } => {
            for (_, p) in fields {
                pattern_bindings(p, out);
            }
        }
        Pattern::Or(pats, _) => {
            // Rust requires all Or-alternatives to bind the same set of names;
            // recording the first alternative's bindings is sufficient.
            if let Some(first) = pats.first() {
                pattern_bindings(first, out);
            }
        }
        // No-binding patterns.
        Pattern::Wildcard(_) | Pattern::Literal(_) | Pattern::Range { .. } | Pattern::Rest(_) => {}
    }
}

/// Exhaustive over `Stmt` — collect identifiers used inside `stmt` that
/// aren't in `params`. Previously a `_ => {}` wildcard silently dropped
/// `For`, `Return`, `WhileLet`, `ForDestructure`, `LetPattern`, and
/// `Break(value)`, so vars referenced only in those statement forms
/// escaped capture analysis.
pub(crate) fn collect_free_vars_in_stmt(
    stmt: &crate::ast::Stmt,
    params: &[String],
    vars: &mut Vec<String>,
) {
    use crate::ast::Stmt;
    match stmt {
        Stmt::Expr { expr, .. } => collect_free_vars(expr, params, vars),
        Stmt::Let { value, .. } => {
            if let Some(val) = value {
                collect_free_vars(val, params, vars);
            }
        }
        Stmt::Return { value, .. } => {
            if let Some(val) = value {
                collect_free_vars(val, params, vars);
            }
        }
        Stmt::While {
            condition, body, ..
        } => {
            collect_free_vars(condition, params, vars);
            for s in &body.stmts {
                collect_free_vars_in_stmt(s, params, vars);
            }
        }
        Stmt::Loop { body, .. } => {
            for s in &body.stmts {
                collect_free_vars_in_stmt(s, params, vars);
            }
        }
        Stmt::For {
            name,
            iterable,
            body,
            ..
        } => {
            collect_free_vars(iterable, params, vars);
            let mut inner = params.to_vec();
            inner.push(name.clone());
            for s in &body.stmts {
                collect_free_vars_in_stmt(s, &inner, vars);
            }
        }
        Stmt::ForDestructure {
            names,
            iterable,
            body,
            ..
        } => {
            collect_free_vars(iterable, params, vars);
            let mut inner = params.to_vec();
            inner.extend(names.iter().cloned());
            for s in &body.stmts {
                collect_free_vars_in_stmt(s, &inner, vars);
            }
        }
        Stmt::WhileLet {
            pattern,
            expr,
            body,
            ..
        } => {
            collect_free_vars(expr, params, vars);
            let mut inner = params.to_vec();
            pattern_bindings(pattern, &mut inner);
            for s in &body.stmts {
                collect_free_vars_in_stmt(s, &inner, vars);
            }
        }
        Stmt::LetPattern { value, .. } => collect_free_vars(value, params, vars),
        Stmt::Break { value: Some(v), .. } => collect_free_vars(v, params, vars),
        Stmt::Break { value: None, .. } | Stmt::Continue { .. } | Stmt::Use(_) | Stmt::Item(_) => {}
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

/// Check that a constant integer literal fits in the target integer type's range.
/// Returns an error if the literal value is outside the type's bounds (matches Rust).
pub(crate) fn check_literal_fits_type(
    expr: &Expr,
    type_name: &str,
    span: crate::lexer::Span,
) -> Result<(), FerriError> {
    let (min, max): (i128, i128) = match type_name {
        "int" => (i64::MIN as i128, i64::MAX as i128),
        "byte" => (0, u8::MAX as i128),
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

/// Emit a narrowing cast if the type annotation names one of Oxy's two
/// integer types or its single float type.
pub(crate) fn emit_narrowing_cast(compiler: &mut Compiler, type_name: &str) {
    let op = match type_name {
        "int" => Some(OpCode::CastInt(IntegerWidth::I64)),
        "byte" => Some(OpCode::CastInt(IntegerWidth::U8)),
        "float" => Some(OpCode::CastFloat(FloatWidth::F64)),
        _ => None,
    };
    if let Some(o) = op {
        compiler.emit(o);
    }
}

/// Whitelist for paths that the compiler should compile down to
/// `OpCode::PathCallBuiltin` (handled at runtime by the VM's
/// `dispatch_pathcall`). The actual list is the registry in
/// `crate::stdlib::registry` — this function just adapts the path type.
pub(crate) fn is_builtin_path(path: &[String]) -> bool {
    let segs: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
    crate::stdlib::registry::is_builtin(&segs)
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
/// Substitute generic type-param names with concrete types in an expression
/// tree. Used by monomorphization to replace `T` with `int` etc. in path calls.
///
/// **Exhaustive over Expr**: when a new variant is added to `Expr`, this match
/// will refuse to compile until the new variant is given a recursion arm. This
/// is deliberate — previously a `_ => {}` wildcard silently dropped Closure
/// (so generic functions containing closures didn't get monomorphized), Tuple,
/// Range, IfLet, As/Try/Await, FString, and Return.
pub(crate) fn substitute_type_params(expr: &mut crate::ast::Expr, subst: &[(String, String)]) {
    use crate::ast::Expr;
    use crate::ast::FStringPart;
    match expr {
        Expr::PathCall { path, args, .. } => {
            if let Some(concrete) = subst.iter().find(|(p, _)| **p == path[0]).map(|(_, c)| c) {
                path[0] = concrete.clone();
            }
            for arg in args {
                substitute_type_params(arg, subst);
            }
        }
        Expr::Call { callee, args, .. } => {
            substitute_type_params(callee, subst);
            for arg in args {
                substitute_type_params(arg, subst);
            }
        }
        Expr::MethodCall { object, args, .. } => {
            substitute_type_params(object, subst);
            for arg in args {
                substitute_type_params(arg, subst);
            }
        }
        Expr::MacroCall { args, .. } => {
            for arg in args {
                substitute_type_params(arg, subst);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            substitute_type_params(left, subst);
            substitute_type_params(right, subst);
        }
        Expr::UnaryOp { expr: inner, .. } => substitute_type_params(inner, subst),
        Expr::Block(block) => substitute_type_params_in_block(block, subst),
        Expr::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            substitute_type_params(condition, subst);
            substitute_type_params_in_block(then_block, subst);
            if let Some(else_expr) = else_block {
                substitute_type_params(else_expr, subst);
            }
        }
        Expr::IfLet {
            expr: scrutinee,
            then_block,
            else_block,
            ..
        } => {
            substitute_type_params(scrutinee, subst);
            substitute_type_params_in_block(then_block, subst);
            if let Some(else_expr) = else_block {
                substitute_type_params(else_expr, subst);
            }
        }
        Expr::Match {
            expr: scrutinee,
            arms,
            ..
        } => {
            substitute_type_params(scrutinee, subst);
            for arm in arms {
                if let Some(g) = &mut arm.guard {
                    substitute_type_params(g, subst);
                }
                substitute_type_params(&mut arm.body, subst);
            }
        }
        Expr::Assign { target, value, .. } | Expr::CompoundAssign { target, value, .. } => {
            substitute_type_params(target, subst);
            substitute_type_params(value, subst);
        }
        Expr::Array { elements, .. } | Expr::Tuple { elements, .. } => {
            for elem in elements {
                substitute_type_params(elem, subst);
            }
        }
        Expr::StructInit { fields, base, .. } => {
            for (_, field_expr) in fields {
                substitute_type_params(field_expr, subst);
            }
            if let Some(b) = base {
                substitute_type_params(b, subst);
            }
        }
        Expr::FieldAccess { object, .. } => substitute_type_params(object, subst),
        Expr::Index { object, index, .. } => {
            substitute_type_params(object, subst);
            substitute_type_params(index, subst);
        }
        Expr::Grouped(inner, _) => substitute_type_params(inner, subst),
        Expr::Range { start, end, .. } => {
            if let Some(s) = start {
                substitute_type_params(s, subst);
            }
            if let Some(e) = end {
                substitute_type_params(e, subst);
            }
        }
        Expr::Repeat { value, count, .. } => {
            substitute_type_params(value, subst);
            substitute_type_params(count, subst);
        }
        Expr::Closure { body, .. } => substitute_type_params(body, subst),
        Expr::As { expr: inner, .. }
        | Expr::Try { expr: inner, .. }
        | Expr::Await { expr: inner, .. } => substitute_type_params(inner, subst),
        Expr::FString { parts, .. } => {
            for part in parts {
                if let FStringPart::Expr(e) = part {
                    substitute_type_params(e, subst);
                }
            }
        }
        Expr::Return { value, .. } => {
            if let Some(v) = value {
                substitute_type_params(v, subst);
            }
        }
        // Terminals — no subexpressions to recurse into.
        Expr::Ident(..)
        | Expr::IntLiteral(..)
        | Expr::FloatLiteral(..)
        | Expr::BoolLiteral(..)
        | Expr::StringLiteral(..)
        | Expr::CharLiteral(..)
        | Expr::Path { .. }
        | Expr::SelfRef(_) => {}
    }
}

fn substitute_type_params_in_block(block: &mut crate::ast::Block, subst: &[(String, String)]) {
    for stmt in &mut block.stmts {
        substitute_type_params_in_stmt(stmt, subst);
    }
}

/// Exhaustive over `Stmt` — see `substitute_type_params` for the rationale.
fn substitute_type_params_in_stmt(stmt: &mut crate::ast::Stmt, subst: &[(String, String)]) {
    use crate::ast::Stmt;
    match stmt {
        Stmt::Expr { expr, .. } => substitute_type_params(expr, subst),
        Stmt::Let { value, .. } => {
            if let Some(v) = value {
                substitute_type_params(v, subst);
            }
        }
        Stmt::Return { value, .. } => {
            if let Some(v) = value {
                substitute_type_params(v, subst);
            }
        }
        Stmt::While {
            condition, body, ..
        } => {
            substitute_type_params(condition, subst);
            substitute_type_params_in_block(body, subst);
        }
        Stmt::Loop { body, .. } => substitute_type_params_in_block(body, subst),
        Stmt::For { iterable, body, .. } => {
            substitute_type_params(iterable, subst);
            substitute_type_params_in_block(body, subst);
        }
        Stmt::ForDestructure { iterable, body, .. } => {
            substitute_type_params(iterable, subst);
            substitute_type_params_in_block(body, subst);
        }
        Stmt::WhileLet { expr, body, .. } => {
            substitute_type_params(expr, subst);
            substitute_type_params_in_block(body, subst);
        }
        Stmt::LetPattern { value, .. } => substitute_type_params(value, subst),
        Stmt::Break { value: Some(v), .. } => substitute_type_params(v, subst),
        // No expressions to substitute into.
        Stmt::Break { value: None, .. } | Stmt::Continue { .. } | Stmt::Use(_) | Stmt::Item(_) => {}
    }
}
