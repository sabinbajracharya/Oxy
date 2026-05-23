//! Semantic type checker for Oxy.
//!
//! Runs after parsing and before execution. Validates type annotations
//! on `let` bindings, function params, and return types.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::*;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::symbols;

/// Internal representation of an Oxy type.
///
/// Numeric variants `I64`, `U8`, `F64` map to the surface names
/// `int`, `byte`, `float`. They keep the storage-shaped names to mirror
/// the corresponding `Value::I64 / U8 / F64` variants. There are no other
/// numeric widths — the Rust-style width zoo was retired.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeInfo {
    I64,
    U8,
    F64,
    Bool,
    String,
    Char,
    Unit,
    /// `Vec<T>` — element type carried in the box.
    Vec(Box<TypeInfo>),
    /// `HashMap<K, V>`.
    HashMap(Box<TypeInfo>, Box<TypeInfo>),
    /// `BTreeMap<K, V>`.
    BTreeMap(Box<TypeInfo>, Box<TypeInfo>),
    /// `Option<T>`.
    Option(Box<TypeInfo>),
    /// `Result<T, E>`.
    Result(Box<TypeInfo>, Box<TypeInfo>),
    /// A user-defined struct or enum, optionally parameterized by generic
    /// type arguments (`Box<i64>` → name = "Box", generic_args = [I64]).
    UserStruct {
        name: String,
        generic_args: Vec<TypeInfo>,
    },
    Function {
        params: Vec<TypeInfo>,
        ret: Box<TypeInfo>,
    },
    Array(Box<TypeInfo>, usize),
    Unknown,
}

impl TypeInfo {
    pub fn is_integer(&self) -> bool {
        matches!(self, TypeInfo::I64 | TypeInfo::U8)
    }

    pub fn is_float(&self) -> bool {
        matches!(self, TypeInfo::F64)
    }

    pub fn name(&self) -> &str {
        match self {
            TypeInfo::I64 => "int",
            TypeInfo::U8 => "byte",
            TypeInfo::F64 => "float",
            TypeInfo::Bool => "bool",
            TypeInfo::String => "String",
            TypeInfo::Char => "char",
            TypeInfo::Unit => "()",
            // Parameterized containers print their bare head — callers that
            // want the full `Vec<T>` form should use `display_name()`.
            TypeInfo::Vec(_) => "Vec",
            TypeInfo::HashMap(..) => "HashMap",
            TypeInfo::BTreeMap(..) => "BTreeMap",
            TypeInfo::Option(_) => "Option",
            TypeInfo::Result(..) => "Result",
            TypeInfo::UserStruct { name, .. } => name.as_str(),
            TypeInfo::Function { .. } => "fn",
            TypeInfo::Array(..) => "[...]",
            TypeInfo::Unknown => "?",
        }
    }

    /// Owned, fully-parameterized display name (e.g. `Vec<i64>`).
    pub fn display_name(&self) -> String {
        match self {
            TypeInfo::Vec(t) => format!("Vec<{}>", t.display_name()),
            TypeInfo::Option(t) => format!("Option<{}>", t.display_name()),
            TypeInfo::HashMap(k, v) => {
                format!("HashMap<{}, {}>", k.display_name(), v.display_name())
            }
            TypeInfo::BTreeMap(k, v) => {
                format!("BTreeMap<{}, {}>", k.display_name(), v.display_name())
            }
            TypeInfo::Result(t, e) => {
                format!("Result<{}, {}>", t.display_name(), e.display_name())
            }
            TypeInfo::Array(elem, n) => format!("[{}; {n}]", elem.display_name()),
            _ => self.name().to_string(),
        }
    }

    pub fn from_annotation(ann: &Option<TypeAnnotation>) -> Result<TypeInfo, FerriError> {
        let ann = match ann {
            Some(a) => a,
            None => return Ok(TypeInfo::Unknown),
        };
        match ann {
            TypeAnnotation::Named {
                name, generic_args, ..
            } => {
                let head = Self::from_name(name);
                Ok(Self::apply_generics(head, generic_args)?)
            }
            TypeAnnotation::Array { inner, size, .. } => {
                let inner_ty = Self::from_annotation(&Some(*inner.clone()))?;
                Ok(TypeInfo::Array(Box::new(inner_ty), *size))
            }
        }
    }

    /// Construct an un-parameterized user struct/enum type.
    pub fn user_struct(name: impl Into<String>) -> TypeInfo {
        TypeInfo::UserStruct {
            name: name.into(),
            generic_args: Vec::new(),
        }
    }

    /// Substitute the user-supplied generic arguments into a parameterized
    /// container type. Non-container heads ignore their generic args.
    pub fn apply_generics(
        head: TypeInfo,
        generic_args: &[TypeAnnotation],
    ) -> Result<TypeInfo, FerriError> {
        let arg = |i: usize| -> Result<TypeInfo, FerriError> {
            match generic_args.get(i) {
                Some(a) => Self::from_annotation(&Some(a.clone())),
                None => Ok(TypeInfo::Unknown),
            }
        };
        Ok(match head {
            TypeInfo::Vec(_) => TypeInfo::Vec(Box::new(arg(0)?)),
            TypeInfo::Option(_) => TypeInfo::Option(Box::new(arg(0)?)),
            TypeInfo::HashMap(..) => TypeInfo::HashMap(Box::new(arg(0)?), Box::new(arg(1)?)),
            TypeInfo::BTreeMap(..) => TypeInfo::BTreeMap(Box::new(arg(0)?), Box::new(arg(1)?)),
            TypeInfo::Result(..) => TypeInfo::Result(Box::new(arg(0)?), Box::new(arg(1)?)),
            TypeInfo::UserStruct { name, .. } if !generic_args.is_empty() => {
                let mut resolved = Vec::with_capacity(generic_args.len());
                for a in generic_args {
                    resolved.push(Self::from_annotation(&Some(a.clone()))?);
                }
                TypeInfo::UserStruct {
                    name,
                    generic_args: resolved,
                }
            }
            other => other,
        })
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
            // Oxy has exactly two integer types: `int` (= i64 internally)
            // and `byte` (= u8). The Rust-style width zoo was retired in
            // favour of a single sensible default — see CLAUDE.md.
            "int" => TypeInfo::I64,
            "byte" => TypeInfo::U8,
            "float" => TypeInfo::F64,
            "bool" => TypeInfo::Bool,
            "String" | "str" => TypeInfo::String,
            "char" => TypeInfo::Char,
            "Fn" => TypeInfo::Function {
                params: vec![],
                ret: Box::new(TypeInfo::Unknown),
            },
            "()" | "Unit" => TypeInfo::Unit,
            "Vec" => TypeInfo::Vec(Box::new(TypeInfo::Unknown)),
            "HashMap" => {
                TypeInfo::HashMap(Box::new(TypeInfo::Unknown), Box::new(TypeInfo::Unknown))
            }
            "BTreeMap" => {
                TypeInfo::BTreeMap(Box::new(TypeInfo::Unknown), Box::new(TypeInfo::Unknown))
            }
            "Option" => TypeInfo::Option(Box::new(TypeInfo::Unknown)),
            "Result" => TypeInfo::Result(Box::new(TypeInfo::Unknown), Box::new(TypeInfo::Unknown)),
            "_" => TypeInfo::Unknown,
            n => TypeInfo::UserStruct {
                name: n.to_string(),
                generic_args: Vec::new(),
            },
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
        // Array literal accepts Vec literal and vice versa (untyped Vec
        // can initialize an array-typed binding). Element types must agree.
        if let (TypeInfo::Array(se, _), TypeInfo::Vec(oe)) = (self, other) {
            return se.accepts(oe);
        }
        if let (TypeInfo::Vec(se), TypeInfo::Array(oe, _)) = (self, other) {
            return se.accepts(oe);
        }
        // Array-to-Array: recurse into element type and require matching length.
        if let (TypeInfo::Array(se, sn), TypeInfo::Array(oe, on)) = (self, other) {
            return sn == on && se.accepts(oe);
        }
        // Container-to-container: recurse into parameters.
        if let (TypeInfo::Vec(se), TypeInfo::Vec(oe)) = (self, other) {
            return se.accepts(oe);
        }
        if let (TypeInfo::Option(se), TypeInfo::Option(oe)) = (self, other) {
            return se.accepts(oe);
        }
        if let (TypeInfo::Result(st, se), TypeInfo::Result(ot, oe)) = (self, other) {
            return st.accepts(ot) && se.accepts(oe);
        }
        if let (TypeInfo::HashMap(sk, sv), TypeInfo::HashMap(ok, ov)) = (self, other) {
            return sk.accepts(ok) && sv.accepts(ov);
        }
        // User-defined structs/enums: same head name, with generic-args
        // compared element-wise. An empty generic-args list on either side
        // acts as a wildcard, so legacy paths that didn't set args still
        // unify with newly-parameterized values.
        if let (
            TypeInfo::UserStruct {
                name: sn,
                generic_args: sa,
            },
            TypeInfo::UserStruct {
                name: on,
                generic_args: oa,
            },
        ) = (self, other)
        {
            if sn != on {
                return false;
            }
            if sa.is_empty() || oa.is_empty() {
                return true;
            }
            if sa.len() != oa.len() {
                return false;
            }
            return sa.iter().zip(oa.iter()).all(|(s, o)| s.accepts(o));
        }
        if let (TypeInfo::BTreeMap(sk, sv), TypeInfo::BTreeMap(ok, ov)) = (self, other) {
            return sk.accepts(ok) && sv.accepts(ov);
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
    /// Names of generic-type parameters in scope (from the enclosing fn,
    /// impl, or struct definition). Allowed as user-struct type names without
    /// triggering "unknown type" errors.
    current_generics: Vec<String>,
    /// Known enum names (qualified). Populated alongside struct_defs so the
    /// type-name validator can accept them.
    enum_defs: std::collections::HashSet<String>,
    /// Declared return type of the function currently being checked.
    /// Used by `Expr::Try` (`?` operator) to reject use of `?` in functions
    /// that don't return `Result<_, _>` / `Option<_>` — without this, an
    /// unhandled error silently exits with code 0.
    current_fn_return: TypeInfo,
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
            current_generics: Vec::new(),
            enum_defs: std::collections::HashSet::new(),
            current_fn_return: TypeInfo::Unit,
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
                return TypeInfo::UserStruct {
                    name: impl_type.clone(),
                    generic_args: Vec::new(),
                };
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

    /// True if `name` refers to a known user-defined type (struct, enum,
    /// type alias, use-alias) or an in-scope generic parameter / `Self`.
    fn is_known_user_type(&self, name: &str) -> bool {
        if name == "Self" || name == "_" {
            return true;
        }
        // Strip any inline generic-arg suffix (`Pair<i64, i64>` → `Pair`),
        // which `current_impl_type` and a few legacy parser paths still
        // produce as a single string.
        let bare = match name.find('<') {
            Some(i) => &name[..i],
            None => name,
        };
        if self.current_generics.iter().any(|g| g == bare) {
            return true;
        }
        if self.struct_defs.contains_key(bare) || self.enum_defs.contains(bare) {
            return true;
        }
        if self.type_aliases.contains_key(bare) || self.use_aliases.contains_key(bare) {
            return true;
        }
        // Module-qualified lookup (e.g. `database::Record` when inside `database`).
        let module_prefix = self.module_stack.join("::");
        if !module_prefix.is_empty() {
            let qualified = format!("{}::{}", module_prefix, bare);
            if self.struct_defs.contains_key(&qualified)
                || self.enum_defs.contains(&qualified)
                || self.type_aliases.contains_key(&qualified)
            {
                return true;
            }
        }
        false
    }

    /// Walk a TypeInfo and error on any nested `UserStruct(name)` whose name
    /// isn't a known type. Used at type-annotation sites to surface
    /// `let v: Vec<bogus_name> = …` as a clean "unknown type" diagnostic
    /// instead of a downstream "type mismatch".
    fn validate_type_known(&self, ty: &TypeInfo, span: Span) -> Result<(), FerriError> {
        match ty {
            TypeInfo::UserStruct { name, generic_args } => {
                if !self.is_known_user_type(name) {
                    // Specific fix-it for the retired Rust-style width zoo
                    // (i8 .. u64, isize, usize, f32) — point users at the
                    // single replacement they should use instead.
                    let suggestion = match name.as_str() {
                        "i8" | "i16" | "i32" | "i64" | "u16" | "u32" | "u64" | "isize"
                        | "usize" => Some("int"),
                        "u8" => Some("byte"),
                        "f32" | "f64" => Some("float"),
                        _ => None,
                    };
                    let message = match suggestion {
                        Some(repl) => format!(
                            "`{name}` is not an Oxy type — use `{repl}` instead. Oxy has only `int`, `byte`, and `float`."
                        ),
                        None => format!("unknown type `{name}`"),
                    };
                    return Err(FerriError::TypeError {
                        message,
                        line: span.line,
                        column: span.column,
                    });
                }
                for g in generic_args {
                    self.validate_type_known(g, span)?;
                }
                Ok(())
            }
            TypeInfo::Vec(t) | TypeInfo::Option(t) => self.validate_type_known(t, span),
            TypeInfo::HashMap(k, v) | TypeInfo::BTreeMap(k, v) | TypeInfo::Result(k, v) => {
                self.validate_type_known(k, span)?;
                self.validate_type_known(v, span)
            }
            TypeInfo::Array(elem, _) => self.validate_type_known(elem, span),
            TypeInfo::Function { params, ret } => {
                for p in params {
                    self.validate_type_known(p, span)?;
                }
                self.validate_type_known(ret, span)
            }
            _ => Ok(()),
        }
    }

    /// Resolve a TypeAnnotation to TypeInfo, handling Named and Array variants.
    fn resolve_annotation(&self, ann: &TypeAnnotation) -> TypeInfo {
        match ann {
            TypeAnnotation::Named {
                name, generic_args, ..
            } => {
                let head = self.resolve_type(name);
                TypeInfo::apply_generics(head, generic_args).unwrap_or(TypeInfo::Unknown)
            }
            TypeAnnotation::Array { inner, size, .. } => {
                let inner_ty = self.resolve_annotation(inner);
                TypeInfo::Array(Box::new(inner_ty), *size)
            }
        }
    }

    /// Type of the trailing semicolon-less expression in a block, or `Unit`
    /// if the block has no producing tail.
    fn block_tail_type(&mut self, block: &Block) -> Result<TypeInfo, FerriError> {
        let block_env = TypeEnv::child(&self.env);
        let saved = self.env.clone();
        self.env = block_env;
        let mut result = TypeInfo::Unit;
        let body_result = (|| -> Result<(), FerriError> {
            for stmt in &block.stmts {
                if let Stmt::Expr {
                    expr,
                    has_semicolon,
                } = stmt
                {
                    if !has_semicolon {
                        result = self.infer_expr(expr)?;
                        continue;
                    }
                }
                self.check_stmt(stmt, &TypeInfo::Unknown)?;
            }
            Ok(())
        })();
        self.env = saved;
        body_result?;
        Ok(result)
    }

    /// Combine two branch types from an `if`/`match`. `Unknown` and `Unit`
    /// arms are absorbed into the other side; otherwise the two must be
    /// mutually `accepts`-compatible.
    fn unify_branch_types(
        &self,
        a: &TypeInfo,
        b: &TypeInfo,
        kind: &str,
        span: Span,
    ) -> Result<TypeInfo, FerriError> {
        if *a == TypeInfo::Unknown {
            return Ok(b.clone());
        }
        if *b == TypeInfo::Unknown {
            return Ok(a.clone());
        }
        if *a == TypeInfo::Unit {
            return Ok(b.clone());
        }
        if *b == TypeInfo::Unit {
            return Ok(a.clone());
        }
        if a.accepts(b) {
            return Ok(a.clone());
        }
        if b.accepts(a) {
            return Ok(b.clone());
        }
        Err(FerriError::TypeError {
            message: format!(
                "{kind} branches produce incompatible types `{}` and `{}`",
                a.display_name(),
                b.display_name()
            ),
            line: span.line,
            column: span.column,
        })
    }

    /// Substitute generic-parameter names with their resolved types.
    /// `param_names[i]` maps to `arg_types[i]`. The substitution happens on
    /// the raw `TypeAnnotation` so nested generics in field types like
    /// `Vec<T>` get properly recursed through.
    fn substitute_generics(
        &self,
        ann: &TypeAnnotation,
        param_names: &[String],
        arg_types: &[TypeInfo],
    ) -> TypeInfo {
        match ann {
            TypeAnnotation::Named {
                name, generic_args, ..
            } => {
                if let Some(idx) = param_names.iter().position(|p| p == name) {
                    return arg_types.get(idx).cloned().unwrap_or(TypeInfo::Unknown);
                }
                let head = self.resolve_type(name);
                let resolved_args: Vec<TypeInfo> = generic_args
                    .iter()
                    .map(|a| self.substitute_generics(a, param_names, arg_types))
                    .collect();
                match head {
                    TypeInfo::Vec(_) if !resolved_args.is_empty() => {
                        TypeInfo::Vec(Box::new(resolved_args[0].clone()))
                    }
                    TypeInfo::Option(_) if !resolved_args.is_empty() => {
                        TypeInfo::Option(Box::new(resolved_args[0].clone()))
                    }
                    TypeInfo::HashMap(..) if resolved_args.len() >= 2 => TypeInfo::HashMap(
                        Box::new(resolved_args[0].clone()),
                        Box::new(resolved_args[1].clone()),
                    ),
                    TypeInfo::BTreeMap(..) if resolved_args.len() >= 2 => TypeInfo::BTreeMap(
                        Box::new(resolved_args[0].clone()),
                        Box::new(resolved_args[1].clone()),
                    ),
                    TypeInfo::Result(..) if resolved_args.len() >= 2 => TypeInfo::Result(
                        Box::new(resolved_args[0].clone()),
                        Box::new(resolved_args[1].clone()),
                    ),
                    TypeInfo::UserStruct { name, .. } if !resolved_args.is_empty() => {
                        TypeInfo::UserStruct {
                            name,
                            generic_args: resolved_args,
                        }
                    }
                    other => other,
                }
            }
            TypeAnnotation::Array { inner, size, .. } => TypeInfo::Array(
                Box::new(self.substitute_generics(inner, param_names, arg_types)),
                *size,
            ),
        }
    }

    /// Builtin method list for a given concrete type, or None if the type
    /// isn't a builtin we track (UserStruct / Unknown / etc).
    fn builtin_methods_for(&self, ty: &TypeInfo) -> Option<&'static [symbols::MethodInfo]> {
        match ty {
            TypeInfo::Vec(_) => Some(symbols::VEC_METHODS),
            // Fixed-size arrays share Vec's read-only surface but disallow
            // mutators; we reuse VEC_METHODS and reject mutators in the
            // call-site check.
            TypeInfo::Array(..) => Some(symbols::VEC_METHODS),
            TypeInfo::String => Some(symbols::STRING_METHODS),
            TypeInfo::HashMap(..) => Some(symbols::HASHMAP_METHODS),
            TypeInfo::BTreeMap(..) => Some(symbols::BTREEMAP_METHODS),
            TypeInfo::Option(_) | TypeInfo::Result(..) => Some(symbols::OPTION_RESULT_METHODS),
            TypeInfo::Char => Some(symbols::CHAR_METHODS),
            t if t.is_integer() || t.is_float() => Some(symbols::NUMERIC_METHODS),
            _ => None,
        }
    }

    /// True if `method` is callable on `ty`, considering both type-specific
    /// methods and the generic methods available on every value.
    fn method_exists_on(&self, ty: &TypeInfo, method: &str) -> bool {
        if symbols::GENERIC_METHODS.iter().any(|m| m.name == method) {
            return true;
        }
        if let Some(list) = self.builtin_methods_for(ty) {
            if list.iter().any(|m| m.name == method) {
                return true;
            }
        }
        // Collection-like types accept iterator methods directly (Oxy's
        // shortcut: `v.map(...)` instead of `v.iter().map(...).collect()`).
        if matches!(
            ty,
            TypeInfo::Vec(_)
                | TypeInfo::Array(..)
                | TypeInfo::HashMap(..)
                | TypeInfo::BTreeMap(..)
                | TypeInfo::Option(_)
                | TypeInfo::Result(..)
        ) && symbols::ITERATOR_METHODS.iter().any(|m| m.name == method)
        {
            return true;
        }
        false
    }

    /// Per-method element-type validation for parameterized containers, and
    /// a parameterized return type when known. Returns `Ok(Some(ret))` if the
    /// method was recognised and the caller should use `ret` as the call's
    /// type; `Ok(None)` to fall through to the generic return-type table.
    fn check_builtin_method_args(
        &mut self,
        obj_ty: &TypeInfo,
        method: &str,
        args: &[Expr],
        arg_types: &[TypeInfo],
        span: Span,
    ) -> Result<Option<TypeInfo>, FerriError> {
        let check = |expected: &TypeInfo, pos: usize, label: &str| -> Result<(), FerriError> {
            let actual = arg_types.get(pos).cloned().unwrap_or(TypeInfo::Unknown);
            if !expected.accepts(&actual) {
                let aspan = args.get(pos).map(|a| a.span()).unwrap_or(span);
                return Err(FerriError::TypeError {
                    message: format!(
                        "type mismatch in `{method}` {label}: expected `{}`, got `{}`",
                        expected.display_name(),
                        actual.display_name()
                    ),
                    line: aspan.line,
                    column: aspan.column,
                });
            }
            Ok(())
        };
        match obj_ty {
            TypeInfo::Vec(elem) | TypeInfo::Array(elem, _) => match method {
                "push" | "contains" => {
                    check(elem, 0, "argument")?;
                    Ok(Some(if method == "contains" {
                        TypeInfo::Bool
                    } else {
                        TypeInfo::Unit
                    }))
                }
                "insert" => {
                    check(&TypeInfo::I64, 0, "index")?;
                    check(elem, 1, "value")?;
                    Ok(Some(TypeInfo::Unit))
                }
                "pop" | "first" | "last" | "min" | "max" => {
                    Ok(Some(TypeInfo::Option(elem.clone())))
                }
                "get" => {
                    check(&TypeInfo::I64, 0, "index")?;
                    Ok(Some(TypeInfo::Option(elem.clone())))
                }
                "remove" => {
                    check(&TypeInfo::I64, 0, "index")?;
                    Ok(Some(TypeInfo::Option(elem.clone())))
                }
                "iter" => Ok(Some(TypeInfo::Vec(elem.clone()))),
                "len" => Ok(Some(TypeInfo::I64)),
                "is_empty" => Ok(Some(TypeInfo::Bool)),
                "clone" => Ok(Some(obj_ty.clone())),
                _ => Ok(None),
            },
            TypeInfo::HashMap(k, v) | TypeInfo::BTreeMap(k, v) => match method {
                "insert" => {
                    check(k, 0, "key")?;
                    check(v, 1, "value")?;
                    Ok(Some(TypeInfo::Option(v.clone())))
                }
                "get" | "remove" => {
                    check(k, 0, "key")?;
                    Ok(Some(TypeInfo::Option(v.clone())))
                }
                "contains_key" => {
                    check(k, 0, "key")?;
                    Ok(Some(TypeInfo::Bool))
                }
                "keys" => Ok(Some(TypeInfo::Vec(k.clone()))),
                "values" => Ok(Some(TypeInfo::Vec(v.clone()))),
                "len" => Ok(Some(TypeInfo::I64)),
                "is_empty" => Ok(Some(TypeInfo::Bool)),
                _ => Ok(None),
            },
            TypeInfo::Option(inner) => match method {
                "unwrap" | "expect" => Ok(Some((**inner).clone())),
                "unwrap_or" => {
                    check(inner, 0, "default")?;
                    Ok(Some((**inner).clone()))
                }
                "is_some" | "is_none" => Ok(Some(TypeInfo::Bool)),
                _ => Ok(None),
            },
            TypeInfo::Result(t, e) => match method {
                "unwrap" => Ok(Some((**t).clone())),
                "unwrap_err" => Ok(Some((**e).clone())),
                "is_ok" | "is_err" => Ok(Some(TypeInfo::Bool)),
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    /// Fixed-size arrays forbid mutators (push/pop/etc). Returns true if
    /// `method` is a Vec mutator that should be rejected on an Array.
    fn is_array_mutator(&self, method: &str) -> bool {
        matches!(
            method,
            "push"
                | "pop"
                | "insert"
                | "remove"
                | "swap_remove"
                | "clear"
                | "truncate"
                | "resize"
                | "extend"
                | "append"
                | "retain"
                | "drain"
                | "sort"
                | "sort_by"
                | "reverse"
                | "dedup"
        )
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
                Item::Enum(e) => {
                    let qualified = if prefix.is_empty() {
                        e.name.clone()
                    } else {
                        format!("{}::{}", prefix, e.name)
                    };
                    self.enum_defs.insert(qualified);
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
        let mut param_names: Vec<String> =
            f.generic_params.iter().map(|p| p.name.clone()).collect();
        for n in extra_generics {
            param_names.push(n.clone());
        }
        // Generic params resolve to Unknown so call-site accepts() passes for
        // any concrete arg; substitution recurses so `Vec<T>` and `Wrapper<T>`
        // also widen their inner T to Unknown.
        let unknowns: Vec<TypeInfo> = param_names.iter().map(|_| TypeInfo::Unknown).collect();
        f.params
            .iter()
            .map(|p| self.substitute_generics(&p.type_ann, &param_names, &unknowns))
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
        // Track the function's own generic params, plus any inherited from
        // an enclosing impl block, while we walk its body.
        let impl_generics = self
            .current_impl_type
            .as_deref()
            .map(|t| self.struct_generic_names(t))
            .unwrap_or_default();
        let saved_generics = self.current_generics.clone();
        for p in &f.generic_params {
            self.current_generics.push(p.name.clone());
        }
        for g in &impl_generics {
            self.current_generics.push(g.clone());
        }

        let ret_ty = if let Some(ref ann) = f.return_type {
            let is_generic = match ann {
                TypeAnnotation::Named { name, .. } => {
                    self.current_generics.iter().any(|g| g == name)
                }
                TypeAnnotation::Array { .. } => false,
            };
            if is_generic {
                TypeInfo::Unknown
            } else {
                let ty = self.resolve_annotation(ann);
                self.validate_type_known(&ty, ann.span())?;
                ty
            }
        } else {
            TypeInfo::Unit
        };
        self.fn_return_types.insert(f.name.clone(), ret_ty.clone());
        let param_tys = self.resolve_param_types(f, &impl_generics);
        // Validate every declared param type for unknown names.
        for (param, p_ty) in f.params.iter().zip(param_tys.iter()) {
            self.validate_type_known(p_ty, param.span)?;
        }
        self.fn_param_types
            .insert(f.name.clone(), param_tys.clone());

        let fn_env = TypeEnv::child(&self.env);
        for (param, p_ty) in f.params.iter().zip(param_tys.iter()) {
            fn_env.borrow_mut().define(&param.name, p_ty.clone());
        }

        let saved_env = self.env.clone();
        self.env = fn_env;
        let saved_fn_return = std::mem::replace(&mut self.current_fn_return, ret_ty.clone());

        let body_result = (|| -> Result<(), FerriError> {
            for stmt in &f.body.stmts {
                self.check_stmt(stmt, &ret_ty)?;
            }
            Ok(())
        })();

        self.env = saved_env;
        self.current_generics = saved_generics;
        self.current_fn_return = saved_fn_return;
        body_result
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
                    let ty = self.resolve_annotation(ann);
                    self.validate_type_known(&ty, ann.span())?;
                    ty
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
                            declared.display_name(), inferred.display_name()
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
                        return Ok(TypeInfo::user_struct(resolved.clone()));
                    }
                }
                // Try module-qualified struct name
                let resolved = self.resolve_struct_name(name);
                if self.struct_defs.contains_key(&resolved) {
                    return Ok(TypeInfo::user_struct(resolved));
                }
                Ok(TypeInfo::Unknown)
            }

            Expr::BinaryOp {
                op,
                left,
                right,
                span,
            } => {
                let lt = self.infer_expr(left)?;
                let rt = self.infer_expr(right)?;
                let is_num = |t: &TypeInfo| t.is_integer() || t.is_float();
                let known = |t: &TypeInfo| *t != TypeInfo::Unknown;
                // Helper to format a clean operand mismatch error.
                let mk_err = |msg: String| FerriError::TypeError {
                    message: msg,
                    line: span.line,
                    column: span.column,
                };
                match op {
                    BinOp::Eq | BinOp::NotEq => {
                        // Either side may be Unknown (e.g. closure args). Once
                        // both are known we require one to accept the other.
                        if known(&lt) && known(&rt) && !lt.accepts(&rt) && !rt.accepts(&lt) {
                            return Err(mk_err(format!(
                                "cannot compare `{}` and `{}` with `{op}`",
                                lt.name(),
                                rt.name()
                            )));
                        }
                        return Ok(TypeInfo::Bool);
                    }
                    BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                        if known(&lt) && known(&rt) {
                            let both_num = is_num(&lt) && is_num(&rt);
                            let same_scalar = lt == rt
                                && matches!(lt, TypeInfo::String | TypeInfo::Char | TypeInfo::Bool);
                            if !both_num && !same_scalar {
                                return Err(mk_err(format!(
                                    "cannot order `{}` and `{}` with `{op}`",
                                    lt.name(),
                                    rt.name()
                                )));
                            }
                        }
                        return Ok(TypeInfo::Bool);
                    }
                    BinOp::And | BinOp::Or => {
                        if known(&lt) && lt != TypeInfo::Bool {
                            return Err(mk_err(format!(
                                "logical `{op}` requires `bool` operands, left is `{}`",
                                lt.name()
                            )));
                        }
                        if known(&rt) && rt != TypeInfo::Bool {
                            return Err(mk_err(format!(
                                "logical `{op}` requires `bool` operands, right is `{}`",
                                rt.name()
                            )));
                        }
                        return Ok(TypeInfo::Bool);
                    }
                    BinOp::Add => {
                        // String/Char concatenation paths.
                        if lt == TypeInfo::String || rt == TypeInfo::String {
                            return Ok(TypeInfo::String);
                        }
                        if lt == TypeInfo::Char || rt == TypeInfo::Char {
                            return Ok(TypeInfo::String);
                        }
                    }
                    BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                        // Pure arithmetic — String/Char operands are illegal.
                    }
                    BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                        if known(&lt) && !lt.is_integer() {
                            return Err(mk_err(format!(
                                "bitwise `{op}` requires integer operands, left is `{}`",
                                lt.name()
                            )));
                        }
                        if known(&rt) && !rt.is_integer() {
                            return Err(mk_err(format!(
                                "bitwise `{op}` requires integer operands, right is `{}`",
                                rt.name()
                            )));
                        }
                        return Ok(if lt.is_integer() { lt } else { rt });
                    }
                }
                // Arithmetic Add/Sub/Mul/Div/Mod — operands must be numeric,
                // or user-defined structs (which may implement operator
                // overloading via traits).
                let arithmetic_ok = |t: &TypeInfo| {
                    *t == TypeInfo::Unknown || is_num(t) || matches!(t, TypeInfo::UserStruct { .. })
                };
                if !arithmetic_ok(&lt) {
                    return Err(mk_err(format!(
                        "arithmetic `{op}` requires numeric operands, left is `{}`",
                        lt.name()
                    )));
                }
                if !arithmetic_ok(&rt) {
                    return Err(mk_err(format!(
                        "arithmetic `{op}` requires numeric operands, right is `{}`",
                        rt.name()
                    )));
                }
                // User-struct operator overloading: result type is the struct
                // (Add/Sub on Vec2 -> Vec2, etc).
                if let TypeInfo::UserStruct { .. } = &lt {
                    return Ok(lt);
                }
                if let TypeInfo::UserStruct { .. } = &rt {
                    return Ok(rt);
                }
                if matches!(lt, TypeInfo::F64) || matches!(rt, TypeInfo::F64) {
                    Ok(TypeInfo::F64)
                } else {
                    Ok(TypeInfo::I64)
                }
            }

            Expr::UnaryOp {
                op,
                expr: inner,
                span,
            } => {
                let inner_ty = self.infer_expr(inner)?;
                match op {
                    UnaryOp::Neg => {
                        // Allow UserStruct in case the type implements
                        // operator overloading via a Neg trait impl.
                        let ok = inner_ty == TypeInfo::Unknown
                            || inner_ty.is_integer()
                            || inner_ty.is_float()
                            || matches!(inner_ty, TypeInfo::UserStruct { .. });
                        if !ok {
                            return Err(FerriError::TypeError {
                                message: format!(
                                    "unary `-` requires a numeric operand, got `{}`",
                                    inner_ty.name()
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        Ok(inner_ty)
                    }
                    UnaryOp::Not => {
                        if inner_ty != TypeInfo::Unknown && inner_ty != TypeInfo::Bool {
                            return Err(FerriError::TypeError {
                                message: format!(
                                    "unary `!` requires a `bool` operand, got `{}`",
                                    inner_ty.name()
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        Ok(TypeInfo::Bool)
                    }
                    UnaryOp::BitNot => {
                        if inner_ty != TypeInfo::Unknown && !inner_ty.is_integer() {
                            return Err(FerriError::TypeError {
                                message: format!(
                                    "unary `~` requires an integer operand, got `{}`",
                                    inner_ty.name()
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        Ok(inner_ty)
                    }
                }
            }

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
                        let arg_types: Vec<TypeInfo> = args
                            .iter()
                            .map(|a| self.infer_expr(a))
                            .collect::<Result<_, _>>()?;
                        // Built-in constructors: parameterize the wrapper by
                        // the inner argument's inferred type.
                        match name.as_str() {
                            "Some" => {
                                let inner = arg_types.first().cloned().unwrap_or(TypeInfo::Unknown);
                                return Ok(TypeInfo::Option(Box::new(inner)));
                            }
                            "Ok" => {
                                let inner = arg_types.first().cloned().unwrap_or(TypeInfo::Unknown);
                                return Ok(TypeInfo::Result(
                                    Box::new(inner),
                                    Box::new(TypeInfo::Unknown),
                                ));
                            }
                            "Err" => {
                                let inner = arg_types.first().cloned().unwrap_or(TypeInfo::Unknown);
                                return Ok(TypeInfo::Result(
                                    Box::new(TypeInfo::Unknown),
                                    Box::new(inner),
                                ));
                            }
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
                span,
            } => {
                self.infer_expr(condition)?;
                let then_ty = self.block_tail_type(then_block)?;
                let result = if let Some(else_expr) = else_block {
                    let else_ty = self.infer_expr(else_expr)?;
                    self.unify_branch_types(&then_ty, &else_ty, "if", *span)?
                } else {
                    then_ty
                };
                Ok(result)
            }

            Expr::IfLet {
                expr: inner,
                then_block,
                else_block,
                span,
                ..
            } => {
                let _ = self.infer_expr(inner)?;
                let then_ty = self.block_tail_type(then_block)?;
                let result = if let Some(else_expr) = else_block {
                    let else_ty = self.infer_expr(else_expr)?;
                    self.unify_branch_types(&then_ty, &else_ty, "if let", *span)?
                } else {
                    then_ty
                };
                Ok(result)
            }

            Expr::Grouped(inner, _) => self.infer_expr(inner),

            Expr::Repeat { value, count, .. } => {
                let val_ty = self.infer_expr(value)?;
                let _ = self.infer_expr(count)?;
                // Repeat literals are constant-length arrays. If the count is
                // an integer literal we propagate it; otherwise the compiler
                // will already have rejected non-constant counts.
                let n = if let Expr::IntLiteral(n, _, _) = count.as_ref() {
                    *n as usize
                } else {
                    0
                };
                Ok(TypeInfo::Array(Box::new(val_ty), n))
            }

            Expr::Array { elements, span } => {
                let mut elem_types = Vec::with_capacity(elements.len());
                for e in elements {
                    elem_types.push(self.infer_expr(e)?);
                }
                // Determine the array's element type. Pick the first non-Unknown
                // type as the "leader" and require every other element to be
                // compatible with it via the standard accepts rules. A mismatch
                // here means the literal is heterogeneous and we error out so
                // it can't be silently widened to Unknown.
                let mut leader: TypeInfo = TypeInfo::Unknown;
                for (i, t) in elem_types.iter().enumerate() {
                    if leader == TypeInfo::Unknown {
                        leader = t.clone();
                        continue;
                    }
                    if leader.accepts(t) {
                        continue;
                    }
                    if t.accepts(&leader) {
                        leader = t.clone();
                        continue;
                    }
                    let espan = elements[i].span();
                    return Err(FerriError::TypeError {
                        message: format!(
                            "array literal has mixed element types: element {} is `{}`, expected `{}`",
                            i + 1,
                            t.name(),
                            leader.name()
                        ),
                        line: espan.line,
                        column: espan.column,
                    });
                }
                let _ = span;
                Ok(TypeInfo::Array(Box::new(leader), elements.len()))
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
                        if let TypeInfo::UserStruct {
                            name: struct_name, ..
                        } = &obj_ty
                        {
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
                span,
            } => {
                let _ = self.infer_expr(matched)?;
                let mut arm_types: Vec<TypeInfo> = Vec::with_capacity(arms.len());
                for arm in arms {
                    let arm_env = TypeEnv::child(&self.env);
                    let saved = self.env.clone();
                    self.env = arm_env;
                    let arm_ty = self.infer_expr(&arm.body)?;
                    self.env = saved;
                    arm_types.push(arm_ty);
                }
                // Pick the first non-Unit/non-Unknown arm as the leader,
                // then require all other producing-arms to unify with it.
                let mut leader: TypeInfo = TypeInfo::Unit;
                for t in &arm_types {
                    if *t == TypeInfo::Unknown || *t == TypeInfo::Unit {
                        continue;
                    }
                    if leader == TypeInfo::Unit {
                        leader = t.clone();
                        continue;
                    }
                    leader = self.unify_branch_types(&leader, t, "match", *span)?;
                }
                Ok(leader)
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
                if let TypeInfo::UserStruct {
                    name: struct_name, ..
                } = &obj_ty
                {
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
                    // Check for impl-on-primitive (e.g. `impl Doublable for i64`).
                    let primitive_qualified = format!("{}::{}", obj_ty.name(), method);
                    let prim_key = if self.fn_param_types.contains_key(&primitive_qualified) {
                        Some(primitive_qualified)
                    } else {
                        None
                    };
                    if let Some(key) = prim_key {
                        let params = self.fn_param_types.get(&key).cloned().unwrap_or_default();
                        self.check_args_against_params(&params, args, true, method, *span)?;
                        if let Some(ret_ty) = self.fn_return_types.get(&key) {
                            return Ok(ret_ty.clone());
                        }
                    } else {
                        let arg_types: Vec<TypeInfo> = args
                            .iter()
                            .map(|a| self.infer_expr(a))
                            .collect::<Result<_, _>>()?;
                        // Validate the method against the builtin method tables.
                        // Skip when the receiver type is Unknown (we have no
                        // signature to compare against) or a UserStruct (handled
                        // above; impl methods may not be in symbols).
                        if obj_ty != TypeInfo::Unknown
                            && !matches!(obj_ty, TypeInfo::UserStruct { .. })
                            && !self.method_exists_on(&obj_ty, method)
                        {
                            return Err(FerriError::TypeError {
                                message: format!(
                                    "no method `{method}` on type `{}`",
                                    obj_ty.name()
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        // Fixed-size arrays disallow Vec mutators.
                        if matches!(obj_ty, TypeInfo::Array(..)) && self.is_array_mutator(method) {
                            return Err(FerriError::TypeError {
                                message: format!(
                                    "method `{method}` is not available on fixed-size arrays; convert to `Vec` first"
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        // Per-method element-type checks for parameterized
                        // containers (`Vec.push(T)`, `HashMap.insert(K, V)`,
                        // ...). Returns the method's parameterized return type
                        // when known.
                        if let Some(ret) = self
                            .check_builtin_method_args(&obj_ty, method, args, &arg_types, *span)?
                        {
                            return Ok(ret);
                        }
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
                if let TypeInfo::UserStruct {
                    name: struct_name,
                    generic_args,
                } = &obj_ty
                {
                    let resolved = self.resolve_struct_name(struct_name);
                    self.check_field_visible(&resolved, field, *span)?;
                    if let Some(def) = self.struct_defs.get(&resolved) {
                        let generic_param_names: Vec<String> =
                            def.generic_params.iter().map(|p| p.name.clone()).collect();
                        let generic_args_owned = generic_args.clone();
                        let def = def.clone();
                        match &def.kind {
                            StructKind::Named(fields) => {
                                for f in fields {
                                    if f.name == *field {
                                        return Ok(self.substitute_generics(
                                            &f.type_ann,
                                            &generic_param_names,
                                            &generic_args_owned,
                                        ));
                                    }
                                }
                                return Err(FerriError::TypeError {
                                    message: format!("no field `{field}` on struct `{resolved}`"),
                                    line: span.line,
                                    column: span.column,
                                });
                            }
                            StructKind::Tuple(types) => {
                                if let Ok(idx) = field.parse::<usize>() {
                                    if let Some(ann) = types.get(idx) {
                                        return Ok(self.substitute_generics(
                                            ann,
                                            &generic_param_names,
                                            &generic_args_owned,
                                        ));
                                    }
                                    return Err(FerriError::TypeError {
                                        message: format!(
                                            "no field `{field}` on tuple struct `{resolved}`"
                                        ),
                                        line: span.line,
                                        column: span.column,
                                    });
                                }
                                return Ok(TypeInfo::Unknown);
                            }
                            StructKind::Unit => {
                                return Err(FerriError::TypeError {
                                    message: format!(
                                        "no field `{field}` on unit struct `{resolved}`"
                                    ),
                                    line: span.line,
                                    column: span.column,
                                });
                            }
                        }
                    }
                    return Ok(TypeInfo::Unknown);
                }
                // Tuple field access (`.0`, `.1`) is also Expr::FieldAccess
                // with a numeric-looking name. Leave those alone for now.
                if field.chars().all(|c| c.is_ascii_digit()) {
                    return Ok(TypeInfo::Unknown);
                }
                // Builtin types (Vec, String, primitives, ...) have no
                // user-accessible fields. If the receiver type is known and
                // concrete, an unknown field is a compile error.
                if obj_ty != TypeInfo::Unknown && !matches!(obj_ty, TypeInfo::UserStruct { .. }) {
                    return Err(FerriError::TypeError {
                        message: format!("no field `{field}` on type `{}`", obj_ty.name()),
                        line: span.line,
                        column: span.column,
                    });
                }
                Ok(TypeInfo::Unknown)
            }

            Expr::Index { object, index, .. } => {
                let obj_ty = self.infer_expr(object)?;
                let idx_ty = self.infer_expr(index)?;
                let is_range_index = matches!(index.as_ref(), Expr::Range { .. });
                // Sequence indexing requires an integer (or a range for slicing).
                let is_seq = matches!(
                    obj_ty,
                    TypeInfo::Vec(_) | TypeInfo::Array(..) | TypeInfo::String
                );
                if is_seq && !is_range_index && idx_ty != TypeInfo::Unknown && !idx_ty.is_integer()
                {
                    let ispan = index.span();
                    return Err(FerriError::TypeError {
                        message: format!(
                            "cannot index `{}` with `{}`: expected integer",
                            obj_ty.name(),
                            idx_ty.name()
                        ),
                        line: ispan.line,
                        column: ispan.column,
                    });
                }
                if obj_ty == TypeInfo::String {
                    // Range index → String slice; integer index → Char.
                    return Ok(if is_range_index {
                        TypeInfo::String
                    } else {
                        TypeInfo::Char
                    });
                }
                if let TypeInfo::Array(elem, _) = &obj_ty {
                    if is_range_index {
                        return Ok(TypeInfo::Vec(elem.clone()));
                    }
                    return Ok((**elem).clone());
                }
                if let TypeInfo::Vec(elem) = &obj_ty {
                    if is_range_index {
                        return Ok(TypeInfo::Vec(elem.clone()));
                    }
                    return Ok((**elem).clone());
                }
                Ok(TypeInfo::Unknown)
            }

            Expr::Range {
                start, end, span, ..
            } => {
                if let Some(s) = start {
                    let st = self.infer_expr(s)?;
                    if st != TypeInfo::Unknown && !st.is_integer() {
                        return Err(FerriError::TypeError {
                            message: format!("range start must be an integer, got `{}`", st.name()),
                            line: span.line,
                            column: span.column,
                        });
                    }
                }
                if let Some(e) = end {
                    let et = self.infer_expr(e)?;
                    if et != TypeInfo::Unknown && !et.is_integer() {
                        return Err(FerriError::TypeError {
                            message: format!("range end must be an integer, got `{}`", et.name()),
                            line: span.line,
                            column: span.column,
                        });
                    }
                }
                Ok(TypeInfo::I64)
            }

            Expr::StructInit {
                name, fields, span, ..
            } => {
                let resolved = self.resolve_struct_name(name);
                // Pre-collect declared field types AND each field's raw
                // annotation, so we can infer concrete generic-arg types
                // from the supplied values (`Box { value: 5 }` → T = i64).
                let generic_param_names: Vec<String> = self
                    .struct_defs
                    .get(&resolved)
                    .map(|def| def.generic_params.iter().map(|p| p.name.clone()).collect())
                    .unwrap_or_default();
                let decl_field_info: HashMap<String, (TypeAnnotation, TypeInfo)> = self
                    .struct_defs
                    .get(&resolved)
                    .and_then(|def| match &def.kind {
                        StructKind::Named(decl_fields) => Some(
                            decl_fields
                                .iter()
                                .map(|f| {
                                    let ty = match &f.type_ann {
                                        TypeAnnotation::Named { name, .. }
                                            if generic_param_names.contains(name) =>
                                        {
                                            TypeInfo::Unknown
                                        }
                                        ann => self.resolve_annotation(ann),
                                    };
                                    (f.name.clone(), (f.type_ann.clone(), ty))
                                })
                                .collect(),
                        ),
                        _ => None,
                    })
                    .unwrap_or_default();
                // First pass: infer field values, capture generic-arg bindings.
                let mut inferred_generics: Vec<TypeInfo> =
                    vec![TypeInfo::Unknown; generic_param_names.len()];
                let mut field_value_types: Vec<(String, TypeInfo, Span)> =
                    Vec::with_capacity(fields.len());
                for (field_name, f_expr) in fields {
                    self.check_field_visible(&resolved, field_name, *span)?;
                    let val_ty = self.infer_expr(f_expr)?;
                    if let Some((ann, _)) = decl_field_info.get(field_name) {
                        if let TypeAnnotation::Named { name: tname, .. } = ann {
                            if let Some(idx) = generic_param_names.iter().position(|g| g == tname) {
                                if inferred_generics[idx] == TypeInfo::Unknown {
                                    inferred_generics[idx] = val_ty.clone();
                                }
                            }
                        }
                    }
                    field_value_types.push((field_name.clone(), val_ty, f_expr.span()));
                }
                // Second pass: validate each field against the substituted
                // declared type.
                for (field_name, val_ty, fspan) in &field_value_types {
                    if let Some((raw_ann, _)) = decl_field_info.get(field_name) {
                        let decl_ty = self.substitute_generics(
                            raw_ann,
                            &generic_param_names,
                            &inferred_generics,
                        );
                        if !decl_ty.accepts(val_ty) {
                            return Err(FerriError::TypeError {
                                message: format!(
                                    "type mismatch: field `{}.{field_name}` declared as `{}`, got `{}`",
                                    resolved,
                                    decl_ty.display_name(),
                                    val_ty.display_name()
                                ),
                                line: fspan.line,
                                column: fspan.column,
                            });
                        }
                    }
                }
                // If `resolved` names a struct-style enum variant (e.g.
                // `Shape::Rectangle`), the produced value's type is the
                // enclosing enum (`Shape`), not the variant. Without this,
                // `area(Shape::Rectangle { ... })` is rejected because the
                // arg types `Shape` and `Shape::Rectangle` don't match.
                let final_name = match resolved.rsplit_once("::") {
                    Some((parent, _)) if self.enum_defs.contains(parent) => parent.to_string(),
                    _ => resolved,
                };
                Ok(TypeInfo::UserStruct {
                    name: final_name,
                    generic_args: inferred_generics,
                })
            }

            Expr::Try { expr: inner, span } => {
                let inner_ty = self.infer_expr(inner)?;
                // The `?` operator only makes sense in a function whose
                // return type is `Result<_, _>` or `Option<_>`. Otherwise
                // an error/None propagated by `?` would silently vanish
                // off the end of the function — exit 0 with no output.
                let ok_here = matches!(
                    &self.current_fn_return,
                    TypeInfo::Result(..) | TypeInfo::Option(..) | TypeInfo::Unknown
                );
                if !ok_here {
                    return Err(FerriError::TypeError {
                        message: format!(
                            "`?` cannot be used in a function returning `{}`. \
                             The enclosing function must return `Result<_, _>` or \
                             `Option<_>` so `?` has something to propagate into. \
                             Use a `match` on the expression instead, or change \
                             the function signature.",
                            self.current_fn_return.display_name()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
                // The expression being `?`'d must itself be a Result or Option.
                // (Unknown is allowed as a wildcard so we don't false-positive
                // on values we couldn't infer.)
                match &inner_ty {
                    TypeInfo::Result(ok, _) => Ok((**ok).clone()),
                    TypeInfo::Option(inner) => Ok((**inner).clone()),
                    TypeInfo::Unknown => Ok(TypeInfo::Unknown),
                    other => Err(FerriError::TypeError {
                        message: format!(
                            "`?` requires a `Result` or `Option` operand; got `{}`",
                            other.display_name()
                        ),
                        line: span.line,
                        column: span.column,
                    }),
                }
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
            Expr::MacroCall { name, args, .. } => {
                // Infer all args so nested calls / field accesses still get
                // type-checked.
                let arg_types: Vec<TypeInfo> = args
                    .iter()
                    .map(|a| self.infer_expr(a))
                    .collect::<Result<_, _>>()?;
                if name == "vec" {
                    // vec![a, b, c] must be homogeneous (or contain Unknown).
                    let mut leader = TypeInfo::Unknown;
                    for (i, t) in arg_types.iter().enumerate() {
                        if *t == TypeInfo::Unknown {
                            continue;
                        }
                        if leader == TypeInfo::Unknown {
                            leader = t.clone();
                            continue;
                        }
                        if leader.accepts(t) {
                            continue;
                        }
                        if t.accepts(&leader) {
                            leader = t.clone();
                            continue;
                        }
                        let espan = args[i].span();
                        return Err(FerriError::TypeError {
                            message: format!(
                                "`vec!` has mixed element types: element {} is `{}`, expected `{}`",
                                i + 1,
                                t.name(),
                                leader.name()
                            ),
                            line: espan.line,
                            column: espan.column,
                        });
                    }
                    return Ok(TypeInfo::Vec(Box::new(leader)));
                }
                Ok(TypeInfo::Unknown)
            }
            Expr::Path { segments, .. } => {
                let qualified = segments.join("::");
                if let Some(ret) = self.fn_return_types.get(&qualified) {
                    return Ok(ret.clone());
                }
                if self.struct_defs.contains_key(&qualified) {
                    return Ok(TypeInfo::user_struct(qualified));
                }
                // Try through use_aliases for the first segment
                if segments.len() == 2 {
                    if let Some(resolved) = self.use_aliases.get(&segments[0]) {
                        let full = format!("{}::{}", resolved, segments[1]);
                        if self.struct_defs.contains_key(&full) {
                            return Ok(TypeInfo::user_struct(full));
                        }
                    }
                }
                Ok(TypeInfo::Unknown)
            }
            Expr::SelfRef { .. } => {
                if let Some(ref impl_type) = self.current_impl_type {
                    Ok(TypeInfo::user_struct(impl_type.clone()))
                } else {
                    Ok(TypeInfo::Unknown)
                }
            }
            Expr::As {
                expr,
                type_name,
                span,
            } => {
                let _ = self.infer_expr(expr)?;
                let target = TypeInfo::from_name(type_name);
                // `as` is only meaningful for primitive scalar conversions.
                // Anything that came back as `UserStruct` is an unknown name.
                let is_scalar = target.is_integer()
                    || target.is_float()
                    || matches!(target, TypeInfo::Bool | TypeInfo::String | TypeInfo::Char);
                if !is_scalar {
                    return Err(FerriError::TypeError {
                        message: format!(
                            "`as` cast to unknown type `{type_name}`; only numeric, bool, String, and char are supported"
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
                Ok(target)
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
