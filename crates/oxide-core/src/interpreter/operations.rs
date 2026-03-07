//! Binary and unary operation evaluation.
//!
//! Handles arithmetic, comparison, logical, bitwise operators, and
//! operator overloading via trait impls (e.g. `impl Add for Point`).

use crate::ast::*;
use crate::env::Environment;
use crate::errors::FerriError;
use crate::types::Value;

use super::Interpreter;

impl Interpreter {
    /// Evaluate a binary operation (e.g. `a + b`, `x == y`).
    ///
    /// Tries built-in operations first (int/float arithmetic, string concat,
    /// comparisons, logical/bitwise ops), then falls back to operator
    /// overloading by looking up trait impls (Add, Sub, Mul, Div).
    pub(crate) fn eval_binary_op(
        &mut self,
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

            _ => {
                // WHY: Built-in operators only cover primitive types. For user-defined types
                // (structs, enums) we fall through to trait-based dispatch so that `a + b` can
                // resolve to `Add::add(a, b)`. This two-phase approach keeps the fast path
                // (primitives) zero-cost while still allowing extensibility via trait impls,
                // mirroring Rust's own operator overloading model.
                let trait_name = match op {
                    BinOp::Add => Some("Add"),
                    BinOp::Sub => Some("Sub"),
                    BinOp::Mul => Some("Mul"),
                    BinOp::Div => Some("Div"),
                    BinOp::Mod => Some("Rem"),
                    _ => None,
                };
                let method_name = match op {
                    BinOp::Add => Some("add"),
                    BinOp::Sub => Some("sub"),
                    BinOp::Mul => Some("mul"),
                    BinOp::Div => Some("div"),
                    BinOp::Mod => Some("rem"),
                    _ => None,
                };
                let type_name = match left {
                    Value::Struct { name, .. } => Some(name.clone()),
                    Value::EnumVariant { enum_name, .. } => Some(enum_name.clone()),
                    _ => None,
                };
                if let (Some(tn), Some(trait_n), Some(method_n)) =
                    (&type_name, trait_name, method_name)
                {
                    if let Some(method_def) = self.find_trait_method(tn, method_n) {
                        // Check method is from the right trait
                        let key = (tn.clone(), trait_n.to_string());
                        if self.trait_impls.contains_key(&key) {
                            let func_env = Environment::child(&self.env);
                            func_env
                                .borrow_mut()
                                .define("self".to_string(), left.clone(), true);
                            // Bind `other`/`rhs` param
                            let non_self_params: Vec<_> = method_def
                                .params
                                .iter()
                                .filter(|p| p.name != "self")
                                .collect();
                            if let Some(param) = non_self_params.first() {
                                func_env.borrow_mut().define(
                                    param.name.clone(),
                                    right.clone(),
                                    true,
                                );
                            }
                            let prev = self.current_self_type.take();
                            self.current_self_type = Some(tn.clone());
                            let result = self.eval_block(&method_def.body, &func_env);
                            self.current_self_type = prev;
                            return match result {
                                Err(FerriError::Return(val)) => Ok(*val),
                                other => other,
                            };
                        }
                    }
                }
                Err(FerriError::Runtime {
                    message: format!(
                        "unsupported operation: {} {op} {}",
                        left.type_name(),
                        right.type_name()
                    ),
                    line,
                    column: col,
                })
            }
        }
    }

    /// Evaluate a unary operation (e.g. `-x`, `!flag`, `&val`, `*val`).
    ///
    /// Note: `&` and `*` are no-ops — Oxide has no borrow checker,
    /// so references and dereferences just pass the value through.
    pub(crate) fn eval_unary_op(
        &mut self,
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
            // Neg trait overloading for user types
            (UnaryOp::Neg, Value::Struct { name, .. })
            | (
                UnaryOp::Neg,
                Value::EnumVariant {
                    enum_name: name, ..
                },
            ) => {
                if let Some(method_def) = self.find_trait_method(name, "neg") {
                    let key = (name.clone(), "Neg".to_string());
                    if self.trait_impls.contains_key(&key) {
                        let func_env = Environment::child(&self.env);
                        func_env
                            .borrow_mut()
                            .define("self".to_string(), val.clone(), true);
                        let prev = self.current_self_type.take();
                        self.current_self_type = Some(name.clone());
                        let result = self.eval_block(&method_def.body, &func_env);
                        self.current_self_type = prev;
                        return match result {
                            Err(FerriError::Return(v)) => Ok(*v),
                            other => other,
                        };
                    }
                }
                Err(FerriError::Runtime {
                    message: format!("unsupported unary operation: {op}{}", val.type_name()),
                    line,
                    column: col,
                })
            }
            _ => Err(FerriError::Runtime {
                message: format!("unsupported unary operation: {op}{}", val.type_name()),
                line,
                column: col,
            }),
        }
    }
}
