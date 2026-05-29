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

mod check_expr;
mod check_item;
mod check_stmt;
mod collect;
mod resolve;

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
    /// The result of calling an `async fn`. `.await` unwraps it.
    Future(Box<TypeInfo>),
    /// The result of `spawn(|| expr)`. `.await` unwraps it.
    JoinHandle(Box<TypeInfo>),
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
            TypeInfo::Future(_) => "Future",
            TypeInfo::JoinHandle(_) => "JoinHandle",
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
            TypeInfo::Future(t) => format!("Future<{}>", t.display_name()),
            TypeInfo::JoinHandle(t) => format!("JoinHandle<{}>", t.display_name()),
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
        // Tuple type annotations `(T1, T2, ...)` — we don't have a
        // TypeInfo::Tuple variant, so just type-check them loosely as
        // Unknown. Runtime tuples Just Work (Value::Tuple); the only thing
        // this costs is precise element-type tracking, which the existing
        // `(name)` annotation never expressed anyway.
        if name.starts_with('(') && name.ends_with(')') && name != "()" {
            return TypeInfo::Unknown;
        }
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
        // Future<T> and JoinHandle<T>: wrapper types that unwrap to their
        // inner type via .await. Accept if inner types agree.
        if let (TypeInfo::Future(st), TypeInfo::Future(ot)) = (self, other) {
            return st.accepts(ot);
        }
        if let (TypeInfo::JoinHandle(st), TypeInfo::JoinHandle(ot)) = (self, other) {
            return st.accepts(ot);
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
    /// For generic functions: qualified name → (generic_param_names, original_param_typeanns, original_return_typeann).
    /// Used at call sites to enforce that the same generic param always binds
    /// to a consistent concrete type across all argument positions, and to
    /// substitute concrete types into the return type.
    fn_generic_info: HashMap<String, (Vec<String>, Vec<TypeAnnotation>, Option<TypeAnnotation>)>,
    /// Tracks nesting depth of loop constructs (while, for, loop) for
    /// detecting break/continue used outside any loop.
    loop_depth: usize,
    /// Qualified function name → FnDef (for visibility checking).
    fn_defs: HashMap<String, FnDef>,
    /// Qualified module path → visibility (for module visibility checking).
    module_vis: HashMap<String, Visibility>,
    /// Re-export aliases: module::local_name → source_path (from `pub use` inside modules).
    reexports: HashMap<String, String>,
    /// Modules brought into scope via `use module::*`. A bare call that resolves
    /// to `module::name` through a glob is visibility-checked like any other
    /// path, so a glob can't smuggle in a private item.
    glob_imports: Vec<String>,
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
            fn_generic_info: HashMap::new(),
            loop_depth: 0,
            fn_defs: HashMap::new(),
            module_vis: HashMap::new(),
            reexports: HashMap::new(),
            glob_imports: Vec::new(),
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    pub fn check_program(&mut self, program: &Program) -> Result<(), FerriError> {
        // First pass: collect struct defs, type aliases, and use aliases
        self.collect_defs(&program.items, "");

        // Second pass: register function return types
        self.collect_fn_types(&program.items, "");

        // Third pass: resolve pub use re-exports so external callers can
        // find re-exported names under the module's qualified path.
        self.resolve_reexports(&program.items, "");

        // Fourth pass: check each item
        for item in &program.items {
            self.check_item(item)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
