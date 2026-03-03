//! Tree-walking interpreter for the Ferrite language.
//!
//! Evaluates the AST produced by the parser, executing statements and
//! evaluating expressions to produce [`Value`]s.

use std::rc::Rc;

use crate::ast::*;
use crate::env::{Env, Environment};
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

/// The Ferrite interpreter.
pub struct Interpreter {
    /// The global environment.
    env: Env,
    /// Captured output (for testing). If `None`, prints to stdout.
    output: Option<Vec<String>>,
}

impl Interpreter {
    /// Create a new interpreter with a fresh global environment.
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
            output: None,
        }
    }

    /// Create an interpreter that captures output instead of printing.
    pub fn new_with_captured_output() -> Self {
        Self {
            env: Environment::new(),
            output: Some(Vec::new()),
        }
    }

    /// Create an interpreter with an existing environment (for REPL).
    pub fn with_env(env: Env) -> Self {
        Self { env, output: None }
    }

    /// Get captured output (for testing).
    pub fn captured_output(&self) -> &[String] {
        self.output.as_deref().unwrap_or(&[])
    }

    /// Get the current environment (for REPL persistence).
    pub fn env(&self) -> &Env {
        &self.env
    }

    /// Execute a complete program: register all functions, then call `main()`.
    pub fn execute_program(&mut self, program: &Program) -> Result<Value, FerriError> {
        // Register all top-level functions
        for item in &program.items {
            self.register_item(item)?;
        }

        // Look for and call main()
        let main_fn = self.env.borrow().get("main").map_err(|_| FerriError::Runtime {
            message: "no `main` function found".into(),
            line: 0,
            column: 0,
        })?;

        if let Value::Function { .. } = &main_fn {
            self.call_function(&main_fn, &[], 0, 0)
        } else {
            Err(FerriError::Runtime {
                message: "`main` is not a function".into(),
                line: 0,
                column: 0,
            })
        }
    }

    /// Execute a single statement in the current environment (for REPL).
    pub fn execute_stmt(&mut self, stmt: &Stmt) -> Result<Value, FerriError> {
        self.eval_stmt(stmt, &self.env.clone())
    }

    /// Register a single item in the current environment (for REPL).
    pub fn register_item(&mut self, item: &Item) -> Result<(), FerriError> {
        match item {
            Item::Function(f) => {
                let value = Value::Function {
                    name: f.name.clone(),
                    params: f.params.clone(),
                    return_type: f.return_type.clone(),
                    body: f.body.clone(),
                    closure_env: Rc::clone(&self.env),
                };
                self.env.borrow_mut().define(f.name.clone(), value, false);
                Ok(())
            }
        }
    }

    // === Statement evaluation ===

    fn eval_stmt(&mut self, stmt: &Stmt, env: &Env) -> Result<Value, FerriError> {
        match stmt {
            Stmt::Let {
                name,
                mutable,
                value,
                ..
            } => {
                let val = if let Some(expr) = value {
                    self.eval_expr(expr, env)?
                } else {
                    Value::Unit
                };
                env.borrow_mut().define(name.clone(), val, *mutable);
                Ok(Value::Unit)
            }
            Stmt::Expr { expr, .. } => self.eval_expr(expr, env),
            Stmt::Return { value, .. } => {
                let val = if let Some(expr) = value {
                    self.eval_expr(expr, env)?
                } else {
                    Value::Unit
                };
                Err(FerriError::Return(val))
            }
            Stmt::While { condition, body, .. } => {
                loop {
                    let cond = self.eval_expr(condition, env)?;
                    if !cond.is_truthy() {
                        break;
                    }
                    match self.eval_block(body, env) {
                        Ok(_) => {}
                        Err(FerriError::Break(val)) => return Ok(val.unwrap_or(Value::Unit)),
                        Err(FerriError::Continue) => continue,
                        Err(e) => return Err(e),
                    }
                }
                Ok(Value::Unit)
            }
            Stmt::Loop { body, .. } => {
                loop {
                    match self.eval_block(body, env) {
                        Ok(_) => {}
                        Err(FerriError::Break(val)) => return Ok(val.unwrap_or(Value::Unit)),
                        Err(FerriError::Continue) => continue,
                        Err(e) => return Err(e),
                    }
                }
            }
            Stmt::For { name, iterable, body, .. } => {
                let iter_val = self.eval_expr(iterable, env)?;
                let values = self.value_to_iter(&iter_val, iterable.span())?;
                let for_env = Environment::child(env);
                for_env.borrow_mut().define(name.clone(), Value::Unit, true);
                for val in values {
                    for_env.borrow_mut().set(name, val).ok();
                    match self.eval_block(body, &for_env) {
                        Ok(_) => {}
                        Err(FerriError::Break(val)) => return Ok(val.unwrap_or(Value::Unit)),
                        Err(FerriError::Continue) => continue,
                        Err(e) => return Err(e),
                    }
                }
                Ok(Value::Unit)
            }
            Stmt::Break { value, .. } => {
                let val = if let Some(expr) = value {
                    Some(self.eval_expr(expr, env)?)
                } else {
                    None
                };
                Err(FerriError::Break(val))
            }
            Stmt::Continue { .. } => Err(FerriError::Continue),
        }
    }

    fn eval_block(&mut self, block: &Block, env: &Env) -> Result<Value, FerriError> {
        let block_env = Environment::child(env);
        let mut result = Value::Unit;

        for (i, stmt) in block.stmts.iter().enumerate() {
            let is_last = i == block.stmts.len() - 1;
            let val = self.eval_stmt(stmt, &block_env)?;

            if is_last {
                match stmt {
                    // Tail expression (no semicolon) becomes block value
                    Stmt::Expr { has_semicolon, .. } if !has_semicolon => {
                        result = val;
                    }
                    // Loop/while/for return their break value when last
                    Stmt::Loop { .. } | Stmt::While { .. } | Stmt::For { .. } => {
                        result = val;
                    }
                    _ => {
                        result = Value::Unit;
                    }
                }
            }
        }

        Ok(result)
    }

    // === Expression evaluation ===

    fn eval_expr(&mut self, expr: &Expr, env: &Env) -> Result<Value, FerriError> {
        match expr {
            Expr::IntLiteral(n, _) => Ok(Value::Integer(*n)),
            Expr::FloatLiteral(n, _) => Ok(Value::Float(*n)),
            Expr::BoolLiteral(b, _) => Ok(Value::Bool(*b)),
            Expr::StringLiteral(s, _) => Ok(Value::String(s.clone())),
            Expr::CharLiteral(c, _) => Ok(Value::Char(*c)),

            Expr::Ident(name, span) => env.borrow().get(name).map_err(|_| FerriError::Runtime {
                message: format!("undefined variable '{name}'"),
                line: span.line,
                column: span.column,
            }),

            Expr::BinaryOp {
                left, op, right, span,
            } => {
                let lval = self.eval_expr(left, env)?;
                let rval = self.eval_expr(right, env)?;
                self.eval_binary_op(&lval, *op, &rval, span.line, span.column)
            }

            Expr::UnaryOp { op, expr: inner, span } => {
                let val = self.eval_expr(inner, env)?;
                self.eval_unary_op(*op, &val, span.line, span.column)
            }

            Expr::Call { callee, args, span } => {
                let func = self.eval_expr(callee, env)?;
                let mut arg_values = Vec::with_capacity(args.len());
                for arg in args {
                    arg_values.push(self.eval_expr(arg, env)?);
                }
                self.call_function(&func, &arg_values, span.line, span.column)
            }

            Expr::MacroCall { name, args, span } => {
                self.eval_macro_call(name, args, env, span.line, span.column)
            }

            Expr::Block(block) => self.eval_block(block, env),

            Expr::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let cond = self.eval_expr(condition, env)?;
                if cond.is_truthy() {
                    self.eval_block(then_block, env)
                } else if let Some(else_expr) = else_block {
                    self.eval_expr(else_expr, env)
                } else {
                    Ok(Value::Unit)
                }
            }

            Expr::Assign { target, value, span } => {
                let val = self.eval_expr(value, env)?;
                if let Expr::Ident(name, _) = target.as_ref() {
                    env.borrow_mut().set(name, val).map_err(|e| {
                        FerriError::Runtime {
                            message: e.to_string(),
                            line: span.line,
                            column: span.column,
                        }
                    })?;
                    Ok(Value::Unit)
                } else {
                    Err(FerriError::Runtime {
                        message: "invalid assignment target".into(),
                        line: span.line,
                        column: span.column,
                    })
                }
            }

            Expr::CompoundAssign {
                target, op, value, span,
            } => {
                if let Expr::Ident(name, _) = target.as_ref() {
                    let current = env.borrow().get(name).map_err(|_| FerriError::Runtime {
                        message: format!("undefined variable '{name}'"),
                        line: span.line,
                        column: span.column,
                    })?;
                    let rval = self.eval_expr(value, env)?;
                    let new_val = self.eval_binary_op(&current, *op, &rval, span.line, span.column)?;
                    env.borrow_mut().set(name, new_val).map_err(|e| {
                        FerriError::Runtime {
                            message: e.to_string(),
                            line: span.line,
                            column: span.column,
                        }
                    })?;
                    Ok(Value::Unit)
                } else {
                    Err(FerriError::Runtime {
                        message: "invalid compound assignment target".into(),
                        line: span.line,
                        column: span.column,
                    })
                }
            }

            Expr::Grouped(inner, _) => self.eval_expr(inner, env),

            Expr::Match { expr, arms, span } => {
                let val = self.eval_expr(expr, env)?;
                for arm in arms {
                    if self.pattern_matches(&arm.pattern, &val) {
                        // If pattern is a variable binding, create a scope with it
                        if let Pattern::Ident(name, _) = &arm.pattern {
                            let match_env = Environment::child(env);
                            match_env.borrow_mut().define(name.clone(), val.clone(), false);
                            return self.eval_expr(&arm.body, &match_env);
                        }
                        return self.eval_expr(&arm.body, env);
                    }
                }
                Err(FerriError::Runtime {
                    message: "non-exhaustive match: no arm matched".into(),
                    line: span.line,
                    column: span.column,
                })
            }

            Expr::Range { start, end, inclusive, span } => {
                let start_val = self.eval_expr(start, env)?;
                let end_val = self.eval_expr(end, env)?;
                match (&start_val, &end_val) {
                    (Value::Integer(s), Value::Integer(e)) => {
                        let end_n = if *inclusive { *e + 1 } else { *e };
                        Ok(Value::Range(*s, end_n))
                    }
                    _ => Err(FerriError::Runtime {
                        message: format!(
                            "range bounds must be integers, got {} and {}",
                            start_val.type_name(),
                            end_val.type_name()
                        ),
                        line: span.line,
                        column: span.column,
                    }),
                }
            }
        }
    }

    // === Binary operations ===

    fn eval_binary_op(
        &self,
        left: &Value,
        op: BinOp,
        right: &Value,
        line: usize,
        col: usize,
    ) -> Result<Value, FerriError> {
        match (left, op, right) {
            // Integer arithmetic
            (Value::Integer(a), BinOp::Add, Value::Integer(b)) => Ok(Value::Integer(a + b)),
            (Value::Integer(a), BinOp::Sub, Value::Integer(b)) => Ok(Value::Integer(a - b)),
            (Value::Integer(a), BinOp::Mul, Value::Integer(b)) => Ok(Value::Integer(a * b)),
            (Value::Integer(a), BinOp::Div, Value::Integer(b)) => {
                if *b == 0 {
                    Err(FerriError::Runtime {
                        message: "division by zero".into(),
                        line,
                        column: col,
                    })
                } else {
                    Ok(Value::Integer(a / b))
                }
            }
            (Value::Integer(a), BinOp::Mod, Value::Integer(b)) => {
                if *b == 0 {
                    Err(FerriError::Runtime {
                        message: "modulo by zero".into(),
                        line,
                        column: col,
                    })
                } else {
                    Ok(Value::Integer(a % b))
                }
            }

            // Float arithmetic
            (Value::Float(a), BinOp::Add, Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Float(a), BinOp::Sub, Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Float(a), BinOp::Mul, Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Float(a), BinOp::Div, Value::Float(b)) => Ok(Value::Float(a / b)),
            (Value::Float(a), BinOp::Mod, Value::Float(b)) => Ok(Value::Float(a % b)),

            // Mixed int/float arithmetic
            (Value::Integer(a), BinOp::Add, Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), BinOp::Add, Value::Integer(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::Integer(a), BinOp::Sub, Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Value::Float(a), BinOp::Sub, Value::Integer(b)) => Ok(Value::Float(a - *b as f64)),
            (Value::Integer(a), BinOp::Mul, Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Value::Float(a), BinOp::Mul, Value::Integer(b)) => Ok(Value::Float(a * *b as f64)),
            (Value::Integer(a), BinOp::Div, Value::Float(b)) => Ok(Value::Float(*a as f64 / b)),
            (Value::Float(a), BinOp::Div, Value::Integer(b)) => Ok(Value::Float(a / *b as f64)),

            // String concatenation
            (Value::String(a), BinOp::Add, Value::String(b)) => {
                Ok(Value::String(format!("{a}{b}")))
            }

            // Comparison operators (work on any PartialOrd pair)
            (l, BinOp::Eq, r) => Ok(Value::Bool(l == r)),
            (l, BinOp::NotEq, r) => Ok(Value::Bool(l != r)),
            (l, BinOp::Lt, r) => Ok(Value::Bool(l < r)),
            (l, BinOp::Gt, r) => Ok(Value::Bool(l > r)),
            (l, BinOp::LtEq, r) => Ok(Value::Bool(l <= r)),
            (l, BinOp::GtEq, r) => Ok(Value::Bool(l >= r)),

            // Logical operators
            (Value::Bool(a), BinOp::And, Value::Bool(b)) => Ok(Value::Bool(*a && *b)),
            (Value::Bool(a), BinOp::Or, Value::Bool(b)) => Ok(Value::Bool(*a || *b)),

            // Bitwise operators
            (Value::Integer(a), BinOp::BitAnd, Value::Integer(b)) => Ok(Value::Integer(a & b)),
            (Value::Integer(a), BinOp::BitOr, Value::Integer(b)) => Ok(Value::Integer(a | b)),
            (Value::Integer(a), BinOp::BitXor, Value::Integer(b)) => Ok(Value::Integer(a ^ b)),
            (Value::Integer(a), BinOp::Shl, Value::Integer(b)) => Ok(Value::Integer(a << b)),
            (Value::Integer(a), BinOp::Shr, Value::Integer(b)) => Ok(Value::Integer(a >> b)),

            _ => Err(FerriError::Runtime {
                message: format!(
                    "unsupported operation: {} {op} {}",
                    left.type_name(),
                    right.type_name()
                ),
                line,
                column: col,
            }),
        }
    }

    // === Unary operations ===

    fn eval_unary_op(
        &self,
        op: UnaryOp,
        val: &Value,
        line: usize,
        col: usize,
    ) -> Result<Value, FerriError> {
        match (op, val) {
            (UnaryOp::Neg, Value::Integer(n)) => Ok(Value::Integer(-n)),
            (UnaryOp::Neg, Value::Float(n)) => Ok(Value::Float(-n)),
            (UnaryOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
            // & (reference) — just pass through the value (no borrow checker!)
            (UnaryOp::Ref, v) => Ok(v.clone()),
            // * (deref) — just pass through the value
            (UnaryOp::Deref, v) => Ok(v.clone()),
            _ => Err(FerriError::Runtime {
                message: format!("unsupported unary operation: {op}{}", val.type_name()),
                line,
                column: col,
            }),
        }
    }

    // === Function calls ===

    fn call_function(
        &mut self,
        func: &Value,
        args: &[Value],
        line: usize,
        col: usize,
    ) -> Result<Value, FerriError> {
        let Value::Function {
            name,
            params,
            body,
            closure_env,
            ..
        } = func
        else {
            return Err(FerriError::Runtime {
                message: format!("'{}' is not callable", func.type_name()),
                line,
                column: col,
            });
        };

        if args.len() != params.len() {
            return Err(FerriError::Runtime {
                message: format!(
                    "function '{name}' expects {} argument(s), got {}",
                    params.len(),
                    args.len()
                ),
                line,
                column: col,
            });
        }

        // Create a new scope from the closure environment
        let call_env = Environment::child(closure_env);
        for (param, arg) in params.iter().zip(args.iter()) {
            call_env
                .borrow_mut()
                .define(param.name.clone(), arg.clone(), true);
        }

        // Execute the function body
        match self.eval_block(body, &call_env) {
            Ok(val) => Ok(val),
            Err(FerriError::Return(val)) => Ok(val),
            Err(e) => Err(e),
        }
    }

    // === Pattern matching ===

    fn pattern_matches(&self, pattern: &Pattern, value: &Value) -> bool {
        match pattern {
            Pattern::Wildcard(_) => true,
            Pattern::Ident(_, _) => true, // Variable pattern always matches
            Pattern::Literal(expr) => match (expr, value) {
                (Expr::IntLiteral(n, _), Value::Integer(v)) => *n == *v,
                (Expr::FloatLiteral(n, _), Value::Float(v)) => *n == *v,
                (Expr::BoolLiteral(b, _), Value::Bool(v)) => *b == *v,
                (Expr::StringLiteral(s, _), Value::String(v)) => s == v,
                (Expr::CharLiteral(c, _), Value::Char(v)) => *c == *v,
                (Expr::UnaryOp { op: UnaryOp::Neg, expr, .. }, Value::Integer(v)) => {
                    if let Expr::IntLiteral(n, _) = expr.as_ref() {
                        -*n == *v
                    } else {
                        false
                    }
                }
                _ => false,
            },
        }
    }

    // === Iteration ===

    fn value_to_iter(&self, value: &Value, span: Span) -> Result<Vec<Value>, FerriError> {
        match value {
            Value::Range(start, end) => {
                Ok((*start..*end).map(Value::Integer).collect())
            }
            _ => Err(FerriError::Runtime {
                message: format!("cannot iterate over {}", value.type_name()),
                line: span.line,
                column: span.column,
            }),
        }
    }

    // === Macro calls (println!, print!, etc.) ===

    fn eval_macro_call(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &Env,
        line: usize,
        col: usize,
    ) -> Result<Value, FerriError> {
        match name {
            "println" => {
                let output = self.format_macro_args(args, env, line, col)?;
                self.write_output(&output);
                self.write_output("\n");
                Ok(Value::Unit)
            }
            "print" => {
                let output = self.format_macro_args(args, env, line, col)?;
                self.write_output(&output);
                Ok(Value::Unit)
            }
            _ => Err(FerriError::Runtime {
                message: format!("unknown macro '{name}!'"),
                line,
                column: col,
            }),
        }
    }

    fn format_macro_args(
        &mut self,
        args: &[Expr],
        env: &Env,
        line: usize,
        col: usize,
    ) -> Result<String, FerriError> {
        if args.is_empty() {
            return Ok(String::new());
        }

        // First argument should be a format string
        let fmt_val = self.eval_expr(&args[0], env)?;
        let Value::String(fmt_str) = fmt_val else {
            // If not a string, just print the value
            return Ok(format!("{fmt_val}"));
        };

        let mut result = String::new();
        let mut arg_idx = 1;
        let mut chars = fmt_str.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                if chars.peek() == Some(&'{') {
                    // Escaped `{{` → literal `{`
                    chars.next();
                    result.push('{');
                } else if chars.peek() == Some(&'}') {
                    // `{}` placeholder
                    chars.next();
                    if arg_idx >= args.len() {
                        return Err(FerriError::Runtime {
                            message: "not enough arguments for format string".into(),
                            line,
                            column: col,
                        });
                    }
                    let val = self.eval_expr(&args[arg_idx], env)?;
                    result.push_str(&format!("{val}"));
                    arg_idx += 1;
                } else if chars.peek() == Some(&':') {
                    // `{:?}` debug format — consume until `}`
                    for c in chars.by_ref() {
                        if c == '}' {
                            break;
                        }
                    }
                    if arg_idx >= args.len() {
                        return Err(FerriError::Runtime {
                            message: "not enough arguments for format string".into(),
                            line,
                            column: col,
                        });
                    }
                    let val = self.eval_expr(&args[arg_idx], env)?;
                    // Debug format — show type info for strings
                    result.push_str(&debug_format(&val));
                    arg_idx += 1;
                } else {
                    result.push(ch);
                }
            } else if ch == '}' && chars.peek() == Some(&'}') {
                // Escaped `}}` → literal `}`
                chars.next();
                result.push('}');
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    }

    fn write_output(&mut self, s: &str) {
        if let Some(ref mut output) = self.output {
            if let Some(last) = output.last_mut() {
                if !last.ends_with('\n') {
                    last.push_str(s);
                    return;
                }
            }
            output.push(s.to_string());
        } else {
            print!("{s}");
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

/// Debug format for values (used by `{:?}`).
fn debug_format(val: &Value) -> String {
    match val {
        Value::String(s) => format!("\"{s}\""),
        Value::Char(c) => format!("'{c}'"),
        other => format!("{other}"),
    }
}

/// Convenience function: parse and execute a Ferrite program.
pub fn run(source: &str) -> Result<Value, FerriError> {
    let program = crate::parser::parse(source)?;
    let mut interp = Interpreter::new();
    interp.execute_program(&program)
}

/// Run a program and capture its output (for testing).
pub fn run_capturing(source: &str) -> Result<(Value, Vec<String>), FerriError> {
    let program = crate::parser::parse(source)?;
    let mut interp = Interpreter::new_with_captured_output();
    let result = interp.execute_program(&program)?;
    Ok((result, interp.captured_output().to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_and_capture(src: &str) -> Vec<String> {
        let (_, output) = run_capturing(src).unwrap();
        output
    }

    fn run_and_get_value(src: &str) -> Value {
        let (val, _) = run_capturing(src).unwrap();
        val
    }

    // === Basic execution ===

    #[test]
    fn test_empty_main() {
        let val = run_and_get_value("fn main() {}");
        assert_eq!(val, Value::Unit);
    }

    #[test]
    fn test_println_string() {
        let output = run_and_capture(r#"fn main() { println!("Hello, Ferrite!"); }"#);
        assert_eq!(output, vec!["Hello, Ferrite!\n"]);
    }

    #[test]
    fn test_println_format() {
        let output = run_and_capture(r#"fn main() { let x = 42; println!("x = {}", x); }"#);
        assert_eq!(output, vec!["x = 42\n"]);
    }

    #[test]
    fn test_println_multiple_args() {
        let output = run_and_capture(
            r#"fn main() { let a = 1; let b = 2; println!("{} + {} = {}", a, b, a + b); }"#,
        );
        assert_eq!(output, vec!["1 + 2 = 3\n"]);
    }

    // === Variables ===

    #[test]
    fn test_let_binding() {
        let output = run_and_capture(
            r#"fn main() { let x = 10; println!("{}", x); }"#,
        );
        assert_eq!(output, vec!["10\n"]);
    }

    #[test]
    fn test_let_mut_and_assign() {
        let output = run_and_capture(
            r#"fn main() { let mut x = 1; x = 2; println!("{}", x); }"#,
        );
        assert_eq!(output, vec!["2\n"]);
    }

    #[test]
    fn test_immutable_assign_error() {
        let result = run(r#"fn main() { let x = 1; x = 2; }"#);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot assign to immutable"));
    }

    #[test]
    fn test_undefined_variable_error() {
        let result = run(r#"fn main() { println!("{}", x); }"#);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("undefined variable"));
    }

    #[test]
    fn test_shadowing() {
        let output = run_and_capture(
            r#"fn main() { let x = 1; let x = "hello"; println!("{}", x); }"#,
        );
        assert_eq!(output, vec!["hello\n"]);
    }

    // === Arithmetic ===

    #[test]
    fn test_integer_arithmetic() {
        let output = run_and_capture(
            r#"fn main() { println!("{}", 2 + 3 * 4); }"#,
        );
        assert_eq!(output, vec!["14\n"]);
    }

    #[test]
    fn test_float_arithmetic() {
        let output = run_and_capture(
            r#"fn main() { println!("{}", 1.5 + 2.5); }"#,
        );
        assert_eq!(output, vec!["4.0\n"]);
    }

    #[test]
    fn test_division_by_zero() {
        let result = run(r#"fn main() { let x = 1 / 0; }"#);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("division by zero"));
    }

    #[test]
    fn test_string_concatenation() {
        let output = run_and_capture(
            r#"fn main() { let s = "hello" + " " + "world"; println!("{}", s); }"#,
        );
        assert_eq!(output, vec!["hello world\n"]);
    }

    #[test]
    fn test_negation() {
        let output = run_and_capture(
            r#"fn main() { let x = 5; println!("{}", -x); }"#,
        );
        assert_eq!(output, vec!["-5\n"]);
    }

    // === Comparisons ===

    #[test]
    fn test_comparisons() {
        let output = run_and_capture(
            r#"fn main() { println!("{} {} {} {}", 1 < 2, 2 > 1, 1 == 1, 1 != 2); }"#,
        );
        assert_eq!(output, vec!["true true true true\n"]);
    }

    // === Logical operators ===

    #[test]
    fn test_logical_and_or() {
        let output = run_and_capture(
            r#"fn main() { println!("{} {}", true && false, true || false); }"#,
        );
        assert_eq!(output, vec!["false true\n"]);
    }

    #[test]
    fn test_logical_not() {
        let output = run_and_capture(
            r#"fn main() { println!("{}", !true); }"#,
        );
        assert_eq!(output, vec!["false\n"]);
    }

    // === Functions ===

    #[test]
    fn test_function_call() {
        let output = run_and_capture(
            r#"
fn add(a: i64, b: i64) -> i64 {
    a + b
}

fn main() {
    let result = add(3, 4);
    println!("{}", result);
}
"#,
        );
        assert_eq!(output, vec!["7\n"]);
    }

    #[test]
    fn test_function_return() {
        let output = run_and_capture(
            r#"
fn early(x: i64) -> i64 {
    if x > 0 {
        return x;
    }
    return 0;
}

fn main() {
    println!("{}", early(5));
    println!("{}", early(-1));
}
"#,
        );
        assert_eq!(output, vec!["5\n", "0\n"]);
    }

    #[test]
    fn test_tail_expression() {
        let output = run_and_capture(
            r#"
fn double(x: i64) -> i64 {
    x * 2
}

fn main() {
    println!("{}", double(21));
}
"#,
        );
        assert_eq!(output, vec!["42\n"]);
    }

    #[test]
    fn test_wrong_arg_count() {
        let result = run(
            r#"
fn foo(a: i64) -> i64 { a }
fn main() { foo(1, 2); }
"#,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expects 1 argument"));
    }

    #[test]
    fn test_recursive_function() {
        let output = run_and_capture(
            r#"
fn factorial(n: i64) -> i64 {
    if n <= 1 {
        return 1;
    }
    n * factorial(n - 1)
}

fn main() {
    println!("{}", factorial(5));
}
"#,
        );
        assert_eq!(output, vec!["120\n"]);
    }

    // === If/else ===

    #[test]
    fn test_if_true() {
        let output = run_and_capture(
            r#"fn main() { if true { println!("yes"); } }"#,
        );
        assert_eq!(output, vec!["yes\n"]);
    }

    #[test]
    fn test_if_false() {
        let output = run_and_capture(
            r#"fn main() { if false { println!("yes"); } }"#,
        );
        assert!(output.is_empty());
    }

    #[test]
    fn test_if_else() {
        let output = run_and_capture(
            r#"fn main() { let x = if true { 1 } else { 2 }; println!("{}", x); }"#,
        );
        assert_eq!(output, vec!["1\n"]);
    }

    #[test]
    fn test_if_else_if() {
        let output = run_and_capture(
            r#"
fn classify(x: i64) -> i64 {
    if x > 0 {
        1
    } else if x < 0 {
        -1
    } else {
        0
    }
}

fn main() {
    println!("{} {} {}", classify(5), classify(-3), classify(0));
}
"#,
        );
        assert_eq!(output, vec!["1 -1 0\n"]);
    }

    // === Block expressions ===

    #[test]
    fn test_block_value() {
        let output = run_and_capture(
            r#"fn main() { let x = { let y = 10; y + 1 }; println!("{}", x); }"#,
        );
        assert_eq!(output, vec!["11\n"]);
    }

    // === Compound assignment ===

    #[test]
    fn test_compound_assignment() {
        let output = run_and_capture(
            r#"fn main() { let mut x = 10; x += 5; x -= 3; println!("{}", x); }"#,
        );
        assert_eq!(output, vec!["12\n"]);
    }

    // === Reference syntax (no-op) ===

    #[test]
    fn test_reference_ignored() {
        let output = run_and_capture(
            r#"
fn greet(name: &String) {
    println!("Hello, {}!", name);
}
fn main() {
    let name = "Ferrite";
    greet(&name);
}
"#,
        );
        assert_eq!(output, vec!["Hello, Ferrite!\n"]);
    }

    // === No main function ===

    #[test]
    fn test_no_main_error() {
        let result = run("fn foo() {}");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no `main` function"));
    }

    // === Multiple println ===

    #[test]
    fn test_multiple_println() {
        let output = run_and_capture(
            r#"
fn main() {
    println!("line 1");
    println!("line 2");
    println!("line 3");
}
"#,
        );
        assert_eq!(output, vec!["line 1\n", "line 2\n", "line 3\n"]);
    }

    // === Full program ===

    #[test]
    fn test_fibonacci() {
        let output = run_and_capture(
            r#"
fn fib(n: i64) -> i64 {
    if n <= 1 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

fn main() {
    println!("{}", fib(10));
}
"#,
        );
        assert_eq!(output, vec!["55\n"]);
    }

    // === Phase 5: Control Flow ===

    #[test]
    fn test_while_loop() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut i = 0;
    let mut sum = 0;
    while i < 5 {
        sum += i;
        i += 1;
    }
    println!("{}", sum);
}
"#,
        );
        assert_eq!(output, vec!["10\n"]);
    }

    #[test]
    fn test_while_false() {
        let output = run_and_capture(
            r#"fn main() { while false { println!("never"); } println!("done"); }"#,
        );
        assert_eq!(output, vec!["done\n"]);
    }

    #[test]
    fn test_loop_with_break() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut i = 0;
    loop {
        if i >= 3 {
            break;
        }
        println!("{}", i);
        i += 1;
    }
}
"#,
        );
        assert_eq!(output, vec!["0\n", "1\n", "2\n"]);
    }

    #[test]
    fn test_loop_break_value() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut i = 0;
    let result = loop {
        i += 1;
        if i == 5 {
            break i * 10;
        }
    };
    println!("{}", result);
}
"#,
        );
        assert_eq!(output, vec!["50\n"]);
    }

    #[test]
    fn test_continue_in_while() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut i = 0;
    while i < 5 {
        i += 1;
        if i == 3 {
            continue;
        }
        println!("{}", i);
    }
}
"#,
        );
        assert_eq!(output, vec!["1\n", "2\n", "4\n", "5\n"]);
    }

    #[test]
    fn test_for_range() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut sum = 0;
    for i in 0..5 {
        sum += i;
    }
    println!("{}", sum);
}
"#,
        );
        assert_eq!(output, vec!["10\n"]);
    }

    #[test]
    fn test_for_range_inclusive() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut sum = 0;
    for i in 0..=5 {
        sum += i;
    }
    println!("{}", sum);
}
"#,
        );
        assert_eq!(output, vec!["15\n"]);
    }

    #[test]
    fn test_for_with_break() {
        let output = run_and_capture(
            r#"
fn main() {
    for i in 0..10 {
        if i == 3 {
            break;
        }
        println!("{}", i);
    }
}
"#,
        );
        assert_eq!(output, vec!["0\n", "1\n", "2\n"]);
    }

    #[test]
    fn test_for_with_continue() {
        let output = run_and_capture(
            r#"
fn main() {
    for i in 0..5 {
        if i % 2 == 0 {
            continue;
        }
        println!("{}", i);
    }
}
"#,
        );
        assert_eq!(output, vec!["1\n", "3\n"]);
    }

    #[test]
    fn test_match_literals() {
        let output = run_and_capture(
            r#"
fn main() {
    let x = 2;
    let result = match x {
        1 => "one",
        2 => "two",
        3 => "three",
        _ => "other",
    };
    println!("{}", result);
}
"#,
        );
        assert_eq!(output, vec!["two\n"]);
    }

    #[test]
    fn test_match_wildcard() {
        let output = run_and_capture(
            r#"
fn main() {
    let x = 99;
    let result = match x {
        1 => "one",
        _ => "other",
    };
    println!("{}", result);
}
"#,
        );
        assert_eq!(output, vec!["other\n"]);
    }

    #[test]
    fn test_match_with_blocks() {
        let output = run_and_capture(
            r#"
fn main() {
    let x = 1;
    match x {
        1 => {
            println!("it's one!");
        }
        _ => {
            println!("something else");
        }
    }
}
"#,
        );
        assert_eq!(output, vec!["it's one!\n"]);
    }

    #[test]
    fn test_match_string() {
        let output = run_and_capture(
            r#"
fn main() {
    let cmd = "hello";
    let result = match cmd {
        "hello" => "greeting",
        "bye" => "farewell",
        _ => "unknown",
    };
    println!("{}", result);
}
"#,
        );
        assert_eq!(output, vec!["greeting\n"]);
    }

    #[test]
    fn test_match_bool() {
        let output = run_and_capture(
            r#"
fn main() {
    let x = true;
    let s = match x {
        true => "yes",
        false => "no",
    };
    println!("{}", s);
}
"#,
        );
        assert_eq!(output, vec!["yes\n"]);
    }

    #[test]
    fn test_match_variable_binding() {
        let output = run_and_capture(
            r#"
fn main() {
    let x = 42;
    let result = match x {
        n => n + 1,
    };
    println!("{}", result);
}
"#,
        );
        assert_eq!(output, vec!["43\n"]);
    }

    #[test]
    fn test_match_non_exhaustive_error() {
        let result = run(
            r#"
fn main() {
    let x = 5;
    match x {
        1 => "one",
        2 => "two",
    };
}
"#,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-exhaustive"));
    }

    #[test]
    fn test_nested_loops() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut count = 0;
    for i in 0..3 {
        for j in 0..3 {
            count += 1;
        }
    }
    println!("{}", count);
}
"#,
        );
        assert_eq!(output, vec!["9\n"]);
    }

    #[test]
    fn test_loop_in_function() {
        let output = run_and_capture(
            r#"
fn find_first_multiple(n: i64, target: i64) -> i64 {
    let mut i = 1;
    loop {
        if i * n >= target {
            return i * n;
        }
        i += 1;
    }
}

fn main() {
    println!("{}", find_first_multiple(7, 50));
}
"#,
        );
        assert_eq!(output, vec!["56\n"]);
    }

    #[test]
    fn test_fizzbuzz() {
        let output = run_and_capture(
            r#"
fn main() {
    for i in 1..=15 {
        if i % 15 == 0 {
            println!("FizzBuzz");
        } else if i % 3 == 0 {
            println!("Fizz");
        } else if i % 5 == 0 {
            println!("Buzz");
        } else {
            println!("{}", i);
        }
    }
}
"#,
        );
        assert_eq!(
            output,
            vec![
                "1\n", "2\n", "Fizz\n", "4\n", "Buzz\n", "Fizz\n", "7\n", "8\n", "Fizz\n",
                "Buzz\n", "11\n", "Fizz\n", "13\n", "14\n", "FizzBuzz\n"
            ]
        );
    }
}
