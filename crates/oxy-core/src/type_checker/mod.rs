//! Semantic type checker for Oxy.
//!
//! Runs after parsing and before execution. Validates type annotations
//! on `let` bindings, function params, and return types.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::*;
use crate::errors::FerriError;

/// Internal representation of an Oxy type.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeInfo {
    I64,
    F64,
    Bool,
    String,
    Char,
    Unit,
    Vec,
    HashMap,
    Option,
    Result,
    UserStruct(String),
    Unknown,
}

impl TypeInfo {
    pub fn name(&self) -> &str {
        match self {
            TypeInfo::I64 => "i64",
            TypeInfo::F64 => "f64",
            TypeInfo::Bool => "bool",
            TypeInfo::String => "String",
            TypeInfo::Char => "char",
            TypeInfo::Unit => "()",
            TypeInfo::Vec => "Vec",
            TypeInfo::HashMap => "HashMap",
            TypeInfo::Option => "Option",
            TypeInfo::Result => "Result",
            TypeInfo::UserStruct(name) => name.as_str(),
            TypeInfo::Unknown => "?",
        }
    }

    pub fn from_annotation(ann: &Option<TypeAnnotation>) -> Result<TypeInfo, FerriError> {
        let ann = match ann {
            Some(a) => a,
            None => return Ok(TypeInfo::Unknown),
        };
        Ok(Self::from_name(&ann.name))
    }

    pub fn from_name(name: &str) -> TypeInfo {
        match name {
            "i64" | "isize" | "u64" | "usize" | "i32" | "u32" | "i8" | "u8" => TypeInfo::I64,
            "f64" | "f32" => TypeInfo::F64,
            "bool" => TypeInfo::Bool,
            "String" | "str" => TypeInfo::String,
            "char" => TypeInfo::Char,
            "()" | "Unit" => TypeInfo::Unit,
            "Vec" => TypeInfo::Vec,
            "HashMap" => TypeInfo::HashMap,
            "Option" => TypeInfo::Option,
            "Result" => TypeInfo::Result,
            "_" => TypeInfo::Unknown,
            n => TypeInfo::UserStruct(n.to_string()),
        }
    }

    pub fn accepts(&self, other: &TypeInfo) -> bool {
        if *self == TypeInfo::Unknown || *other == TypeInfo::Unknown {
            return true;
        }
        if self == other {
            return true;
        }
        matches!(
            (self, other),
            (TypeInfo::I64, TypeInfo::F64) | (TypeInfo::F64, TypeInfo::I64)
        )
    }
}

/// Scoped type environment.
#[derive(Clone)]
struct TypeEnv {
    bindings: HashMap<String, TypeInfo>,
    parent: Option<Rc<RefCell<TypeEnv>>>,
}

impl TypeEnv {
    fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            bindings: HashMap::new(),
            parent: None,
        }))
    }

    fn child(parent: &Rc<RefCell<Self>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            bindings: HashMap::new(),
            parent: Some(Rc::clone(parent)),
        }))
    }

    fn define(&mut self, name: &str, ty: TypeInfo) {
        self.bindings.insert(name.to_string(), ty);
    }

    fn get(&self, name: &str) -> Option<TypeInfo> {
        if let Some(ty) = self.bindings.get(name) {
            return Some(ty.clone());
        }
        if let Some(ref parent) = self.parent {
            return parent.borrow().get(name);
        }
        None
    }
}

/// The type checker.
pub struct TypeChecker {
    env: Rc<RefCell<TypeEnv>>,
    struct_defs: HashMap<String, StructDef>,
    type_aliases: HashMap<String, TypeAnnotation>,
    fn_return_types: HashMap<String, TypeInfo>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            env: TypeEnv::new(),
            struct_defs: HashMap::new(),
            type_aliases: HashMap::new(),
            fn_return_types: HashMap::new(),
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    /// Resolve a type name through type aliases (e.g. `Meters` → `f64`).
    fn resolve_type(&self, name: &str) -> TypeInfo {
        if let Some(alias) = self.type_aliases.get(name) {
            return TypeInfo::from_name(&alias.name);
        }
        TypeInfo::from_name(name)
    }

    pub fn check_program(&mut self, program: &Program) -> Result<(), FerriError> {
        // First pass: collect struct definitions and type aliases
        for item in &program.items {
            match item {
                Item::Struct(s) => {
                    self.struct_defs.insert(s.name.clone(), s.clone());
                }
                Item::TypeAlias { name, target, .. } => {
                    self.type_aliases.insert(name.clone(), target.clone());
                }
                _ => {}
            }
        }

        // Second pass: register function return types
        for item in &program.items {
            if let Item::Function(f) = item {
                let ret_ty = if let Some(ref ann) = f.return_type {
                    self.resolve_type(&ann.name)
                } else {
                    TypeInfo::Unit
                };
                self.fn_return_types.insert(f.name.clone(), ret_ty);
            }
        }

        // Third pass: check each item
        for item in &program.items {
            self.check_item(item)?;
        }

        Ok(())
    }

    fn check_item(&mut self, item: &Item) -> Result<(), FerriError> {
        match item {
            Item::Function(f) => self.check_function(f),
            Item::Const {
                name,
                value,
                type_ann,
                span,
                ..
            } => {
                let declared = if let Some(ann) = type_ann {
                    self.resolve_type(&ann.name)
                } else {
                    TypeInfo::Unknown
                };
                let inferred = self.infer_expr(value)?;
                if !declared.accepts(&inferred) {
                    return Err(FerriError::TypeError {
                        message: format!(
                            "type mismatch: const `{name}` declared as `{}`, but value has type `{}`",
                            declared.name(), inferred.name()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn check_function(&mut self, f: &FnDef) -> Result<(), FerriError> {
        let ret_ty = if let Some(ref ann) = f.return_type {
            self.resolve_type(&ann.name)
        } else {
            TypeInfo::Unit
        };
        self.fn_return_types.insert(f.name.clone(), ret_ty.clone());

        let fn_env = TypeEnv::child(&self.env);
        for param in &f.params {
            let param_ty = self.resolve_type(&param.type_ann.name);
            fn_env.borrow_mut().define(&param.name, param_ty);
        }

        let saved_env = self.env.clone();
        self.env = fn_env;

        for stmt in &f.body.stmts {
            self.check_stmt(stmt, &ret_ty)?;
        }

        self.env = saved_env;
        Ok(())
    }

    fn check_stmt(&mut self, stmt: &Stmt, fn_ret: &TypeInfo) -> Result<(), FerriError> {
        match stmt {
            Stmt::Let {
                name,
                type_ann,
                value,
                span,
                ..
            } => {
                let declared = if let Some(ann) = type_ann {
                    self.resolve_type(&ann.name)
                } else {
                    TypeInfo::Unknown
                };
                let inferred = if let Some(expr) = value {
                    self.infer_expr(expr)?
                } else {
                    TypeInfo::Unit
                };
                if !declared.accepts(&inferred) {
                    return Err(FerriError::TypeError {
                        message: format!(
                            "type mismatch: variable `{name}` declared as `{}`, but value has type `{}`",
                            declared.name(), inferred.name()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
                let stored_ty = if declared != TypeInfo::Unknown {
                    declared
                } else {
                    inferred
                };
                self.env.borrow_mut().define(name, stored_ty);
                Ok(())
            }
            Stmt::Expr {
                expr,
                has_semicolon,
            } => {
                // Tail expression without semicolon is an implicit return — check type.
                // Skip check if inferred as Unit (control-flow expressions with explicit
                // returns, e.g. `if x > 0 { return x; }`).
                if !has_semicolon && *fn_ret != TypeInfo::Unknown {
                    let inferred = self.infer_expr(expr)?;
                    if inferred != TypeInfo::Unit && !fn_ret.accepts(&inferred) {
                        let span = expr.span();
                        return Err(FerriError::TypeError {
                            message: format!(
                                "type mismatch: function returns `{}`, but tail expression has type `{}`",
                                fn_ret.name(), inferred.name()
                            ),
                            line: span.line,
                            column: span.column,
                        });
                    }
                }
                // Check if the inner expression is an if/if-let (they only exist as Expr)
                if let Expr::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } = expr
                {
                    self.infer_expr(condition)?;
                    let block_env = TypeEnv::child(&self.env);
                    let saved = self.env.clone();
                    self.env = block_env;
                    for s in &then_block.stmts {
                        self.check_stmt(s, fn_ret)?;
                    }
                    self.env = saved;
                    if let Some(else_expr) = else_block {
                        self.infer_expr(else_expr)?;
                    }
                } else if let Expr::IfLet {
                    expr: inner,
                    then_block,
                    else_block,
                    ..
                } = expr
                {
                    let _ = self.infer_expr(inner)?;
                    let block_env = TypeEnv::child(&self.env);
                    let saved = self.env.clone();
                    self.env = block_env;
                    for s in &then_block.stmts {
                        self.check_stmt(s, fn_ret)?;
                    }
                    self.env = saved;
                    if let Some(else_expr) = else_block {
                        self.infer_expr(else_expr)?;
                    }
                } else {
                    self.infer_expr(expr)?;
                }
                Ok(())
            }
            Stmt::Return { value, span } => {
                let inferred = if let Some(expr) = value {
                    self.infer_expr(expr)?
                } else {
                    TypeInfo::Unit
                };
                if !fn_ret.accepts(&inferred) {
                    return Err(FerriError::TypeError {
                        message: format!(
                            "type mismatch: function returns `{}`, but return expression has type `{}`",
                            fn_ret.name(), inferred.name()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
                Ok(())
            }
            Stmt::While {
                condition, body, ..
            } => {
                self.infer_expr(condition)?;
                self.check_block(body, fn_ret)?;
                Ok(())
            }
            Stmt::Loop { body, .. } => {
                self.check_block(body, fn_ret)?;
                Ok(())
            }
            Stmt::For {
                name,
                iterable,
                body,
                ..
            } => {
                let _ = self.infer_expr(iterable)?;
                let body_env = TypeEnv::child(&self.env);
                body_env.borrow_mut().define(name, TypeInfo::Unknown);
                let saved = self.env.clone();
                self.env = body_env;
                self.check_block(body, fn_ret)?;
                self.env = saved;
                Ok(())
            }
            Stmt::WhileLet {
                expr: inner, body, ..
            } => {
                let _ = self.infer_expr(inner)?;
                self.check_block(body, fn_ret)?;
                Ok(())
            }
            Stmt::ForDestructure {
                names,
                iterable,
                body,
                ..
            } => {
                let _ = self.infer_expr(iterable)?;
                let body_env = TypeEnv::child(&self.env);
                for name in names {
                    body_env.borrow_mut().define(name, TypeInfo::Unknown);
                }
                let saved = self.env.clone();
                self.env = body_env;
                self.check_block(body, fn_ret)?;
                self.env = saved;
                Ok(())
            }
            Stmt::LetPattern { value, .. } => {
                self.infer_expr(value)?;
                Ok(())
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => Ok(()),
        }
    }

    fn check_block(&mut self, block: &Block, fn_ret: &TypeInfo) -> Result<(), FerriError> {
        let block_env = TypeEnv::child(&self.env);
        let saved = self.env.clone();
        self.env = block_env;
        for stmt in &block.stmts {
            self.check_stmt(stmt, fn_ret)?;
        }
        self.env = saved;
        Ok(())
    }

    #[allow(dead_code)]
    fn check_expr_type(&mut self, expr: &Expr, expected: &TypeInfo) -> Result<(), FerriError> {
        let inferred = self.infer_expr(expr)?;
        if !expected.accepts(&inferred) {
            let span = expr.span();
            return Err(FerriError::TypeError {
                message: format!(
                    "type mismatch: expected `{}`, got `{}`",
                    expected.name(),
                    inferred.name()
                ),
                line: span.line,
                column: span.column,
            });
        }
        Ok(())
    }

    fn infer_expr(&mut self, expr: &Expr) -> Result<TypeInfo, FerriError> {
        match expr {
            Expr::IntLiteral(..) => Ok(TypeInfo::I64),
            Expr::FloatLiteral(..) => Ok(TypeInfo::F64),
            Expr::BoolLiteral(..) => Ok(TypeInfo::Bool),
            Expr::StringLiteral(..) => Ok(TypeInfo::String),
            Expr::CharLiteral(..) => Ok(TypeInfo::Char),

            Expr::Ident(name, _span) => {
                if let Some(ty) = self.env.borrow().get(name) {
                    return Ok(ty);
                }
                if let Some(ret) = self.fn_return_types.get(name) {
                    return Ok(ret.clone());
                }
                Ok(TypeInfo::Unknown)
            }

            Expr::BinaryOp { left, right, .. } => {
                let lt = self.infer_expr(left)?;
                let rt = self.infer_expr(right)?;
                // Numeric ops coerce to I64 by default
                if lt == TypeInfo::F64 || rt == TypeInfo::F64 {
                    Ok(TypeInfo::F64)
                } else {
                    Ok(TypeInfo::I64)
                }
            }

            Expr::UnaryOp { expr: inner, .. } => self.infer_expr(inner),

            Expr::Call { callee, args, .. } => {
                for arg in args {
                    self.infer_expr(arg)?;
                }
                if let Expr::Ident(name, _) = callee.as_ref() {
                    if let Some(ret) = self.fn_return_types.get(name) {
                        return Ok(ret.clone());
                    }
                    // Built-in constructors
                    match name.as_str() {
                        "Some" => return Ok(TypeInfo::Option),
                        "Ok" | "Err" => return Ok(TypeInfo::Result),
                        _ => {}
                    }
                }
                Ok(TypeInfo::Unknown)
            }

            Expr::Block(block) => {
                let mut last_ty = TypeInfo::Unit;
                for (i, stmt) in block.stmts.iter().enumerate() {
                    let is_last = i == block.stmts.len() - 1;
                    self.check_stmt(stmt, &TypeInfo::Unknown)?;
                    if is_last {
                        if let Stmt::Expr {
                            expr,
                            has_semicolon,
                        } = stmt
                        {
                            if !has_semicolon {
                                last_ty = self.infer_expr(expr)?;
                            }
                        }
                    }
                }
                Ok(last_ty)
            }

            Expr::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.infer_expr(condition)?;
                let block_env = TypeEnv::child(&self.env);
                let saved = self.env.clone();
                self.env = block_env;
                let mut result = TypeInfo::Unit;
                for stmt in &then_block.stmts {
                    if let Stmt::Expr {
                        expr,
                        has_semicolon,
                    } = stmt
                    {
                        if !has_semicolon {
                            result = self.infer_expr(expr)?;
                        }
                    }
                }
                self.env = saved;
                if let Some(else_expr) = else_block {
                    let else_ty = self.infer_expr(else_expr)?;
                    if result == TypeInfo::Unit {
                        result = else_ty;
                    }
                }
                Ok(result)
            }

            Expr::IfLet {
                expr: inner,
                then_block,
                else_block,
                ..
            } => {
                let _ = self.infer_expr(inner)?;
                let block_env = TypeEnv::child(&self.env);
                let saved = self.env.clone();
                self.env = block_env;
                let mut result = TypeInfo::Unit;
                for stmt in &then_block.stmts {
                    if let Stmt::Expr {
                        expr,
                        has_semicolon,
                    } = stmt
                    {
                        if !has_semicolon {
                            result = self.infer_expr(expr)?;
                        }
                    }
                }
                self.env = saved;
                if let Some(else_expr) = else_block {
                    let else_ty = self.infer_expr(else_expr)?;
                    if result == TypeInfo::Unit {
                        result = else_ty;
                    }
                }
                Ok(result)
            }

            Expr::Grouped(inner, _) => self.infer_expr(inner),

            Expr::Array { elements, .. } => {
                for e in elements {
                    self.infer_expr(e)?;
                }
                Ok(TypeInfo::Vec)
            }

            Expr::Tuple { elements, .. } => {
                for e in elements {
                    self.infer_expr(e)?;
                }
                Ok(TypeInfo::Unknown)
            }

            Expr::Assign { target, value, .. } => {
                let vt = self.infer_expr(value)?;
                if let Expr::Ident(name, _) = target.as_ref() {
                    self.env.borrow_mut().define(name, vt);
                }
                Ok(TypeInfo::Unit)
            }

            Expr::Match {
                expr: matched,
                arms,
                span: _span,
            } => {
                let _ = self.infer_expr(matched)?;
                let mut result = TypeInfo::Unit;
                for arm in arms {
                    let arm_env = TypeEnv::child(&self.env);
                    let saved = self.env.clone();
                    self.env = arm_env;
                    let arm_ty = self.infer_expr(&arm.body)?;
                    self.env = saved;
                    if result == TypeInfo::Unit {
                        result = arm_ty;
                    }
                }
                Ok(result)
            }

            Expr::PathCall { args, .. } => {
                for arg in args {
                    self.infer_expr(arg)?;
                }
                Ok(TypeInfo::Unknown)
            }

            Expr::MethodCall { object, args, .. } => {
                let _ = self.infer_expr(object)?;
                for arg in args {
                    self.infer_expr(arg)?;
                }
                Ok(TypeInfo::Unknown)
            }

            Expr::FieldAccess { object, .. } => {
                let _ = self.infer_expr(object)?;
                Ok(TypeInfo::Unknown)
            }

            Expr::Index { object, index, .. } => {
                let _ = self.infer_expr(object)?;
                let _ = self.infer_expr(index)?;
                Ok(TypeInfo::Unknown)
            }

            Expr::Range { .. } => Ok(TypeInfo::I64),

            Expr::StructInit { name, fields, .. } => {
                for (_, f_expr) in fields {
                    self.infer_expr(f_expr)?;
                }
                Ok(TypeInfo::UserStruct(name.clone()))
            }

            Expr::Try { expr: inner, .. } => {
                let _ = self.infer_expr(inner)?;
                Ok(TypeInfo::Unknown)
            }

            Expr::Closure { .. } => Ok(TypeInfo::Unknown),
            Expr::Await { expr: inner, .. } => {
                let _ = self.infer_expr(inner)?;
                Ok(TypeInfo::Unknown)
            }
            Expr::FString { .. } => Ok(TypeInfo::String),
            Expr::MacroCall { .. } => Ok(TypeInfo::Unknown),
            Expr::Path { .. } => Ok(TypeInfo::Unknown),
            Expr::SelfRef { .. } => Ok(TypeInfo::Unknown),
            Expr::As { expr, type_name, .. } => {
                let _ = self.infer_expr(expr)?;
                match type_name.as_str() {
                    "i64" | "usize" => Ok(TypeInfo::I64),
                    "f64" => Ok(TypeInfo::F64),
                    "char" => Ok(TypeInfo::Char),
                    "bool" => Ok(TypeInfo::Bool),
                    "String" => Ok(TypeInfo::String),
                    _ => Ok(TypeInfo::Unknown),
                }
            }
            Expr::CompoundAssign {
                target: _, value, ..
            } => {
                let _ = self.infer_expr(value)?;
                Ok(TypeInfo::Unit)
            }
        }
    }
}

#[cfg(test)]
mod tests;
