//! Semantic type checker for Oxy.
//!
//! Runs after parsing and before execution. Validates type annotations
//! on `let` bindings, function params, and return types.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::*;
use crate::errors::FerriError;
use crate::lexer::{FloatSuffix, IntegerSuffix, Span};

/// Internal representation of an Oxy type.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeInfo {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
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
    Function {
        params: Vec<TypeInfo>,
        ret: Box<TypeInfo>,
    },
    Array(Box<TypeInfo>, usize),
    Unknown,
}

impl TypeInfo {
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            TypeInfo::I8
                | TypeInfo::I16
                | TypeInfo::I32
                | TypeInfo::I64
                | TypeInfo::U8
                | TypeInfo::U16
                | TypeInfo::U32
                | TypeInfo::U64
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(self, TypeInfo::F32 | TypeInfo::F64)
    }

    pub fn name(&self) -> &str {
        match self {
            TypeInfo::I8 => "i8",
            TypeInfo::I16 => "i16",
            TypeInfo::I32 => "i32",
            TypeInfo::I64 => "i64",
            TypeInfo::U8 => "u8",
            TypeInfo::U16 => "u16",
            TypeInfo::U32 => "u32",
            TypeInfo::U64 => "u64",
            TypeInfo::F32 => "f32",
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
            TypeInfo::Function { .. } => "fn",
            TypeInfo::Array(..) => "[...]",
            TypeInfo::Unknown => "?",
        }
    }

    pub fn from_annotation(ann: &Option<TypeAnnotation>) -> Result<TypeInfo, FerriError> {
        let ann = match ann {
            Some(a) => a,
            None => return Ok(TypeInfo::Unknown),
        };
        match ann {
            TypeAnnotation::Named { name, .. } => Ok(Self::from_name(name)),
            TypeAnnotation::Array { inner, size, .. } => {
                let inner_ty = Self::from_annotation(&Some(*inner.clone()))?;
                Ok(TypeInfo::Array(Box::new(inner_ty), *size))
            }
        }
    }

    pub fn from_name(name: &str) -> TypeInfo {
        // Parse function type syntax: fn(P1, P2, ...) -> R
        if let Some(inner) = name.strip_prefix("fn(") {
            if let Some(paren_end) = inner.find(')') {
                let params_str = &inner[..paren_end];
                let after_paren = &inner[paren_end + 1..];
                let ret_str = after_paren
                    .strip_prefix(" -> ")
                    .or_else(|| after_paren.trim_start().strip_prefix("-> "));
                let params: Vec<TypeInfo> = if params_str.is_empty() {
                    vec![]
                } else {
                    params_str
                        .split(',')
                        .map(|s| Self::from_name(s.trim()))
                        .collect()
                };
                let ret = if let Some(r) = ret_str {
                    Box::new(Self::from_name(r))
                } else {
                    Box::new(TypeInfo::Unit)
                };
                return TypeInfo::Function { params, ret };
            }
        }
        match name {
            "i8" => TypeInfo::I8,
            "i16" => TypeInfo::I16,
            "i32" => TypeInfo::I32,
            "i64" | "isize" => TypeInfo::I64,
            "u8" => TypeInfo::U8,
            "u16" => TypeInfo::U16,
            "u32" => TypeInfo::U32,
            "u64" | "usize" => TypeInfo::U64,
            "f32" => TypeInfo::F32,
            "f64" => TypeInfo::F64,
            "bool" => TypeInfo::Bool,
            "String" | "str" => TypeInfo::String,
            "char" => TypeInfo::Char,
            "Fn" => TypeInfo::Function {
                params: vec![],
                ret: Box::new(TypeInfo::Unknown),
            },
            "()" | "Unit" => TypeInfo::Unit,
            "Vec" => TypeInfo::Vec,
            "HashMap" => TypeInfo::HashMap,
            "Option" => TypeInfo::Option,
            "Result" => TypeInfo::Result,
            "_" => TypeInfo::Unknown,
            n => TypeInfo::UserStruct(n.to_string()),
        }
    }

    /// Returns true if `self` can accept a value of type `other`.
    /// Implements promotion: narrower → wider, int → float.
    pub fn accepts(&self, other: &TypeInfo) -> bool {
        if *self == TypeInfo::Unknown || *other == TypeInfo::Unknown {
            return true;
        }
        if self == other {
            return true;
        }
        // Any integer type accepts any other integer type (suffixed literals,
        // cross-sign assignments — wrapping happens at runtime).
        if self.is_integer() && other.is_integer() {
            return true;
        }
        // Float promotion
        if matches!((self, other), (TypeInfo::F64, TypeInfo::F32)) {
            return true;
        }
        // Integer → float
        if self.is_float() && other.is_integer() {
            return true;
        }
        // Function types: structural compatibility.
        if let (
            TypeInfo::Function {
                params: self_params,
                ret: self_ret,
            },
            TypeInfo::Function {
                params: other_params,
                ret: other_ret,
            },
        ) = (self, other)
        {
            // Bare Fn (0 params, no specific signature) — compatible with any function
            if self_params.is_empty() || other_params.is_empty() {
                return true;
            }
            if self_params.len() != other_params.len() {
                return false;
            }
            for (s, o) in self_params.iter().zip(other_params.iter()) {
                if !s.accepts(o) {
                    return false;
                }
            }
            return self_ret.accepts(other_ret);
        }
        // Unit accepts Function (closures returned from unannotated functions)
        if matches!(self, TypeInfo::Unit) && matches!(other, TypeInfo::Function { .. }) {
            return true;
        }
        // Array accepts Vec and vice versa (vec literal can initialize array-typed binding)
        if matches!(
            (&self, &other),
            (TypeInfo::Array(..), TypeInfo::Vec) | (TypeInfo::Vec, TypeInfo::Array(..))
        ) {
            return true;
        }
        false
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
    /// Declared parameter types per function/method, keyed by the same
    /// qualified name as `fn_return_types`. Generic params are stored as
    /// `TypeInfo::Unknown` so the caller-side accepts() check passes.
    fn_param_types: HashMap<String, Vec<TypeInfo>>,
    /// Tracks the current module nesting for field visibility checks.
    module_stack: Vec<String>,
    /// Import aliases: short_name → qualified_name (e.g. "Record" → "database::Record").
    use_aliases: HashMap<String, String>,
    /// Current impl type name for `Self` resolution (qualified).
    current_impl_type: Option<String>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            env: TypeEnv::new(),
            struct_defs: HashMap::new(),
            type_aliases: HashMap::new(),
            fn_return_types: HashMap::new(),
            fn_param_types: HashMap::new(),
            module_stack: Vec::new(),
            use_aliases: HashMap::new(),
            current_impl_type: None,
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    /// Resolve a type name through type aliases and module context.
    fn resolve_type(&self, name: &str) -> TypeInfo {
        // `Self` resolves to the current impl type
        if name == "Self" {
            if let Some(ref impl_type) = self.current_impl_type {
                return TypeInfo::UserStruct(impl_type.clone());
            }
        }
        if let Some(alias) = self.type_aliases.get(name) {
            return TypeInfo::from_annotation(&Some(alias.clone())).unwrap_or(TypeInfo::Unknown);
        }
        // Try module-qualified type alias
        if !name.contains("::") {
            let module_prefix = self.module_stack.join("::");
            if !module_prefix.is_empty() {
                let qualified = format!("{}::{}", module_prefix, name);
                if let Some(alias) = self.type_aliases.get(&qualified) {
                    return TypeInfo::from_annotation(&Some(alias.clone()))
                        .unwrap_or(TypeInfo::Unknown);
                }
            }
        }
        // Try module-qualified struct name
        let resolved = self.resolve_struct_name(name);
        TypeInfo::from_name(&resolved)
    }

    /// Resolve a TypeAnnotation to TypeInfo, handling Named and Array variants.
    fn resolve_annotation(&self, ann: &TypeAnnotation) -> TypeInfo {
        match ann {
            TypeAnnotation::Named { name, .. } => self.resolve_type(name),
            TypeAnnotation::Array { inner, size, .. } => {
                let inner_ty = self.resolve_annotation(inner);
                TypeInfo::Array(Box::new(inner_ty), *size)
            }
        }
    }

    /// Check whether a struct field is visible from the current module context.
    fn check_field_visible(
        &self,
        struct_name: &str,
        field_name: &str,
        span: Span,
    ) -> Result<(), FerriError> {
        if let Some(struct_def) = self.struct_defs.get(struct_name) {
            if let StructKind::Named(fields) = &struct_def.kind {
                for field in fields {
                    if field.name == field_name {
                        if matches!(field.visibility, Visibility::Private) {
                            let struct_module =
                                struct_name.rsplit_once("::").map(|(m, _)| m).unwrap_or("");
                            let current_module = self.module_stack.join("::");
                            if struct_module != current_module {
                                return Err(FerriError::Runtime {
                                    message: format!(
                                        "field `{}` of struct `{}` is private",
                                        field_name, struct_name
                                    ),
                                    line: span.line,
                                    column: span.column,
                                });
                            }
                        }
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    /// Resolve a struct name through use_aliases (for `use foo::Bar` → `Bar` unqualified).
    fn resolve_struct_name(&self, name: &str) -> String {
        // `Self` resolves to the current impl type
        if name == "Self" {
            if let Some(ref impl_type) = self.current_impl_type {
                return impl_type.clone();
            }
        }
        // Check use_aliases for a direct alias
        if let Some(resolved) = self.use_aliases.get(name) {
            if self.struct_defs.contains_key(resolved) {
                return resolved.clone();
            }
        }
        // Try module-qualified name (current module prefix + name)
        if !name.contains("::") {
            let module_prefix = self.module_stack.join("::");
            if !module_prefix.is_empty() {
                let qualified = format!("{}::{}", module_prefix, name);
                if self.struct_defs.contains_key(&qualified) {
                    return qualified;
                }
            }
        }
        name.to_string()
    }

    pub fn check_program(&mut self, program: &Program) -> Result<(), FerriError> {
        // First pass: collect struct defs, type aliases, and use aliases
        self.collect_defs(&program.items, "");

        // Second pass: register function return types
        self.collect_fn_types(&program.items, "");

        // Third pass: check each item
        for item in &program.items {
            self.check_item(item)?;
        }

        Ok(())
    }

    /// Recursively collect struct defs, type aliases, and use aliases with module prefix.
    fn collect_defs(&mut self, items: &[Item], prefix: &str) {
        for item in items {
            match item {
                Item::Struct(s) => {
                    let qualified = if prefix.is_empty() {
                        s.name.clone()
                    } else {
                        format!("{}::{}", prefix, s.name)
                    };
                    self.struct_defs.insert(qualified, s.clone());
                }
                Item::TypeAlias { name, target, .. } => {
                    let qualified = if prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{}::{}", prefix, name)
                    };
                    self.type_aliases.insert(qualified, target.clone());
                }
                Item::Module(m) => {
                    let nested_prefix = if prefix.is_empty() {
                        m.name.clone()
                    } else {
                        format!("{}::{}", prefix, m.name)
                    };
                    if let Some(body) = &m.body {
                        self.collect_defs(body, &nested_prefix);
                    }
                }
                _ => {}
            }
        }
    }

    /// Recursively register function return types with module prefix.
    /// Generic-parameter names declared on the struct identified by
    /// `qualified` (or its bare type name). Empty if not a known struct.
    fn struct_generic_names(&self, qualified: &str) -> Vec<String> {
        let bare = qualified.rsplit("::").next().unwrap_or(qualified);
        self.struct_defs
            .get(qualified)
            .or_else(|| self.struct_defs.get(bare))
            .map(|def| def.generic_params.iter().map(|p| p.name.clone()).collect())
            .unwrap_or_default()
    }

    /// Resolve each param's declared type to `TypeInfo`. Names that match
    /// either the function's own generic params or `extra_generics` (e.g.
    /// generics from an enclosing impl block) become `TypeInfo::Unknown`
    /// so call-site checks don't false-positive against monomorphic args.
    fn resolve_param_types(&self, f: &FnDef, extra_generics: &[String]) -> Vec<TypeInfo> {
        let mut generic_names: Vec<&str> =
            f.generic_params.iter().map(|p| p.name.as_str()).collect();
        for n in extra_generics {
            generic_names.push(n.as_str());
        }
        f.params
            .iter()
            .map(|p| match &p.type_ann {
                TypeAnnotation::Named { name, .. } if generic_names.contains(&name.as_str()) => {
                    TypeInfo::Unknown
                }
                ann => self.resolve_annotation(ann),
            })
            .collect()
    }

    fn collect_fn_types(&mut self, items: &[Item], prefix: &str) {
        let saved_stack = self.module_stack.clone();
        self.module_stack = if prefix.is_empty() {
            vec![]
        } else {
            prefix.split("::").map(|s| s.to_string()).collect()
        };
        for item in items {
            match item {
                Item::Function(f) => {
                    let qualified = if prefix.is_empty() {
                        f.name.clone()
                    } else {
                        format!("{}::{}", prefix, f.name)
                    };
                    let ret_ty = if let Some(ref ann) = f.return_type {
                        let is_generic = match ann {
                            TypeAnnotation::Named { name, .. } => {
                                let generic_names: Vec<&str> =
                                    f.generic_params.iter().map(|p| p.name.as_str()).collect();
                                generic_names.contains(&name.as_str())
                            }
                            TypeAnnotation::Array { .. } => false,
                        };
                        if is_generic {
                            TypeInfo::Unknown
                        } else {
                            self.resolve_annotation(ann)
                        }
                    } else {
                        TypeInfo::Unit
                    };
                    let param_tys = self.resolve_param_types(f, &[]);
                    self.fn_return_types.insert(qualified.clone(), ret_ty);
                    self.fn_param_types.insert(qualified, param_tys);
                }
                Item::Module(m) => {
                    let nested_prefix = if prefix.is_empty() {
                        m.name.clone()
                    } else {
                        format!("{}::{}", prefix, m.name)
                    };
                    if let Some(body) = &m.body {
                        self.collect_fn_types(body, &nested_prefix);
                    }
                }
                Item::Impl(i) => {
                    let type_prefix = if prefix.is_empty() {
                        i.type_name.clone()
                    } else {
                        format!("{}::{}", prefix, i.type_name)
                    };
                    let impl_generics = self.struct_generic_names(&type_prefix);
                    for method in &i.methods {
                        let qualified = format!("{}::{}", type_prefix, method.name);
                        let unqualified = format!("{}::{}", i.type_name, method.name);
                        let ret_ty = if let Some(ref ann) = method.return_type {
                            self.resolve_annotation(ann)
                        } else {
                            TypeInfo::Unit
                        };
                        let param_tys = self.resolve_param_types(method, &impl_generics);
                        // Also register under unqualified type name (for use-aliased lookups)
                        self.fn_return_types
                            .insert(unqualified.clone(), ret_ty.clone());
                        self.fn_return_types.insert(qualified.clone(), ret_ty);
                        self.fn_param_types.insert(unqualified, param_tys.clone());
                        self.fn_param_types.insert(qualified, param_tys);
                    }
                }
                Item::ImplTrait(i) => {
                    let type_prefix = if prefix.is_empty() {
                        i.type_name.clone()
                    } else {
                        format!("{}::{}", prefix, i.type_name)
                    };
                    let impl_generics = self.struct_generic_names(&type_prefix);
                    for method in &i.methods {
                        let qualified = format!("{}::{}", type_prefix, method.name);
                        let unqualified = format!("{}::{}", i.type_name, method.name);
                        let ret_ty = if let Some(ref ann) = method.return_type {
                            self.resolve_annotation(ann)
                        } else {
                            TypeInfo::Unit
                        };
                        let param_tys = self.resolve_param_types(method, &impl_generics);
                        self.fn_return_types
                            .insert(unqualified.clone(), ret_ty.clone());
                        self.fn_return_types.insert(qualified.clone(), ret_ty);
                        self.fn_param_types.insert(unqualified, param_tys.clone());
                        self.fn_param_types.insert(qualified, param_tys);
                    }
                }
                _ => {}
            }
        }
        self.module_stack = saved_stack;
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
                    self.resolve_annotation(ann)
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
            Item::Module(m) => {
                self.module_stack.push(m.name.clone());
                if let Some(body) = &m.body {
                    for item in body {
                        self.check_item(item)?;
                    }
                }
                self.module_stack.pop();
                Ok(())
            }
            Item::Use(use_def) => {
                let base_path = use_def.path.join("::");
                match &use_def.tree {
                    UseTree::Simple(alias) => {
                        let local_name = alias
                            .as_ref()
                            .cloned()
                            .unwrap_or_else(|| use_def.path.last().cloned().unwrap_or_default());
                        self.use_aliases.insert(local_name, base_path.clone());
                    }
                    UseTree::Group(items) => {
                        for (name, alias) in items {
                            let local_name = alias.as_ref().unwrap_or(name);
                            let qualified = format!("{}::{}", base_path, name);
                            self.use_aliases.insert(local_name.clone(), qualified);
                        }
                    }
                    UseTree::Glob => {
                        // Glob: we can't enumerate all exports at type-check time,
                        // so we skip. Visibility is enforced by the compiler.
                    }
                }
                Ok(())
            }
            Item::Impl(i) => {
                let qualified_type = if self.module_stack.is_empty() {
                    i.type_name.clone()
                } else {
                    format!("{}::{}", self.module_stack.join("::"), i.type_name)
                };
                let resolved = self.resolve_struct_name(&qualified_type);
                let saved_impl = self.current_impl_type.clone();
                self.current_impl_type = Some(resolved);
                for method in &i.methods {
                    self.check_function(method)?;
                }
                self.current_impl_type = saved_impl;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn check_function(&mut self, f: &FnDef) -> Result<(), FerriError> {
        let ret_ty = if let Some(ref ann) = f.return_type {
            // If return type is a generic param, treat as Unknown (type-erased generics)
            let is_generic = match ann {
                TypeAnnotation::Named { name, .. } => {
                    let generic_names: Vec<&str> =
                        f.generic_params.iter().map(|p| p.name.as_str()).collect();
                    generic_names.contains(&name.as_str())
                }
                TypeAnnotation::Array { .. } => false,
            };
            if is_generic {
                TypeInfo::Unknown
            } else {
                self.resolve_annotation(ann)
            }
        } else {
            TypeInfo::Unit
        };
        self.fn_return_types.insert(f.name.clone(), ret_ty.clone());
        let impl_generics = self
            .current_impl_type
            .as_deref()
            .map(|t| self.struct_generic_names(t))
            .unwrap_or_default();
        let param_tys = self.resolve_param_types(f, &impl_generics);
        self.fn_param_types
            .insert(f.name.clone(), param_tys.clone());

        let fn_env = TypeEnv::child(&self.env);
        for (param, p_ty) in f.params.iter().zip(param_tys.iter()) {
            fn_env.borrow_mut().define(&param.name, p_ty.clone());
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
                    self.resolve_annotation(ann)
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
            Stmt::Use(use_def) => {
                let base_path = use_def.path.join("::");
                match &use_def.tree {
                    UseTree::Simple(alias) => {
                        let local_name = alias
                            .as_ref()
                            .cloned()
                            .unwrap_or_else(|| use_def.path.last().cloned().unwrap_or_default());
                        self.use_aliases.insert(local_name, base_path.clone());
                    }
                    UseTree::Group(items) => {
                        for (name, alias) in items {
                            let local_name = alias.as_ref().unwrap_or(name);
                            let qualified = format!("{}::{}", base_path, name);
                            self.use_aliases.insert(local_name.clone(), qualified);
                        }
                    }
                    UseTree::Glob => {
                        // Glob entries are resolved by the compiler
                    }
                }
                Ok(())
            }
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

    /// Check arity + per-arg type compatibility against the declared
    /// `params`. `display_name` and `span` are used for error messages.
    /// `skip_self` drops the first param (for method-call syntax where
    /// the receiver is implicit). Returns the first mismatch as a
    /// `TypeError`, or `Ok(())` if all args fit.
    fn check_args_against_params(
        &mut self,
        params: &[TypeInfo],
        args: &[Expr],
        skip_self: bool,
        display_name: &str,
        span: Span,
    ) -> Result<(), FerriError> {
        let effective: &[TypeInfo] = if skip_self && !params.is_empty() {
            &params[1..]
        } else {
            params
        };
        if args.len() != effective.len() {
            return Err(FerriError::TypeError {
                message: format!(
                    "wrong number of arguments to `{display_name}`: expected {}, got {}",
                    effective.len(),
                    args.len()
                ),
                line: span.line,
                column: span.column,
            });
        }
        for (i, (param_ty, arg)) in effective.iter().zip(args.iter()).enumerate() {
            let arg_ty = self.infer_expr(arg)?;
            if !param_ty.accepts(&arg_ty) {
                let arg_span = arg.span();
                return Err(FerriError::TypeError {
                    message: format!(
                        "type mismatch in call to `{display_name}`: argument {} expected `{}`, got `{}`",
                        i + 1,
                        param_ty.name(),
                        arg_ty.name()
                    ),
                    line: arg_span.line,
                    column: arg_span.column,
                });
            }
        }
        Ok(())
    }

    fn infer_expr(&mut self, expr: &Expr) -> Result<TypeInfo, FerriError> {
        match expr {
            Expr::IntLiteral(_, suffix, _) => Ok(match suffix {
                IntegerSuffix::I8 => TypeInfo::I8,
                IntegerSuffix::I16 => TypeInfo::I16,
                IntegerSuffix::I32 => TypeInfo::I32,
                IntegerSuffix::I64 => TypeInfo::I64,
                IntegerSuffix::U8 => TypeInfo::U8,
                IntegerSuffix::U16 => TypeInfo::U16,
                IntegerSuffix::U32 => TypeInfo::U32,
                IntegerSuffix::U64 => TypeInfo::U64,
                IntegerSuffix::None => TypeInfo::I64,
            }),
            Expr::FloatLiteral(_, suffix, _) => Ok(match suffix {
                FloatSuffix::F32 => TypeInfo::F32,
                FloatSuffix::F64 => TypeInfo::F64,
                FloatSuffix::None => TypeInfo::F64,
            }),
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
                // Try module-qualified function return type
                {
                    let module_prefix = self.module_stack.join("::");
                    if !module_prefix.is_empty() {
                        let qualified = format!("{}::{}", module_prefix, name);
                        if let Some(ret) = self.fn_return_types.get(&qualified) {
                            return Ok(ret.clone());
                        }
                    }
                }
                // Try use_aliases -> struct_defs
                if let Some(resolved) = self.use_aliases.get(name) {
                    if self.struct_defs.contains_key(resolved) {
                        return Ok(TypeInfo::UserStruct(resolved.clone()));
                    }
                }
                // Try module-qualified struct name
                let resolved = self.resolve_struct_name(name);
                if self.struct_defs.contains_key(&resolved) {
                    return Ok(TypeInfo::UserStruct(resolved));
                }
                Ok(TypeInfo::Unknown)
            }

            Expr::BinaryOp {
                op, left, right, ..
            } => {
                let lt = self.infer_expr(left)?;
                let rt = self.infer_expr(right)?;
                // Comparisons and logical ops always produce Bool.
                if matches!(
                    op,
                    BinOp::Eq
                        | BinOp::NotEq
                        | BinOp::Lt
                        | BinOp::Gt
                        | BinOp::LtEq
                        | BinOp::GtEq
                        | BinOp::And
                        | BinOp::Or
                ) {
                    return Ok(TypeInfo::Bool);
                }
                // String concatenation
                if lt == TypeInfo::String || rt == TypeInfo::String {
                    return Ok(TypeInfo::String);
                }
                // Char + Char → String (or Char + anything → String)
                if lt == TypeInfo::Char || rt == TypeInfo::Char {
                    return Ok(TypeInfo::String);
                }
                // Numeric ops: float wins, otherwise promote to wider integer
                if matches!(lt, TypeInfo::F32 | TypeInfo::F64)
                    || matches!(rt, TypeInfo::F32 | TypeInfo::F64)
                {
                    Ok(TypeInfo::F64)
                } else {
                    Ok(TypeInfo::I64)
                }
            }

            Expr::UnaryOp { expr: inner, .. } => self.infer_expr(inner),

            Expr::Call {
                callee, args, span, ..
            } => {
                if let Expr::Ident(name, _) = callee.as_ref() {
                    // Resolve the callee's qualified name and look up its params.
                    let resolved_key = if self.fn_param_types.contains_key(name) {
                        Some(name.clone())
                    } else if !name.contains("::") {
                        let module_prefix = self.module_stack.join("::");
                        if !module_prefix.is_empty() {
                            let qualified = format!("{}::{}", module_prefix, name);
                            if self.fn_param_types.contains_key(&qualified) {
                                Some(qualified)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if let Some(key) = resolved_key {
                        let params = self.fn_param_types.get(&key).cloned().unwrap_or_default();
                        self.check_args_against_params(&params, args, false, name, *span)?;
                        if let Some(ret) = self.fn_return_types.get(&key) {
                            return Ok(ret.clone());
                        }
                    } else {
                        // Unknown callee — fall back to inferring args without
                        // checking against any signature.
                        for arg in args {
                            self.infer_expr(arg)?;
                        }
                        // Built-in constructors
                        match name.as_str() {
                            "Some" => return Ok(TypeInfo::Option),
                            "Ok" | "Err" => return Ok(TypeInfo::Result),
                            _ => {}
                        }
                    }
                } else {
                    for arg in args {
                        self.infer_expr(arg)?;
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

            Expr::Repeat { value, count, .. } => {
                let val_ty = self.infer_expr(value)?;
                let _ = self.infer_expr(count)?;
                Ok(val_ty)
            }

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
                match target.as_ref() {
                    Expr::Ident(name, _) => {
                        // Check compatibility with existing binding
                        if let Some(existing) = self.env.borrow().get(name) {
                            if !existing.accepts(&vt) {
                                return Err(FerriError::TypeError {
                                    message: format!(
                                        "type mismatch: cannot assign `{}` to variable `{name}` of type `{}`",
                                        vt.name(),
                                        existing.name()
                                    ),
                                    line: target.span().line,
                                    column: target.span().column,
                                });
                            }
                        }
                        self.env.borrow_mut().define(name, vt);
                    }
                    Expr::FieldAccess {
                        object,
                        field,
                        span: fspan,
                    } => {
                        let obj_ty = self.infer_expr(object)?;
                        if let TypeInfo::UserStruct(struct_name) = &obj_ty {
                            let resolved = self.resolve_struct_name(struct_name);
                            if let Some(def) = self.struct_defs.get(&resolved) {
                                let generic_names: Vec<String> =
                                    def.generic_params.iter().map(|p| p.name.clone()).collect();
                                if let StructKind::Named(decl_fields) = &def.kind {
                                    for f in decl_fields {
                                        if f.name == *field {
                                            let decl_ty = match &f.type_ann {
                                                TypeAnnotation::Named { name, .. }
                                                    if generic_names.contains(name) =>
                                                {
                                                    TypeInfo::Unknown
                                                }
                                                ann => self.resolve_annotation(ann),
                                            };
                                            if !decl_ty.accepts(&vt) {
                                                return Err(FerriError::TypeError {
                                                    message: format!(
                                                        "type mismatch: cannot assign `{}` to field `{}.{field}` of type `{}`",
                                                        vt.name(),
                                                        resolved,
                                                        decl_ty.name()
                                                    ),
                                                    line: fspan.line,
                                                    column: fspan.column,
                                                });
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
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

            Expr::PathCall {
                path, args, span, ..
            } => {
                let qualified = path.join("::");
                // Resolve key, mirroring the lookup order used for fn_return_types.
                let resolved_key = if self.fn_param_types.contains_key(&qualified) {
                    Some(qualified.clone())
                } else if path.len() == 2 {
                    self.use_aliases.get(&path[0]).and_then(|prefix| {
                        let aliased = format!("{}::{}", prefix, &path[1]);
                        if self.fn_param_types.contains_key(&aliased) {
                            Some(aliased)
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
                .or_else(|| {
                    let module_prefix = self.module_stack.join("::");
                    if module_prefix.is_empty() {
                        None
                    } else {
                        let module_qualified = format!("{}::{}", module_prefix, qualified);
                        if self.fn_param_types.contains_key(&module_qualified) {
                            Some(module_qualified)
                        } else {
                            None
                        }
                    }
                });
                if let Some(key) = resolved_key {
                    let params = self.fn_param_types.get(&key).cloned().unwrap_or_default();
                    self.check_args_against_params(&params, args, false, &qualified, *span)?;
                    if let Some(ret) = self.fn_return_types.get(&key) {
                        return Ok(ret.clone());
                    }
                } else {
                    for arg in args {
                        self.infer_expr(arg)?;
                    }
                }
                Ok(TypeInfo::Unknown)
            }

            Expr::MethodCall {
                object,
                method,
                args,
                span,
                ..
            } => {
                let obj_ty = self.infer_expr(object)?;
                if let TypeInfo::UserStruct(struct_name) = &obj_ty {
                    let resolved = self.resolve_struct_name(struct_name);
                    let qualified = format!("{}::{}", resolved, method);
                    let module_qualified = if self.module_stack.is_empty() {
                        None
                    } else {
                        Some(format!(
                            "{}::{}::{}",
                            self.module_stack.join("::"),
                            resolved,
                            method
                        ))
                    };
                    let resolved_key = if self.fn_param_types.contains_key(&qualified) {
                        Some(qualified.clone())
                    } else if let Some(mq) = module_qualified.as_ref() {
                        if self.fn_param_types.contains_key(mq) {
                            Some(mq.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if let Some(key) = resolved_key {
                        let params = self.fn_param_types.get(&key).cloned().unwrap_or_default();
                        self.check_args_against_params(&params, args, true, method, *span)?;
                        if let Some(ret_ty) = self.fn_return_types.get(&key) {
                            return Ok(ret_ty.clone());
                        }
                    } else {
                        // Unknown user-method — infer args for side effects,
                        // then fall through to the builtin method table.
                        for arg in args {
                            self.infer_expr(arg)?;
                        }
                    }
                } else {
                    for arg in args {
                        self.infer_expr(arg)?;
                    }
                }
                // Common built-in method return types. Keeps downstream
                // type-checking honest when calls are chained through builtins
                // like `.to_string()`. Anything not listed stays Unknown.
                Ok(match method.as_str() {
                    "to_string" => TypeInfo::String,
                    "len" => TypeInfo::I64,
                    "is_empty" | "contains" | "starts_with" | "ends_with" => TypeInfo::Bool,
                    "clone" => obj_ty.clone(),
                    _ => TypeInfo::Unknown,
                })
            }

            Expr::FieldAccess {
                object,
                field,
                span,
                ..
            } => {
                let obj_ty = self.infer_expr(object)?;
                if let TypeInfo::UserStruct(struct_name) = &obj_ty {
                    let resolved = self.resolve_struct_name(struct_name);
                    self.check_field_visible(&resolved, field, *span)?;
                    // Return the field's declared type
                    if let Some(def) = self.struct_defs.get(&resolved) {
                        if let StructKind::Named(fields) = &def.kind {
                            for f in fields {
                                if f.name == *field {
                                    return Ok(self.resolve_annotation(&f.type_ann));
                                }
                            }
                        }
                    }
                }
                Ok(TypeInfo::Unknown)
            }

            Expr::Index { object, index, .. } => {
                let obj_ty = self.infer_expr(object)?;
                let _ = self.infer_expr(index)?;
                if obj_ty == TypeInfo::String {
                    return Ok(TypeInfo::Char);
                }
                Ok(TypeInfo::Unknown)
            }

            Expr::Range { .. } => Ok(TypeInfo::I64),

            Expr::StructInit {
                name, fields, span, ..
            } => {
                let resolved = self.resolve_struct_name(name);
                // Pre-collect declared field types so we can borrow self mutably
                // inside the inference loop. Generic parameter names (e.g. `T`)
                // resolve to Unknown so call-site checks pass.
                let field_types: HashMap<String, TypeInfo> =
                    if let Some(def) = self.struct_defs.get(&resolved) {
                        let generic_names: Vec<String> =
                            def.generic_params.iter().map(|p| p.name.clone()).collect();
                        if let StructKind::Named(decl_fields) = &def.kind {
                            decl_fields
                                .iter()
                                .map(|f| {
                                    let ty = match &f.type_ann {
                                        TypeAnnotation::Named { name, .. }
                                            if generic_names.contains(name) =>
                                        {
                                            TypeInfo::Unknown
                                        }
                                        ann => self.resolve_annotation(ann),
                                    };
                                    (f.name.clone(), ty)
                                })
                                .collect()
                        } else {
                            HashMap::new()
                        }
                    } else {
                        HashMap::new()
                    };
                for (field_name, f_expr) in fields {
                    self.check_field_visible(&resolved, field_name, *span)?;
                    let val_ty = self.infer_expr(f_expr)?;
                    if let Some(decl_ty) = field_types.get(field_name) {
                        if !decl_ty.accepts(&val_ty) {
                            let fspan = f_expr.span();
                            return Err(FerriError::TypeError {
                                message: format!(
                                    "type mismatch: field `{}.{field_name}` declared as `{}`, got `{}`",
                                    resolved,
                                    decl_ty.name(),
                                    val_ty.name()
                                ),
                                line: fspan.line,
                                column: fspan.column,
                            });
                        }
                    }
                }
                Ok(TypeInfo::UserStruct(resolved))
            }

            Expr::Try { expr: inner, .. } => {
                let _ = self.infer_expr(inner)?;
                Ok(TypeInfo::Unknown)
            }

            Expr::Closure {
                params,
                return_type,
                body,
                ..
            } => {
                let mut param_types = Vec::with_capacity(params.len());
                let closure_env = TypeEnv::child(&self.env);
                for p in params {
                    let p_ty = if let Some(ref ann) = p.type_ann {
                        self.resolve_annotation(ann)
                    } else {
                        TypeInfo::Unknown
                    };
                    closure_env.borrow_mut().define(&p.name, p_ty.clone());
                    param_types.push(p_ty);
                }
                let saved_env = self.env.clone();
                self.env = closure_env;
                let inferred_ret = self.infer_expr(body)?;
                self.env = saved_env;
                if let Some(ref ann) = return_type {
                    let declared_ret = self.resolve_annotation(ann);
                    if !declared_ret.accepts(&inferred_ret) {
                        return Err(FerriError::TypeError {
                            message: format!(
                                "type mismatch: closure returns `{}`, but body has type `{}`",
                                declared_ret.name(),
                                inferred_ret.name()
                            ),
                            line: ann.span().line,
                            column: ann.span().column,
                        });
                    }
                }
                Ok(TypeInfo::Function {
                    params: param_types,
                    ret: Box::new(inferred_ret),
                })
            }
            Expr::Await { expr: inner, .. } => {
                let _ = self.infer_expr(inner)?;
                Ok(TypeInfo::Unknown)
            }
            Expr::FString { .. } => Ok(TypeInfo::String),
            Expr::MacroCall { args, .. } => {
                // Macros are opaque at type-check time, but their argument
                // expressions still need to be inferred so any calls or
                // field accesses inside get their own type checks run.
                for arg in args {
                    self.infer_expr(arg)?;
                }
                Ok(TypeInfo::Unknown)
            }
            Expr::Path { segments, .. } => {
                let qualified = segments.join("::");
                if let Some(ret) = self.fn_return_types.get(&qualified) {
                    return Ok(ret.clone());
                }
                if self.struct_defs.contains_key(&qualified) {
                    return Ok(TypeInfo::UserStruct(qualified));
                }
                // Try through use_aliases for the first segment
                if segments.len() == 2 {
                    if let Some(resolved) = self.use_aliases.get(&segments[0]) {
                        let full = format!("{}::{}", resolved, segments[1]);
                        if self.struct_defs.contains_key(&full) {
                            return Ok(TypeInfo::UserStruct(full));
                        }
                    }
                }
                Ok(TypeInfo::Unknown)
            }
            Expr::SelfRef { .. } => {
                if let Some(ref impl_type) = self.current_impl_type {
                    Ok(TypeInfo::UserStruct(impl_type.clone()))
                } else {
                    Ok(TypeInfo::Unknown)
                }
            }
            Expr::As {
                expr, type_name, ..
            } => {
                let _ = self.infer_expr(expr)?;
                Ok(TypeInfo::from_name(type_name))
            }
            Expr::Return { value, .. } => {
                if let Some(expr) = value {
                    let _ = self.infer_expr(expr)?;
                }
                Ok(TypeInfo::Unknown) // diverging expression
            }
            Expr::CompoundAssign { target, value, .. } => {
                let vt = self.infer_expr(value)?;
                if let Expr::Ident(name, _) = target.as_ref() {
                    if let Some(existing) = self.env.borrow().get(name) {
                        if !existing.accepts(&vt) {
                            return Err(FerriError::TypeError {
                                message: format!(
                                    "type mismatch: cannot compound-assign `{}` to variable `{name}` of type `{}`",
                                    vt.name(),
                                    existing.name()
                                ),
                                line: target.span().line,
                                column: target.span().column,
                            });
                        }
                    }
                }
                Ok(TypeInfo::Unit)
            }
        }
    }
}

#[cfg(test)]
mod tests;
