//! Tree-walking interpreter for the Ferrite language.
//!
//! Evaluates the AST produced by the parser, executing statements and
//! evaluating expressions to produce [`Value`]s.

use std::collections::HashMap;
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
    /// Registered struct definitions.
    struct_defs: HashMap<String, StructDef>,
    /// Registered enum definitions.
    enum_defs: HashMap<String, EnumDef>,
    /// Methods registered via `impl` blocks, keyed by type name.
    impl_methods: HashMap<String, Vec<FnDef>>,
    /// Current `Self` type name (set when executing impl methods).
    current_self_type: Option<String>,
}

impl Interpreter {
    /// Create a new interpreter with a fresh global environment.
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
            output: None,
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            impl_methods: HashMap::new(),
            current_self_type: None,
        }
    }

    /// Create an interpreter that captures output instead of printing.
    pub fn new_with_captured_output() -> Self {
        Self {
            env: Environment::new(),
            output: Some(Vec::new()),
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            impl_methods: HashMap::new(),
            current_self_type: None,
        }
    }

    /// Create an interpreter with an existing environment (for REPL).
    pub fn with_env(env: Env) -> Self {
        Self {
            env,
            output: None,
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            impl_methods: HashMap::new(),
            current_self_type: None,
        }
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
        let main_fn = self
            .env
            .borrow()
            .get("main")
            .map_err(|_| FerriError::Runtime {
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
            Item::Struct(s) => {
                self.struct_defs.insert(s.name.clone(), s.clone());
                Ok(())
            }
            Item::Enum(e) => {
                self.enum_defs.insert(e.name.clone(), e.clone());
                Ok(())
            }
            Item::Impl(i) => {
                let methods = self.impl_methods.entry(i.type_name.clone()).or_default();
                for method in &i.methods {
                    // Remove existing method with same name (allow re-definition)
                    methods.retain(|m| m.name != method.name);
                    methods.push(method.clone());
                }
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
                Err(FerriError::Return(Box::new(val)))
            }
            Stmt::While {
                condition, body, ..
            } => {
                loop {
                    let cond = self.eval_expr(condition, env)?;
                    if !cond.is_truthy() {
                        break;
                    }
                    match self.eval_block(body, env) {
                        Ok(_) => {}
                        Err(FerriError::Break(val)) => {
                            return Ok(val.map(|v| *v).unwrap_or(Value::Unit))
                        }
                        Err(FerriError::Continue) => continue,
                        Err(e) => return Err(e),
                    }
                }
                Ok(Value::Unit)
            }
            Stmt::Loop { body, .. } => loop {
                match self.eval_block(body, env) {
                    Ok(_) => {}
                    Err(FerriError::Break(val)) => {
                        return Ok(val.map(|v| *v).unwrap_or(Value::Unit))
                    }
                    Err(FerriError::Continue) => continue,
                    Err(e) => return Err(e),
                }
            },
            Stmt::For {
                name,
                iterable,
                body,
                ..
            } => {
                let iter_val = self.eval_expr(iterable, env)?;
                let values = self.value_to_iter(&iter_val, iterable.span())?;
                let for_env = Environment::child(env);
                for_env.borrow_mut().define(name.clone(), Value::Unit, true);
                for val in values {
                    for_env.borrow_mut().set(name, val).ok();
                    match self.eval_block(body, &for_env) {
                        Ok(_) => {}
                        Err(FerriError::Break(val)) => {
                            return Ok(val.map(|v| *v).unwrap_or(Value::Unit))
                        }
                        Err(FerriError::Continue) => continue,
                        Err(e) => return Err(e),
                    }
                }
                Ok(Value::Unit)
            }
            Stmt::Break { value, .. } => {
                let val = if let Some(expr) = value {
                    Some(Box::new(self.eval_expr(expr, env)?))
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
                left,
                op,
                right,
                span,
            } => {
                let lval = self.eval_expr(left, env)?;
                let rval = self.eval_expr(right, env)?;
                self.eval_binary_op(&lval, *op, &rval, span.line, span.column)
            }

            Expr::UnaryOp {
                op,
                expr: inner,
                span,
            } => {
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

            Expr::Assign {
                target,
                value,
                span,
            } => {
                let val = self.eval_expr(value, env)?;
                if let Expr::Ident(name, _) = target.as_ref() {
                    env.borrow_mut()
                        .set(name, val)
                        .map_err(|e| FerriError::Runtime {
                            message: e.to_string(),
                            line: span.line,
                            column: span.column,
                        })?;
                    Ok(Value::Unit)
                } else if let Expr::FieldAccess { object, field, .. } = target.as_ref() {
                    // Field assignment: `s.field = val`
                    if let Expr::Ident(name, _) = object.as_ref() {
                        let mut current =
                            env.borrow().get(name).map_err(|_| FerriError::Runtime {
                                message: format!("undefined variable '{name}'"),
                                line: span.line,
                                column: span.column,
                            })?;
                        if let Value::Struct { fields, .. } = &mut current {
                            if fields.contains_key(field) {
                                fields.insert(field.clone(), val);
                            } else {
                                return Err(FerriError::Runtime {
                                    message: format!("no field '{field}' on struct"),
                                    line: span.line,
                                    column: span.column,
                                });
                            }
                        } else {
                            return Err(FerriError::Runtime {
                                message: format!("cannot set field on {}", current.type_name()),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        env.borrow_mut()
                            .set(name, current)
                            .map_err(|e| FerriError::Runtime {
                                message: e.to_string(),
                                line: span.line,
                                column: span.column,
                            })?;
                        Ok(Value::Unit)
                    } else if let Expr::SelfRef(_) = object.as_ref() {
                        // self.field = val
                        let mut current =
                            env.borrow().get("self").map_err(|_| FerriError::Runtime {
                                message: "'self' not available in this context".into(),
                                line: span.line,
                                column: span.column,
                            })?;
                        if let Value::Struct { fields, .. } = &mut current {
                            if fields.contains_key(field) {
                                fields.insert(field.clone(), val);
                            } else {
                                return Err(FerriError::Runtime {
                                    message: format!("no field '{field}' on struct"),
                                    line: span.line,
                                    column: span.column,
                                });
                            }
                        } else {
                            return Err(FerriError::Runtime {
                                message: format!("cannot set field on {}", current.type_name()),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        env.borrow_mut()
                            .set("self", current)
                            .map_err(|e| FerriError::Runtime {
                                message: e.to_string(),
                                line: span.line,
                                column: span.column,
                            })?;
                        Ok(Value::Unit)
                    } else {
                        Err(FerriError::Runtime {
                            message: "invalid field assignment target".into(),
                            line: span.line,
                            column: span.column,
                        })
                    }
                } else if let Expr::Index { object, index, .. } = target.as_ref() {
                    // Index assignment: `v[0] = x`
                    let idx = self.eval_expr(index, env)?;
                    let Value::Integer(i) = idx else {
                        return Err(FerriError::Runtime {
                            message: format!("index must be integer, got {}", idx.type_name()),
                            line: span.line,
                            column: span.column,
                        });
                    };
                    let i = i as usize;
                    if let Expr::Ident(name, _) = object.as_ref() {
                        let mut current =
                            env.borrow().get(name).map_err(|_| FerriError::Runtime {
                                message: format!("undefined variable '{name}'"),
                                line: span.line,
                                column: span.column,
                            })?;
                        match &mut current {
                            Value::Vec(v) => {
                                if i >= v.len() {
                                    return Err(FerriError::Runtime {
                                        message: format!(
                                            "index out of bounds: len is {}, but index is {i}",
                                            v.len()
                                        ),
                                        line: span.line,
                                        column: span.column,
                                    });
                                }
                                v[i] = val;
                            }
                            _ => {
                                return Err(FerriError::Runtime {
                                    message: format!(
                                        "cannot index-assign into {}",
                                        current.type_name()
                                    ),
                                    line: span.line,
                                    column: span.column,
                                });
                            }
                        }
                        env.borrow_mut()
                            .set(name, current)
                            .map_err(|e| FerriError::Runtime {
                                message: e.to_string(),
                                line: span.line,
                                column: span.column,
                            })?;
                        Ok(Value::Unit)
                    } else {
                        Err(FerriError::Runtime {
                            message: "invalid index assignment target".into(),
                            line: span.line,
                            column: span.column,
                        })
                    }
                } else {
                    Err(FerriError::Runtime {
                        message: "invalid assignment target".into(),
                        line: span.line,
                        column: span.column,
                    })
                }
            }

            Expr::CompoundAssign {
                target,
                op,
                value,
                span,
            } => {
                if let Expr::Ident(name, _) = target.as_ref() {
                    let current = env.borrow().get(name).map_err(|_| FerriError::Runtime {
                        message: format!("undefined variable '{name}'"),
                        line: span.line,
                        column: span.column,
                    })?;
                    let rval = self.eval_expr(value, env)?;
                    let new_val =
                        self.eval_binary_op(&current, *op, &rval, span.line, span.column)?;
                    env.borrow_mut()
                        .set(name, new_val)
                        .map_err(|e| FerriError::Runtime {
                            message: e.to_string(),
                            line: span.line,
                            column: span.column,
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
                    if Self::pattern_matches(&arm.pattern, &val) {
                        let match_env = Environment::child(env);
                        Self::bind_pattern(&arm.pattern, &val, &match_env);
                        return self.eval_expr(&arm.body, &match_env);
                    }
                }
                Err(FerriError::Runtime {
                    message: "non-exhaustive match: no arm matched".into(),
                    line: span.line,
                    column: span.column,
                })
            }

            Expr::Range {
                start,
                end,
                inclusive,
                span,
            } => {
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

            Expr::Array { elements, .. } => {
                let vals: Vec<Value> = elements
                    .iter()
                    .map(|e| self.eval_expr(e, env))
                    .collect::<Result<_, _>>()?;
                Ok(Value::Vec(vals))
            }

            Expr::Tuple { elements, .. } => {
                let vals: Vec<Value> = elements
                    .iter()
                    .map(|e| self.eval_expr(e, env))
                    .collect::<Result<_, _>>()?;
                Ok(Value::Tuple(vals))
            }

            Expr::Index {
                object,
                index,
                span,
            } => {
                let obj = self.eval_expr(object, env)?;
                let idx = self.eval_expr(index, env)?;
                match (&obj, &idx) {
                    (Value::Vec(v), Value::Integer(i)) => {
                        let i = *i as usize;
                        v.get(i).cloned().ok_or_else(|| FerriError::Runtime {
                            message: format!(
                                "index out of bounds: len is {}, but index is {i}",
                                v.len()
                            ),
                            line: span.line,
                            column: span.column,
                        })
                    }
                    (Value::String(s), Value::Integer(i)) => {
                        let i = *i as usize;
                        s.chars()
                            .nth(i)
                            .map(Value::Char)
                            .ok_or_else(|| FerriError::Runtime {
                                message: format!(
                                    "index out of bounds: len is {}, but index is {i}",
                                    s.len()
                                ),
                                line: span.line,
                                column: span.column,
                            })
                    }
                    (Value::Tuple(t), Value::Integer(i)) => {
                        let i = *i as usize;
                        t.get(i).cloned().ok_or_else(|| FerriError::Runtime {
                            message: format!(
                                "index out of bounds: len is {}, but index is {i}",
                                t.len()
                            ),
                            line: span.line,
                            column: span.column,
                        })
                    }
                    _ => Err(FerriError::Runtime {
                        message: format!(
                            "cannot index {} with {}",
                            obj.type_name(),
                            idx.type_name()
                        ),
                        line: span.line,
                        column: span.column,
                    }),
                }
            }

            Expr::FieldAccess {
                object,
                field,
                span,
            } => {
                let obj = self.eval_expr(object, env)?;
                // Tuple index access: t.0, t.1 etc.
                if let Ok(idx) = field.parse::<usize>() {
                    match &obj {
                        Value::Tuple(t) => t.get(idx).cloned().ok_or_else(|| FerriError::Runtime {
                            message: format!(
                                "index out of bounds: len is {}, but index is {idx}",
                                t.len()
                            ),
                            line: span.line,
                            column: span.column,
                        }),
                        _ => Err(FerriError::Runtime {
                            message: format!(
                                "cannot access field `.{field}` on {}",
                                obj.type_name()
                            ),
                            line: span.line,
                            column: span.column,
                        }),
                    }
                } else if let Value::Struct { fields, .. } = &obj {
                    fields
                        .get(field)
                        .cloned()
                        .ok_or_else(|| FerriError::Runtime {
                            message: format!("no field `{field}` on struct {}", obj.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                } else {
                    Err(FerriError::Runtime {
                        message: format!("cannot access field `.{field}` on {}", obj.type_name()),
                        line: span.line,
                        column: span.column,
                    })
                }
            }

            Expr::MethodCall {
                object,
                method,
                args,
                span,
            } => {
                let obj = self.eval_expr(object, env)?;
                let arg_vals: Vec<Value> = args
                    .iter()
                    .map(|a| self.eval_expr(a, env))
                    .collect::<Result<_, _>>()?;
                self.call_method(obj, method, arg_vals, object, env, span)
            }

            Expr::StructInit {
                name, fields, span, ..
            } => {
                // Resolve `Self` to the current impl type
                let resolved_name = if name == "Self" {
                    self.current_self_type
                        .clone()
                        .unwrap_or_else(|| name.clone())
                } else {
                    name.clone()
                };
                let mut field_map = HashMap::new();
                for (fname, fexpr) in fields {
                    let val = self.eval_expr(fexpr, env)?;
                    field_map.insert(fname.clone(), val);
                }
                // Validate fields against struct definition if registered
                if let Some(sdef) = self.struct_defs.get(&resolved_name) {
                    if let StructKind::Named(def_fields) = &sdef.kind {
                        for df in def_fields {
                            if !field_map.contains_key(&df.name) {
                                return Err(FerriError::Runtime {
                                    message: format!(
                                        "missing field `{}` in initializer of `{resolved_name}`",
                                        df.name
                                    ),
                                    line: span.line,
                                    column: span.column,
                                });
                            }
                        }
                    }
                }
                Ok(Value::Struct {
                    name: resolved_name,
                    fields: field_map,
                })
            }

            Expr::PathCall { path, args, span } => {
                let arg_vals: Vec<Value> = args
                    .iter()
                    .map(|a| self.eval_expr(a, env))
                    .collect::<Result<_, _>>()?;
                self.eval_path_call(path, &arg_vals, span, env)
            }

            Expr::Path { segments, span, .. } => self.eval_path(segments, span),

            Expr::SelfRef(span) => env.borrow().get("self").map_err(|_| FerriError::Runtime {
                message: "'self' not available in this context".into(),
                line: span.line,
                column: span.column,
            }),
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
            Err(FerriError::Return(val)) => Ok(*val),
            Err(e) => Err(e),
        }
    }

    // === Pattern matching ===

    fn pattern_matches(pattern: &Pattern, value: &Value) -> bool {
        match pattern {
            Pattern::Wildcard(_) => true,
            Pattern::Ident(_, _) => true, // Variable pattern always matches
            Pattern::Literal(expr) => match (expr, value) {
                (Expr::IntLiteral(n, _), Value::Integer(v)) => *n == *v,
                (Expr::FloatLiteral(n, _), Value::Float(v)) => *n == *v,
                (Expr::BoolLiteral(b, _), Value::Bool(v)) => *b == *v,
                (Expr::StringLiteral(s, _), Value::String(v)) => s == v,
                (Expr::CharLiteral(c, _), Value::Char(v)) => *c == *v,
                (
                    Expr::UnaryOp {
                        op: UnaryOp::Neg,
                        expr,
                        ..
                    },
                    Value::Integer(v),
                ) => {
                    if let Expr::IntLiteral(n, _) = expr.as_ref() {
                        -*n == *v
                    } else {
                        false
                    }
                }
                _ => false,
            },
            Pattern::EnumVariant {
                enum_name,
                variant,
                fields,
                ..
            } => {
                if let Value::EnumVariant {
                    enum_name: en,
                    variant: vn,
                    data,
                } = value
                {
                    en == enum_name
                        && vn == variant
                        && data.len() == fields.len()
                        && fields
                            .iter()
                            .zip(data.iter())
                            .all(|(pat, val)| Self::pattern_matches(pat, val))
                } else {
                    false
                }
            }
            Pattern::Struct { name, fields, .. } => {
                if let Value::Struct {
                    name: sn,
                    fields: sf,
                } = value
                {
                    sn == name
                        && fields.iter().all(|(fname, pat)| {
                            sf.get(fname).is_some_and(|v| Self::pattern_matches(pat, v))
                        })
                } else {
                    false
                }
            }
        }
    }

    // === Iteration ===

    fn value_to_iter(&self, value: &Value, span: Span) -> Result<Vec<Value>, FerriError> {
        match value {
            Value::Range(start, end) => Ok((*start..*end).map(Value::Integer).collect()),
            Value::Vec(v) => Ok(v.clone()),
            Value::String(s) => Ok(s.chars().map(Value::Char).collect()),
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
            "vec" => {
                let vals: Vec<Value> = args
                    .iter()
                    .map(|a| self.eval_expr(a, env))
                    .collect::<Result<_, _>>()?;
                Ok(Value::Vec(vals))
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

    // === Method dispatch ===

    fn call_method(
        &mut self,
        receiver: Value,
        method: &str,
        args: Vec<Value>,
        receiver_expr: &Expr,
        env: &Env,
        span: &Span,
    ) -> Result<Value, FerriError> {
        match &receiver {
            Value::Vec(_) => self.call_vec_method(receiver, method, args, receiver_expr, env, span),
            Value::String(_) => self.call_string_method(receiver, method, args, span),
            Value::Tuple(_) => Err(FerriError::Runtime {
                message: format!("no method `{method}` on tuple"),
                line: span.line,
                column: span.column,
            }),
            Value::Struct { name, .. }
            | Value::EnumVariant {
                enum_name: name, ..
            } => {
                let type_name = name.clone();
                self.call_user_method(receiver, &type_name, method, args, receiver_expr, env, span)
            }
            _ => Err(FerriError::Runtime {
                message: format!("no method `{method}` on type {}", receiver.type_name()),
                line: span.line,
                column: span.column,
            }),
        }
    }

    fn call_vec_method(
        &mut self,
        receiver: Value,
        method: &str,
        args: Vec<Value>,
        receiver_expr: &Expr,
        env: &Env,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let Value::Vec(v) = receiver else {
            unreachable!()
        };
        match method {
            "len" => Ok(Value::Integer(v.len() as i64)),
            "is_empty" => Ok(Value::Bool(v.is_empty())),
            "contains" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Vec::contains() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                Ok(Value::Bool(v.contains(&args[0])))
            }
            "push" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Vec::push() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let mut new_v = v;
                new_v.push(args.into_iter().next().unwrap());
                self.mutate_variable(receiver_expr, Value::Vec(new_v), env, span)?;
                Ok(Value::Unit)
            }
            "pop" => {
                let mut new_v = v;
                let popped = new_v.pop();
                self.mutate_variable(receiver_expr, Value::Vec(new_v), env, span)?;
                match popped {
                    Some(val) => Ok(val),
                    None => Ok(Value::Unit),
                }
            }
            "first" => Ok(v.first().cloned().unwrap_or(Value::Unit)),
            "last" => Ok(v.last().cloned().unwrap_or(Value::Unit)),
            "reverse" => {
                let mut new_v = v;
                new_v.reverse();
                self.mutate_variable(receiver_expr, Value::Vec(new_v), env, span)?;
                Ok(Value::Unit)
            }
            "join" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Vec::join() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let sep = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => format!("{other}"),
                };
                let s: Vec<String> = v.iter().map(|e| format!("{e}")).collect();
                Ok(Value::String(s.join(&sep)))
            }
            _ => Err(FerriError::Runtime {
                message: format!("no method `{method}` on Vec"),
                line: span.line,
                column: span.column,
            }),
        }
    }

    fn call_string_method(
        &mut self,
        receiver: Value,
        method: &str,
        args: Vec<Value>,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let Value::String(s) = receiver else {
            unreachable!()
        };
        match method {
            "len" => Ok(Value::Integer(s.len() as i64)),
            "is_empty" => Ok(Value::Bool(s.is_empty())),
            "contains" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("String::contains() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let needle = match &args[0] {
                    Value::String(s) => s.clone(),
                    Value::Char(c) => c.to_string(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!(
                                "String::contains() expects a string or char, got {}",
                                other.type_name()
                            ),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                Ok(Value::Bool(s.contains(&needle)))
            }
            "to_uppercase" => Ok(Value::String(s.to_uppercase())),
            "to_lowercase" => Ok(Value::String(s.to_lowercase())),
            "trim" => Ok(Value::String(s.trim().to_string())),
            "starts_with" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!(
                            "String::starts_with() takes 1 argument, got {}",
                            args.len()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
                let prefix = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected string, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                Ok(Value::Bool(s.starts_with(&prefix)))
            }
            "ends_with" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!(
                            "String::ends_with() takes 1 argument, got {}",
                            args.len()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
                let suffix = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected string, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                Ok(Value::Bool(s.ends_with(&suffix)))
            }
            "replace" => {
                if args.len() != 2 {
                    return Err(FerriError::Runtime {
                        message: format!("String::replace() takes 2 arguments, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let from = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected string, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                let to = match &args[1] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected string, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                Ok(Value::String(s.replace(&from, &to)))
            }
            "chars" => {
                let chars: Vec<Value> = s.chars().map(Value::Char).collect();
                Ok(Value::Vec(chars))
            }
            "split" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("String::split() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let delim = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected string, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                let parts: Vec<Value> = s
                    .split(&delim)
                    .map(|p| Value::String(p.to_string()))
                    .collect();
                Ok(Value::Vec(parts))
            }
            "repeat" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("String::repeat() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let n = match &args[0] {
                    Value::Integer(n) => *n as usize,
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected integer, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                Ok(Value::String(s.repeat(n)))
            }
            "push_str" => {
                // push_str is immutable in Ferrite — returns new string
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("String::push_str() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let suffix = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!("expected string, got {}", other.type_name()),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                let mut new_s = s;
                new_s.push_str(&suffix);
                Ok(Value::String(new_s))
            }
            _ => Err(FerriError::Runtime {
                message: format!("no method `{method}` on String"),
                line: span.line,
                column: span.column,
            }),
        }
    }

    /// Mutate the variable that the receiver expression refers to.
    fn bind_pattern(pattern: &Pattern, value: &Value, env: &Env) {
        match pattern {
            Pattern::Ident(name, _) => {
                env.borrow_mut().define(name.clone(), value.clone(), false);
            }
            Pattern::EnumVariant { fields, .. } => {
                if let Value::EnumVariant { data, .. } = value {
                    for (pat, val) in fields.iter().zip(data.iter()) {
                        Self::bind_pattern(pat, val, env);
                    }
                }
            }
            Pattern::Struct { fields, .. } => {
                if let Value::Struct {
                    fields: sfields, ..
                } = value
                {
                    for (fname, pat) in fields {
                        if let Some(val) = sfields.get(fname) {
                            Self::bind_pattern(pat, val, env);
                        }
                    }
                }
            }
            Pattern::Wildcard(_) | Pattern::Literal(_) => {}
        }
    }

    fn eval_path_call(
        &mut self,
        path: &[String],
        args: &[Value],
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        if path.len() == 2 {
            let type_name = &path[0];
            let method_name = &path[1];

            // Check for enum variant constructor: `Shape::Circle(5.0)`
            if let Some(edef) = self.enum_defs.get(type_name).cloned() {
                for variant in &edef.variants {
                    if variant.name == *method_name {
                        return Ok(Value::EnumVariant {
                            enum_name: type_name.clone(),
                            variant: method_name.clone(),
                            data: args.to_vec(),
                        });
                    }
                }
            }

            // Check for associated function in impl: `Point::new(1.0, 2.0)`
            if let Some(methods) = self.impl_methods.get(type_name).cloned() {
                for method_def in &methods {
                    if method_def.name == *method_name {
                        // Check it's an associated function (first param is not `self`)
                        let is_method = method_def.params.first().is_some_and(|p| p.name == "self");
                        if is_method {
                            return Err(FerriError::Runtime {
                                message: format!(
                                    "`{type_name}::{method_name}` is a method, not an associated function — call with `.{method_name}()` on an instance"
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }

                        let func_env = Environment::child(env);
                        // Bind parameters
                        for (param, arg) in method_def.params.iter().zip(args.iter()) {
                            func_env
                                .borrow_mut()
                                .define(param.name.clone(), arg.clone(), true);
                        }

                        let prev_self_type = self.current_self_type.take();
                        self.current_self_type = Some(type_name.clone());
                        let result = self.eval_block(&method_def.body, &func_env);
                        self.current_self_type = prev_self_type;

                        return match result {
                            Err(FerriError::Return(val)) => Ok(*val),
                            other => other,
                        };
                    }
                }
            }
        }

        Err(FerriError::Runtime {
            message: format!("undefined path `{}`", path.join("::")),
            line: span.line,
            column: span.column,
        })
    }

    fn eval_path(&self, segments: &[String], span: &Span) -> Result<Value, FerriError> {
        if segments.len() == 2 {
            let type_name = &segments[0];
            let variant_name = &segments[1];

            // Unit enum variant: `Color::Red`
            if let Some(edef) = self.enum_defs.get(type_name) {
                for variant in &edef.variants {
                    if variant.name == *variant_name {
                        if let EnumVariantKind::Unit = variant.kind {
                            return Ok(Value::EnumVariant {
                                enum_name: type_name.clone(),
                                variant: variant_name.clone(),
                                data: vec![],
                            });
                        }
                    }
                }
            }
        }

        Err(FerriError::Runtime {
            message: format!("undefined path `{}`", segments.join("::")),
            line: span.line,
            column: span.column,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn call_user_method(
        &mut self,
        receiver: Value,
        type_name: &str,
        method: &str,
        args: Vec<Value>,
        receiver_expr: &Expr,
        env: &Env,
        span: &Span,
    ) -> Result<Value, FerriError> {
        if let Some(methods) = self.impl_methods.get(type_name).cloned() {
            for method_def in &methods {
                if method_def.name == method {
                    let func_env = Environment::child(env);

                    // Bind `self`
                    func_env
                        .borrow_mut()
                        .define("self".to_string(), receiver.clone(), true);

                    // Bind remaining params (skip `self` in params)
                    let non_self_params: Vec<_> = method_def
                        .params
                        .iter()
                        .filter(|p| p.name != "self")
                        .collect();

                    for (param, arg) in non_self_params.iter().zip(args.iter()) {
                        func_env
                            .borrow_mut()
                            .define(param.name.clone(), arg.clone(), true);
                    }

                    let prev_self_type = self.current_self_type.take();
                    self.current_self_type = Some(type_name.to_string());
                    let result = self.eval_block(&method_def.body, &func_env);

                    // If method mutated `self`, propagate changes back
                    if let Ok(updated_self) = func_env.borrow().get("self") {
                        if updated_self != receiver {
                            let _ = self.mutate_variable(receiver_expr, updated_self, env, span);
                        }
                    }

                    self.current_self_type = prev_self_type;

                    return match result {
                        Err(FerriError::Return(val)) => Ok(*val),
                        other => other,
                    };
                }
            }
        }

        Err(FerriError::Runtime {
            message: format!("no method `{method}` found for type `{type_name}`"),
            line: span.line,
            column: span.column,
        })
    }

    fn mutate_variable(
        &mut self,
        expr: &Expr,
        new_val: Value,
        env: &Env,
        span: &Span,
    ) -> Result<(), FerriError> {
        match expr {
            Expr::Ident(name, _) => env.borrow_mut().set(name, new_val).map_err(|e| match e {
                FerriError::Runtime { message, .. } => FerriError::Runtime {
                    message,
                    line: span.line,
                    column: span.column,
                },
                other => other,
            }),
            _ => Err(FerriError::Runtime {
                message: "cannot mutate non-variable receiver".into(),
                line: span.line,
                column: span.column,
            }),
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
        Value::Vec(v) => {
            let items: Vec<String> = v.iter().map(debug_format).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Tuple(t) => {
            let items: Vec<String> = t.iter().map(debug_format).collect();
            if t.len() == 1 {
                format!("({},)", items[0])
            } else {
                format!("({})", items.join(", "))
            }
        }
        Value::Struct { name, fields } => {
            let mut sorted: Vec<_> = fields.iter().collect();
            sorted.sort_by_key(|(k, _)| (*k).clone());
            let items: Vec<String> = sorted
                .iter()
                .map(|(k, v)| format!("{k}: {}", debug_format(v)))
                .collect();
            format!("{name} {{ {} }}", items.join(", "))
        }
        Value::EnumVariant {
            enum_name,
            variant,
            data,
        } => {
            if data.is_empty() {
                format!("{enum_name}::{variant}")
            } else {
                let items: Vec<String> = data.iter().map(debug_format).collect();
                format!("{enum_name}::{variant}({})", items.join(", "))
            }
        }
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
        let output = run_and_capture(r#"fn main() { let x = 10; println!("{}", x); }"#);
        assert_eq!(output, vec!["10\n"]);
    }

    #[test]
    fn test_let_mut_and_assign() {
        let output = run_and_capture(r#"fn main() { let mut x = 1; x = 2; println!("{}", x); }"#);
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
        let output =
            run_and_capture(r#"fn main() { let x = 1; let x = "hello"; println!("{}", x); }"#);
        assert_eq!(output, vec!["hello\n"]);
    }

    // === Arithmetic ===

    #[test]
    fn test_integer_arithmetic() {
        let output = run_and_capture(r#"fn main() { println!("{}", 2 + 3 * 4); }"#);
        assert_eq!(output, vec!["14\n"]);
    }

    #[test]
    fn test_float_arithmetic() {
        let output = run_and_capture(r#"fn main() { println!("{}", 1.5 + 2.5); }"#);
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
        let output =
            run_and_capture(r#"fn main() { let s = "hello" + " " + "world"; println!("{}", s); }"#);
        assert_eq!(output, vec!["hello world\n"]);
    }

    #[test]
    fn test_negation() {
        let output = run_and_capture(r#"fn main() { let x = 5; println!("{}", -x); }"#);
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
        let output =
            run_and_capture(r#"fn main() { println!("{} {}", true && false, true || false); }"#);
        assert_eq!(output, vec!["false true\n"]);
    }

    #[test]
    fn test_logical_not() {
        let output = run_and_capture(r#"fn main() { println!("{}", !true); }"#);
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
        let result = run(r#"
fn foo(a: i64) -> i64 { a }
fn main() { foo(1, 2); }
"#);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expects 1 argument"));
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
        let output = run_and_capture(r#"fn main() { if true { println!("yes"); } }"#);
        assert_eq!(output, vec!["yes\n"]);
    }

    #[test]
    fn test_if_false() {
        let output = run_and_capture(r#"fn main() { if false { println!("yes"); } }"#);
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
        let output =
            run_and_capture(r#"fn main() { let x = { let y = 10; y + 1 }; println!("{}", x); }"#);
        assert_eq!(output, vec!["11\n"]);
    }

    // === Compound assignment ===

    #[test]
    fn test_compound_assignment() {
        let output =
            run_and_capture(r#"fn main() { let mut x = 10; x += 5; x -= 3; println!("{}", x); }"#);
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("no `main` function"));
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
        let result = run(r#"
fn main() {
    let x = 5;
    match x {
        1 => "one",
        2 => "two",
    };
}
"#);
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
                "1\n",
                "2\n",
                "Fizz\n",
                "4\n",
                "Buzz\n",
                "Fizz\n",
                "7\n",
                "8\n",
                "Fizz\n",
                "Buzz\n",
                "11\n",
                "Fizz\n",
                "13\n",
                "14\n",
                "FizzBuzz\n"
            ]
        );
    }

    // === Phase 6: Collections & Strings ===

    #[test]
    fn test_array_literal() {
        let output = run_and_capture("fn main() { let a = [1, 2, 3]; println!(\"{:?}\", a); }");
        assert_eq!(output, vec!["[1, 2, 3]\n"]);
    }

    #[test]
    fn test_empty_array() {
        let output = run_and_capture("fn main() { let a = []; println!(\"{:?}\", a); }");
        assert_eq!(output, vec!["[]\n"]);
    }

    #[test]
    fn test_vec_macro() {
        let output =
            run_and_capture("fn main() { let v = vec![10, 20, 30]; println!(\"{:?}\", v); }");
        assert_eq!(output, vec!["[10, 20, 30]\n"]);
    }

    #[test]
    fn test_vec_index() {
        let output =
            run_and_capture("fn main() { let v = vec![10, 20, 30]; println!(\"{}\", v[1]); }");
        assert_eq!(output, vec!["20\n"]);
    }

    #[test]
    fn test_vec_push() {
        let output = run_and_capture(
            r#"fn main() {
let mut v = vec![1, 2];
v.push(3);
println!("{:?}", v);
}"#,
        );
        assert_eq!(output, vec!["[1, 2, 3]\n"]);
    }

    #[test]
    fn test_vec_pop() {
        let output = run_and_capture(
            r#"fn main() {
let mut v = vec![1, 2, 3];
let x = v.pop();
println!("{} {:?}", x, v);
}"#,
        );
        assert_eq!(output, vec!["3 [1, 2]\n"]);
    }

    #[test]
    fn test_vec_len() {
        let output =
            run_and_capture("fn main() { let v = vec![1, 2, 3]; println!(\"{}\", v.len()); }");
        assert_eq!(output, vec!["3\n"]);
    }

    #[test]
    fn test_vec_is_empty() {
        let output = run_and_capture(
            r#"fn main() {
let a = [];
let b = vec![1];
println!("{} {}", a.is_empty(), b.is_empty());
}"#,
        );
        assert_eq!(output, vec!["true false\n"]);
    }

    #[test]
    fn test_vec_contains() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec![1, 2, 3];
println!("{} {}", v.contains(2), v.contains(5));
}"#,
        );
        assert_eq!(output, vec!["true false\n"]);
    }

    #[test]
    fn test_vec_index_assign() {
        let output = run_and_capture(
            r#"fn main() {
let mut v = vec![1, 2, 3];
v[1] = 99;
println!("{:?}", v);
}"#,
        );
        assert_eq!(output, vec!["[1, 99, 3]\n"]);
    }

    #[test]
    fn test_vec_iteration() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec![10, 20, 30];
let mut sum = 0;
for x in v {
    sum += x;
}
println!("{}", sum);
}"#,
        );
        assert_eq!(output, vec!["60\n"]);
    }

    #[test]
    fn test_vec_join() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec!["a", "b", "c"];
println!("{}", v.join(", "));
}"#,
        );
        assert_eq!(output, vec!["a, b, c\n"]);
    }

    #[test]
    fn test_tuple_literal() {
        let output = run_and_capture("fn main() { let t = (1, 2, 3); println!(\"{:?}\", t); }");
        assert_eq!(output, vec!["(1, 2, 3)\n"]);
    }

    #[test]
    fn test_tuple_index() {
        let output = run_and_capture(
            r#"fn main() {
let t = (10, "hello", true);
println!("{} {} {}", t.0, t.1, t.2);
}"#,
        );
        assert_eq!(output, vec!["10 hello true\n"]);
    }

    #[test]
    fn test_empty_tuple() {
        let output = run_and_capture("fn main() { let t = (); println!(\"{:?}\", t); }");
        assert_eq!(output, vec!["()\n"]);
    }

    #[test]
    fn test_single_element_tuple() {
        let output = run_and_capture("fn main() { let t = (42,); println!(\"{:?}\", t); }");
        assert_eq!(output, vec!["(42,)\n"]);
    }

    #[test]
    fn test_string_len() {
        let output = run_and_capture(r#"fn main() { let s = "hello"; println!("{}", s.len()); }"#);
        assert_eq!(output, vec!["5\n"]);
    }

    #[test]
    fn test_string_contains() {
        let output = run_and_capture(
            r#"fn main() {
let s = "hello world";
println!("{} {}", s.contains("world"), s.contains("xyz"));
}"#,
        );
        assert_eq!(output, vec!["true false\n"]);
    }

    #[test]
    fn test_string_to_uppercase() {
        let output =
            run_and_capture(r#"fn main() { let s = "hello"; println!("{}", s.to_uppercase()); }"#);
        assert_eq!(output, vec!["HELLO\n"]);
    }

    #[test]
    fn test_string_to_lowercase() {
        let output =
            run_and_capture(r#"fn main() { let s = "HELLO"; println!("{}", s.to_lowercase()); }"#);
        assert_eq!(output, vec!["hello\n"]);
    }

    #[test]
    fn test_string_trim() {
        let output =
            run_and_capture(r#"fn main() { let s = "  hello  "; println!(">{}<", s.trim()); }"#);
        assert_eq!(output, vec![">hello<\n"]);
    }

    #[test]
    fn test_string_starts_with() {
        let output = run_and_capture(
            r#"fn main() {
let s = "hello world";
println!("{} {}", s.starts_with("hello"), s.starts_with("world"));
}"#,
        );
        assert_eq!(output, vec!["true false\n"]);
    }

    #[test]
    fn test_string_ends_with() {
        let output = run_and_capture(
            r#"fn main() {
let s = "hello world";
println!("{} {}", s.ends_with("world"), s.ends_with("hello"));
}"#,
        );
        assert_eq!(output, vec!["true false\n"]);
    }

    #[test]
    fn test_string_replace() {
        let output = run_and_capture(
            r#"fn main() {
let s = "hello world";
println!("{}", s.replace("world", "ferrite"));
}"#,
        );
        assert_eq!(output, vec!["hello ferrite\n"]);
    }

    #[test]
    fn test_string_split() {
        let output = run_and_capture(
            r#"fn main() {
let s = "a,b,c";
let parts = s.split(",");
println!("{:?}", parts);
}"#,
        );
        assert_eq!(output, vec!["[\"a\", \"b\", \"c\"]\n"]);
    }

    #[test]
    fn test_string_chars() {
        let output = run_and_capture(
            r#"fn main() {
let s = "hi";
let chars = s.chars();
println!("{:?}", chars);
}"#,
        );
        assert_eq!(output, vec!["['h', 'i']\n"]);
    }

    #[test]
    fn test_string_repeat() {
        let output = run_and_capture(r#"fn main() { println!("{}", "ab".repeat(3)); }"#);
        assert_eq!(output, vec!["ababab\n"]);
    }

    #[test]
    fn test_string_iteration() {
        let output = run_and_capture(
            r#"fn main() {
for c in "abc" {
    println!("{}", c);
}
}"#,
        );
        assert_eq!(output, vec!["a\n", "b\n", "c\n"]);
    }

    #[test]
    fn test_vec_first_last() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec![10, 20, 30];
println!("{} {}", v.first(), v.last());
}"#,
        );
        assert_eq!(output, vec!["10 30\n"]);
    }

    #[test]
    fn test_vec_reverse() {
        let output = run_and_capture(
            r#"fn main() {
let mut v = vec![1, 2, 3];
v.reverse();
println!("{:?}", v);
}"#,
        );
        assert_eq!(output, vec!["[3, 2, 1]\n"]);
    }

    #[test]
    fn test_nested_vec() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec![vec![1, 2], vec![3, 4]];
println!("{}", v[0][1]);
println!("{:?}", v);
}"#,
        );
        assert_eq!(output, vec!["2\n", "[[1, 2], [3, 4]]\n"]);
    }

    #[test]
    fn test_debug_format_collections() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec!["hello", "world"];
println!("{:?}", v);
let t = (1, "two", true);
println!("{:?}", t);
}"#,
        );
        assert_eq!(
            output,
            vec!["[\"hello\", \"world\"]\n", "(1, \"two\", true)\n"]
        );
    }

    #[test]
    fn test_index_out_of_bounds() {
        let result = run("fn main() { let v = vec![1, 2]; let x = v[5]; }");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("index out of bounds"));
    }

    #[test]
    fn test_tuple_index_out_of_bounds() {
        let result = run("fn main() { let t = (1, 2); let x = t.5; }");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("index out of bounds"));
    }

    // === Phase 7: Structs ===

    #[test]
    fn test_struct_basic() {
        let out = run_and_capture(
            r#"
struct Point {
    x: f64,
    y: f64,
}

fn main() {
    let p = Point { x: 1.0, y: 2.0 };
    println!("{} {}", p.x, p.y);
}
"#,
        );
        assert_eq!(out, vec!["1.0 2.0\n"]);
    }

    #[test]
    fn test_struct_field_assignment() {
        let out = run_and_capture(
            r#"
struct Point {
    x: f64,
    y: f64,
}

fn main() {
    let mut p = Point { x: 1.0, y: 2.0 };
    p.x = 10.0;
    println!("{} {}", p.x, p.y);
}
"#,
        );
        assert_eq!(out, vec!["10.0 2.0\n"]);
    }

    #[test]
    fn test_struct_with_impl() {
        let out = run_and_capture(
            r#"
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }

    fn display(&self) {
        println!("({}, {})", self.x, self.y);
    }
}

fn main() {
    let p = Point::new(3.0, 4.0);
    p.display();
}
"#,
        );
        assert_eq!(out, vec!["(3.0, 4.0)\n"]);
    }

    #[test]
    fn test_struct_method_with_args() {
        let out = run_and_capture(
            r#"
struct Rect {
    w: f64,
    h: f64,
}

impl Rect {
    fn area(&self) -> f64 {
        self.w * self.h
    }
}

fn main() {
    let r = Rect { w: 5.0, h: 3.0 };
    println!("{}", r.area());
}
"#,
        );
        assert_eq!(out, vec!["15.0\n"]);
    }

    #[test]
    fn test_struct_debug_format() {
        let out = run_and_capture(
            r#"
struct Point {
    x: f64,
    y: f64,
}

fn main() {
    let p = Point { x: 1.0, y: 2.0 };
    println!("{:?}", p);
}
"#,
        );
        assert_eq!(out, vec!["Point { x: 1.0, y: 2.0 }\n"]);
    }

    // === Phase 7: Enums ===

    #[test]
    fn test_enum_unit_variant() {
        let out = run_and_capture(
            r#"
enum Color {
    Red,
    Green,
    Blue,
}

fn main() {
    let c = Color::Red;
    println!("{}", c);
}
"#,
        );
        assert_eq!(out, vec!["Color::Red\n"]);
    }

    #[test]
    fn test_enum_tuple_variant() {
        let out = run_and_capture(
            r#"
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
}

fn main() {
    let s = Shape::Circle(5.0);
    println!("{}", s);
}
"#,
        );
        assert_eq!(out, vec!["Shape::Circle(5.0)\n"]);
    }

    #[test]
    fn test_enum_match() {
        let out = run_and_capture(
            r#"
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
}

impl Shape {
    fn area(&self) -> f64 {
        match self {
            Shape::Circle(r) => 3.14159 * r * r,
            Shape::Rectangle(w, h) => w * h,
        }
    }
}

fn main() {
    let s = Shape::Circle(5.0);
    println!("{}", s.area());
    let r = Shape::Rectangle(4.0, 3.0);
    println!("{}", r.area());
}
"#,
        );
        assert_eq!(out, vec!["78.53975\n", "12.0\n"]);
    }

    #[test]
    fn test_enum_match_unit_variant() {
        let out = run_and_capture(
            r#"
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

fn describe(d: Direction) -> String {
    match d {
        Direction::Up => "going up",
        Direction::Down => "going down",
        _ => "sideways",
    }
}

fn main() {
    println!("{}", describe(Direction::Up));
    println!("{}", describe(Direction::Left));
}
"#,
        );
        assert_eq!(out, vec!["going up\n", "sideways\n"]);
    }

    #[test]
    fn test_enum_debug_format() {
        let out = run_and_capture(
            r#"
enum Shape {
    Circle(f64),
    Point,
}

fn main() {
    let s = Shape::Circle(2.5);
    let p = Shape::Point;
    println!("{:?}", s);
    println!("{:?}", p);
}
"#,
        );
        assert_eq!(out, vec!["Shape::Circle(2.5)\n", "Shape::Point\n"]);
    }

    // === Phase 7: Full example ===

    #[test]
    fn test_point_distance() {
        let out = run_and_capture(
            r#"
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }
}

fn main() {
    let p1 = Point::new(0.0, 0.0);
    let p2 = Point::new(3.0, 4.0);
    let dx = p1.x - p2.x;
    let dy = p1.y - p2.y;
    let dist_sq = dx * dx + dy * dy;
    println!("{}", dist_sq);
}
"#,
        );
        assert_eq!(out, vec!["25.0\n"]);
    }

    #[test]
    fn test_struct_self_type_resolution() {
        let out = run_and_capture(
            r#"
struct Counter {
    count: i64,
}

impl Counter {
    fn new() -> Self {
        Self { count: 0 }
    }

    fn value(&self) -> i64 {
        self.count
    }
}

fn main() {
    let c = Counter::new();
    println!("{}", c.value());
}
"#,
        );
        assert_eq!(out, vec!["0\n"]);
    }

    #[test]
    fn test_struct_shorthand_init() {
        let out = run_and_capture(
            r#"
struct Point {
    x: f64,
    y: f64,
}

fn main() {
    let x = 1.0;
    let y = 2.0;
    let p = Point { x, y };
    println!("{} {}", p.x, p.y);
}
"#,
        );
        assert_eq!(out, vec!["1.0 2.0\n"]);
    }
}
