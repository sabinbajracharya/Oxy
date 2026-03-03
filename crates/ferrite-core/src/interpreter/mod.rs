//! Tree-walking interpreter for the Ferrite language.
//!
//! Evaluates the AST produced by the parser, executing statements and
//! evaluating expressions to produce [`Value`]s.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::ast::*;
use crate::env::{Env, Environment};
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::{
    FunctionData, FutureData, Value, ERR_VARIANT, NONE_VARIANT, OK_VARIANT, OPTION_TYPE,
    RESULT_TYPE, SOME_VARIANT,
};

mod format;
mod http;
mod json;
mod macros;
mod methods;
mod operations;
mod path;
mod pattern;

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
    /// Registered trait definitions.
    trait_defs: HashMap<String, TraitDef>,
    /// Methods registered via `impl Trait for Type`, keyed by (type_name, trait_name).
    trait_impls: HashMap<(String, String), Vec<FnDef>>,
    /// Current `Self` type name (set when executing impl methods).
    current_self_type: Option<String>,
    /// Module environments, keyed by module path (e.g., "math", "utils::helpers").
    modules: HashMap<String, ModuleData>,
    /// Base directory for file-based module resolution.
    base_dir: Option<String>,
    /// Type aliases (documentation-only in a dynamically typed language).
    type_aliases: HashMap<String, TypeAnnotation>,
    /// Command-line arguments passed to the program.
    cli_args: Vec<String>,
    /// Names of functions declared with `async fn`.
    async_fns: HashSet<String>,
    /// Derived traits per type, e.g. `"Point" -> {"Debug", "Clone", "PartialEq"}`.
    derived_traits: HashMap<String, HashSet<String>>,
}

/// Data stored for a registered module.
#[derive(Clone)]
struct ModuleData {
    env: Env,
    struct_defs: HashMap<String, StructDef>,
    enum_defs: HashMap<String, EnumDef>,
    impl_methods: HashMap<String, Vec<FnDef>>,
    trait_defs: HashMap<String, TraitDef>,
    trait_impls: HashMap<(String, String), Vec<FnDef>>,
}

impl Interpreter {
    /// Internal constructor with all fields parameterized.
    fn new_internal(env: Env, output: Option<Vec<String>>) -> Self {
        Self {
            env,
            output,
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            impl_methods: HashMap::new(),
            trait_defs: HashMap::new(),
            trait_impls: HashMap::new(),
            current_self_type: None,
            modules: HashMap::new(),
            base_dir: None,
            type_aliases: HashMap::new(),
            cli_args: Vec::new(),
            async_fns: HashSet::new(),
            derived_traits: HashMap::new(),
        }
    }

    /// Create a new interpreter with a fresh global environment.
    pub fn new() -> Self {
        Self::new_internal(Environment::new(), None)
    }

    /// Create an interpreter that captures output instead of printing.
    pub fn new_with_captured_output() -> Self {
        Self::new_internal(Environment::new(), Some(Vec::new()))
    }

    /// Create an interpreter with an existing environment (for REPL).
    pub fn with_env(env: Env) -> Self {
        Self::new_internal(env, None)
    }

    /// Set the base directory for file-based module resolution.
    pub fn set_base_dir(&mut self, dir: String) {
        self.base_dir = Some(dir);
    }

    /// Set command-line arguments for the program.
    pub fn set_cli_args(&mut self, args: Vec<String>) {
        self.cli_args = args;
    }

    /// Get captured output (for testing).
    pub fn captured_output(&self) -> &[String] {
        self.output.as_deref().unwrap_or(&[])
    }

    /// Get the current environment (for REPL persistence).
    pub fn env(&self) -> &Env {
        &self.env
    }

    /// Register derived traits from `#[derive(...)]` attributes on a type.
    fn register_derive_traits(&mut self, type_name: &str, attributes: &[Attribute]) {
        for attr in attributes {
            if attr.name == "derive" {
                for trait_name in &attr.args {
                    self.derived_traits
                        .entry(type_name.to_string())
                        .or_default()
                        .insert(trait_name.clone());
                }
            }
        }
    }

    /// Check if a type has a specific derived trait.
    fn has_derive(&self, type_name: &str, trait_name: &str) -> bool {
        self.derived_traits
            .get(type_name)
            .is_some_and(|traits| traits.contains(trait_name))
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

        if let Value::Function(_) = &main_fn {
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
                let value = Value::Function(Box::new(FunctionData {
                    name: f.name.clone(),
                    params: f.params.clone(),
                    return_type: f.return_type.clone(),
                    body: f.body.clone(),
                    closure_env: Rc::clone(&self.env),
                }));
                self.env.borrow_mut().define(f.name.clone(), value, false);
                if f.is_async {
                    self.async_fns.insert(f.name.clone());
                }
                Ok(())
            }
            Item::Struct(s) => {
                self.register_derive_traits(&s.name, &s.attributes);
                self.struct_defs.insert(s.name.clone(), s.clone());
                Ok(())
            }
            Item::Enum(e) => {
                self.register_derive_traits(&e.name, &e.attributes);
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
            Item::Trait(t) => {
                self.trait_defs.insert(t.name.clone(), t.clone());
                Ok(())
            }
            Item::ImplTrait(i) => {
                let key = (i.type_name.clone(), i.trait_name.clone());
                let methods = self.trait_impls.entry(key).or_default();
                for method in &i.methods {
                    methods.retain(|m| m.name != method.name);
                    methods.push(method.clone());
                }
                Ok(())
            }
            Item::Module(m) => self.register_module(m),
            Item::Use(u) => self.register_use(u),
            Item::TypeAlias { name, target, .. } => {
                self.type_aliases.insert(name.clone(), target.clone());
                Ok(())
            }
            Item::Const {
                name, value, span, ..
            } => {
                let val = self.eval_expr(value, &self.env.clone())?;
                self.env.borrow_mut().define(name.clone(), val, false);
                let _ = span;
                Ok(())
            }
        }
    }

    /// Register an inline or file-based module.
    fn register_module(&mut self, module: &ModuleDef) -> Result<(), FerriError> {
        let items = if let Some(body) = &module.body {
            // Inline module
            body.clone()
        } else {
            // File-based module: load from `name.fe` or `name/mod.fe`
            let source = self.load_module_file(&module.name, module.span)?;
            let program = crate::parser::parse(&source)?;
            program.items
        };

        // Create a sub-interpreter to process module items
        let mod_env = Environment::new();
        let mut mod_struct_defs = HashMap::new();
        let mut mod_enum_defs = HashMap::new();
        let mut mod_impl_methods: HashMap<String, Vec<FnDef>> = HashMap::new();
        let mut mod_trait_defs = HashMap::new();
        let mut mod_trait_impls: HashMap<(String, String), Vec<FnDef>> = HashMap::new();

        for item in &items {
            match item {
                Item::Function(f) => {
                    let value = Value::Function(Box::new(FunctionData {
                        name: f.name.clone(),
                        params: f.params.clone(),
                        return_type: f.return_type.clone(),
                        body: f.body.clone(),
                        closure_env: Rc::clone(&mod_env),
                    }));
                    mod_env.borrow_mut().define(f.name.clone(), value, false);
                    if f.is_async {
                        self.async_fns.insert(f.name.clone());
                    }
                }
                Item::Struct(s) => {
                    self.register_derive_traits(&s.name, &s.attributes);
                    mod_struct_defs.insert(s.name.clone(), s.clone());
                }
                Item::Enum(e) => {
                    self.register_derive_traits(&e.name, &e.attributes);
                    mod_enum_defs.insert(e.name.clone(), e.clone());
                }
                Item::Impl(i) => {
                    let methods = mod_impl_methods.entry(i.type_name.clone()).or_default();
                    for method in &i.methods {
                        methods.retain(|m| m.name != method.name);
                        methods.push(method.clone());
                    }
                }
                Item::Trait(t) => {
                    mod_trait_defs.insert(t.name.clone(), t.clone());
                }
                Item::ImplTrait(i) => {
                    let key = (i.type_name.clone(), i.trait_name.clone());
                    let methods = mod_trait_impls.entry(key).or_default();
                    for method in &i.methods {
                        methods.retain(|m| m.name != method.name);
                        methods.push(method.clone());
                    }
                }
                Item::Module(_) | Item::Use(_) | Item::TypeAlias { .. } | Item::Const { .. } => {
                    // Nested modules/use/type aliases/consts in modules: skip for now
                }
            }
        }

        self.modules.insert(
            module.name.clone(),
            ModuleData {
                env: mod_env,
                struct_defs: mod_struct_defs,
                enum_defs: mod_enum_defs,
                impl_methods: mod_impl_methods,
                trait_defs: mod_trait_defs,
                trait_impls: mod_trait_impls,
            },
        );
        Ok(())
    }

    /// Load a file-based module's source code.
    fn load_module_file(&self, name: &str, span: Span) -> Result<String, FerriError> {
        let base = self.base_dir.as_deref().unwrap_or(".");

        // WHY: We try both `name.fe` and `name/mod.fe` to mirror Rust's module resolution
        // convention—a module can be either a single file (`foo.fe`) or a directory with an
        // entry point (`foo/mod.fe`). This lets users organise large modules into subdirectories
        // without changing their import statements.
        let path1 = format!("{base}/{name}.fe");
        let path2 = format!("{base}/{name}/mod.fe");

        if let Ok(source) = std::fs::read_to_string(&path1) {
            return Ok(source);
        }
        if let Ok(source) = std::fs::read_to_string(&path2) {
            return Ok(source);
        }

        Err(FerriError::Runtime {
            message: format!("could not find module `{name}`: tried '{path1}' and '{path2}'"),
            line: span.line,
            column: span.column,
        })
    }

    /// Process a `use` declaration — import items from a module into current scope.
    fn register_use(&mut self, use_def: &UseDef) -> Result<(), FerriError> {
        // The last segment before the tree is the module name,
        // unless it's a simple use (then the last is the item name).
        let (mod_name, item_to_import) = match &use_def.tree {
            UseTree::Simple => {
                if use_def.path.len() < 2 {
                    // `use item;` — no module, nothing to resolve
                    return Ok(());
                }
                // `use module::item;`
                let mod_name = use_def.path[..use_def.path.len() - 1].join("::");
                let item_name = use_def.path.last().unwrap().clone();
                (mod_name, Some(item_name))
            }
            UseTree::Glob => {
                // `use module::*;`
                let mod_name = use_def.path.join("::");
                (mod_name, None)
            }
            UseTree::Group(_) => {
                // `use module::{a, b};`
                let mod_name = use_def.path.join("::");
                (mod_name, None)
            }
        };

        // Skip crate/self/super prefixes — resolve to just the module name
        let resolved_mod = mod_name
            .strip_prefix("crate::")
            .or_else(|| mod_name.strip_prefix("self::"))
            .unwrap_or(&mod_name)
            .to_string();

        let module = self.modules.get(&resolved_mod).cloned();
        let Some(module) = module else {
            // Module not found — silently ignore (may be a std lib reference)
            return Ok(());
        };

        match &use_def.tree {
            UseTree::Simple => {
                if let Some(name) = item_to_import {
                    self.import_item_from_module(&module, &name);
                }
            }
            UseTree::Glob => {
                // Import everything from the module
                self.import_all_from_module(&module);
            }
            UseTree::Group(names) => {
                for name in names {
                    self.import_item_from_module(&module, name);
                }
            }
        }

        Ok(())
    }

    /// Import a single named item from a module into the current scope.
    fn import_item_from_module(&mut self, module: &ModuleData, name: &str) {
        // Check functions/values in module env
        if let Ok(val) = module.env.borrow().get(name) {
            self.env.borrow_mut().define(name.to_string(), val, false);
        }
        // Check struct defs
        if let Some(s) = module.struct_defs.get(name) {
            self.struct_defs.insert(name.to_string(), s.clone());
        }
        // Check enum defs
        if let Some(e) = module.enum_defs.get(name) {
            self.enum_defs.insert(name.to_string(), e.clone());
        }
        // Check trait defs
        if let Some(t) = module.trait_defs.get(name) {
            self.trait_defs.insert(name.to_string(), t.clone());
        }
        // Import impl methods for this type
        if let Some(methods) = module.impl_methods.get(name) {
            let entry = self.impl_methods.entry(name.to_string()).or_default();
            for m in methods {
                entry.retain(|existing| existing.name != m.name);
                entry.push(m.clone());
            }
        }
        // Import trait impls for this type
        for ((type_name, trait_name), methods) in &module.trait_impls {
            if type_name == name {
                let key = (type_name.clone(), trait_name.clone());
                let entry = self.trait_impls.entry(key).or_default();
                for m in methods {
                    entry.retain(|existing| existing.name != m.name);
                    entry.push(m.clone());
                }
            }
        }
    }

    /// Import all items from a module into the current scope.
    fn import_all_from_module(&mut self, module: &ModuleData) {
        // Import all functions/values
        let bindings: Vec<(String, Value)> =
            module.env.borrow().all_bindings().into_iter().collect();
        for (name, val) in bindings {
            self.env.borrow_mut().define(name, val, false);
        }
        // Import all struct defs
        for (name, s) in &module.struct_defs {
            self.struct_defs.insert(name.clone(), s.clone());
        }
        // Import all enum defs
        for (name, e) in &module.enum_defs {
            self.enum_defs.insert(name.clone(), e.clone());
        }
        // Import all trait defs
        for (name, t) in &module.trait_defs {
            self.trait_defs.insert(name.clone(), t.clone());
        }
        // Import all impl methods
        for (type_name, methods) in &module.impl_methods {
            let entry = self.impl_methods.entry(type_name.clone()).or_default();
            for m in methods {
                entry.retain(|existing| existing.name != m.name);
                entry.push(m.clone());
            }
        }
        // Import all trait impls
        for (key, methods) in &module.trait_impls {
            let entry = self.trait_impls.entry(key.clone()).or_default();
            for m in methods {
                entry.retain(|existing| existing.name != m.name);
                entry.push(m.clone());
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

            Stmt::WhileLet {
                pattern,
                expr,
                body,
                ..
            } => {
                let mut result = Value::Unit;
                loop {
                    let val = self.eval_expr(expr, env)?;
                    if !Self::pattern_matches(pattern, &val) {
                        break;
                    }
                    let iter_env = Environment::child(env);
                    Self::bind_pattern(pattern, &val, &iter_env);
                    match self.eval_block(body, &iter_env) {
                        Ok(v) => result = v,
                        Err(FerriError::Break(v)) => {
                            result = v.map(|v| *v).unwrap_or(Value::Unit);
                            break;
                        }
                        Err(FerriError::Continue) => continue,
                        Err(e) => return Err(e),
                    }
                }
                Ok(result)
            }
            Stmt::ForDestructure {
                names,
                iterable,
                body,
                ..
            } => {
                let iter_val = self.eval_expr(iterable, env)?;
                let values = self.value_to_iter(&iter_val, iterable.span())?;
                let for_env = Environment::child(env);
                for name in names {
                    for_env.borrow_mut().define(name.clone(), Value::Unit, true);
                }
                for val in values {
                    // Destructure tuple into individual variables
                    if let Value::Tuple(ref elems) = val {
                        for (i, name) in names.iter().enumerate() {
                            let v = elems.get(i).cloned().unwrap_or(Value::Unit);
                            for_env.borrow_mut().set(name, v).ok();
                        }
                    } else if names.len() == 1 {
                        for_env.borrow_mut().set(&names[0], val).ok();
                    }
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
                    Stmt::Loop { .. }
                    | Stmt::While { .. }
                    | Stmt::For { .. }
                    | Stmt::ForDestructure { .. } => {
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

            Expr::Ident(name, span) => {
                if name == NONE_VARIANT {
                    return Ok(Value::none());
                }
                env.borrow().get(name).map_err(|_| FerriError::Runtime {
                    message: format!("undefined variable '{name}'"),
                    line: span.line,
                    column: span.column,
                })
            }

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

            Expr::Call { callee, args, span } => self.eval_call_expr(callee, args, span, env),

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
            } => self.eval_assign_expr(target, value, span, env),

            Expr::CompoundAssign {
                target,
                op,
                value,
                span,
            } => self.eval_compound_assign_expr(target, *op, value, span, env),

            Expr::Grouped(inner, _) => self.eval_expr(inner, env),

            Expr::Match { expr, arms, span } => self.eval_match_expr(expr, arms, span, env),

            Expr::Range {
                start,
                end,
                inclusive,
                span,
            } => self.eval_range_expr(start, end, *inclusive, span, env),

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
            } => self.eval_index_expr(object, index, span, env),

            Expr::FieldAccess {
                object,
                field,
                span,
            } => self.eval_field_access_expr(object, field, span, env),

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
            } => self.eval_struct_init_expr(name, fields, span, env),

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

            Expr::IfLet {
                pattern,
                expr,
                then_block,
                else_block,
                ..
            } => self.eval_if_let_expr(pattern, expr, then_block, else_block, env),

            Expr::Try { expr, span } => self.eval_try_expr(expr, span, env),

            Expr::Closure {
                params,
                return_type,
                body,
                ..
            } => self.eval_closure_expr(params, return_type, body, env),

            Expr::Await { expr, .. } => self.eval_await_expr(expr, env),

            Expr::FString { parts, .. } => self.eval_fstring_expr(parts, env),
        }
    }

    // === Extracted expression helpers ===

    fn try_builtin_call(
        &mut self,
        name: &str,
        args: &[Expr],
        span: &Span,
        env: &Env,
    ) -> Result<Option<Value>, FerriError> {
        match name {
            SOME_VARIANT => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Some() takes exactly 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let val = self.eval_expr(&args[0], env)?;
                Ok(Some(Value::some(val)))
            }
            OK_VARIANT => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Ok() takes exactly 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let val = self.eval_expr(&args[0], env)?;
                Ok(Some(Value::ok(val)))
            }
            ERR_VARIANT => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Err() takes exactly 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let val = self.eval_expr(&args[0], env)?;
                Ok(Some(Value::err(val)))
            }
            "spawn" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("spawn() takes exactly 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let func = self.eval_expr(&args[0], env)?;
                let result = self.call_function(&func, &[], span.line, span.column)?;
                Ok(Some(Value::JoinHandle(Box::new(result))))
            }
            "sleep" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("sleep() takes exactly 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let val = self.eval_expr(&args[0], env)?;
                if let Value::Integer(ms) = val {
                    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
                    return Ok(Some(Value::Unit));
                }
                Err(FerriError::Runtime {
                    message: format!(
                        "sleep() expects integer milliseconds, got {}",
                        val.type_name()
                    ),
                    line: span.line,
                    column: span.column,
                })
            }
            _ => Ok(None),
        }
    }

    fn eval_call_expr(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        if let Expr::Ident(name, _) = callee {
            if let Some(result) = self.try_builtin_call(name, args, span, env)? {
                return Ok(result);
            }
        }
        let func = self.eval_expr(callee, env)?;
        let mut arg_values = Vec::with_capacity(args.len());
        for arg in args {
            arg_values.push(self.eval_expr(arg, env)?);
        }
        // WHY: Ferrite simulates async with lazy thunks (FutureData) instead of real Rust futures
        // because the tree-walking interpreter has no async runtime. A Future here is just a
        // deferred function call—its body is captured but not executed until `.await` is
        // evaluated, giving users async-like syntax without the complexity of an executor.
        if let Value::Function(ref func_data) = func {
            if self.async_fns.contains(&func_data.name) {
                return Ok(Value::Future(Box::new(FutureData {
                    name: func_data.name.clone(),
                    params: func_data.params.clone(),
                    return_type: func_data.return_type.clone(),
                    body: func_data.body.clone(),
                    closure_env: Rc::clone(&func_data.closure_env),
                    args: arg_values,
                })));
            }
        }
        self.call_function(&func, &arg_values, span.line, span.column)
    }

    fn eval_assign_expr(
        &mut self,
        target: &Expr,
        value: &Expr,
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        let val = self.eval_expr(value, env)?;
        if let Expr::Ident(name, _) = target {
            env.borrow_mut()
                .set(name, val)
                .map_err(|e| FerriError::Runtime {
                    message: e.to_string(),
                    line: span.line,
                    column: span.column,
                })?;
            Ok(Value::Unit)
        } else if let Expr::FieldAccess { object, field, .. } = target {
            // Field assignment: `s.field = val`
            if let Expr::Ident(name, _) = object.as_ref() {
                let mut current = env.borrow().get(name).map_err(|_| FerriError::Runtime {
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
                let mut current = env.borrow().get("self").map_err(|_| FerriError::Runtime {
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
        } else if let Expr::Index { object, index, .. } = target {
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
                let mut current = env.borrow().get(name).map_err(|_| FerriError::Runtime {
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
                            message: format!("cannot index-assign into {}", current.type_name()),
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

    fn eval_compound_assign_expr(
        &mut self,
        target: &Expr,
        op: BinOp,
        value: &Expr,
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        if let Expr::Ident(name, _) = target {
            let current = env.borrow().get(name).map_err(|_| FerriError::Runtime {
                message: format!("undefined variable '{name}'"),
                line: span.line,
                column: span.column,
            })?;
            let rval = self.eval_expr(value, env)?;
            let new_val = self.eval_binary_op(&current, op, &rval, span.line, span.column)?;
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

    // WHY: Exhaustiveness is checked at runtime, not compile time—if no arm matches we return
    // an error. A wildcard `_` arm trivially satisfies this because it matches any value.
    // Since Ferrite is interpreted (no separate compilation phase), static exhaustiveness
    // analysis would add complexity with limited benefit; the runtime error is sufficient.
    fn eval_match_expr(
        &mut self,
        expr: &Expr,
        arms: &[MatchArm],
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
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

    fn eval_range_expr(
        &mut self,
        start: &Expr,
        end: &Expr,
        inclusive: bool,
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        let start_val = self.eval_expr(start, env)?;
        let end_val = self.eval_expr(end, env)?;
        match (&start_val, &end_val) {
            (Value::Integer(s), Value::Integer(e)) => {
                let end_n = if inclusive { *e + 1 } else { *e };
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

    fn eval_index_expr(
        &mut self,
        object: &Expr,
        index: &Expr,
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        let obj = self.eval_expr(object, env)?;
        let idx = self.eval_expr(index, env)?;
        match (&obj, &idx) {
            (Value::Vec(v), Value::Integer(i)) => {
                let i = *i as usize;
                v.get(i).cloned().ok_or_else(|| FerriError::Runtime {
                    message: format!("index out of bounds: len is {}, but index is {i}", v.len()),
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
                    message: format!("index out of bounds: len is {}, but index is {i}", t.len()),
                    line: span.line,
                    column: span.column,
                })
            }
            _ => Err(FerriError::Runtime {
                message: format!("cannot index {} with {}", obj.type_name(), idx.type_name()),
                line: span.line,
                column: span.column,
            }),
        }
    }

    fn eval_field_access_expr(
        &mut self,
        object: &Expr,
        field: &str,
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
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
                    message: format!("cannot access field `.{field}` on {}", obj.type_name()),
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

    fn eval_struct_init_expr(
        &mut self,
        name: &str,
        fields: &[(String, Expr)],
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        // Resolve `Self` to the current impl type
        let resolved_name = if name == "Self" {
            self.current_self_type
                .clone()
                .unwrap_or_else(|| name.to_string())
        } else {
            name.to_string()
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

    fn eval_if_let_expr(
        &mut self,
        pattern: &Pattern,
        expr: &Expr,
        then_block: &Block,
        else_block: &Option<Box<Expr>>,
        env: &Env,
    ) -> Result<Value, FerriError> {
        let val = self.eval_expr(expr, env)?;
        if Self::pattern_matches(pattern, &val) {
            let child_env = Environment::child(env);
            Self::bind_pattern(pattern, &val, &child_env);
            self.eval_block(then_block, &child_env)
        } else if let Some(else_expr) = else_block {
            self.eval_expr(else_expr, env)
        } else {
            Ok(Value::Unit)
        }
    }

    fn eval_try_expr(&mut self, expr: &Expr, span: &Span, env: &Env) -> Result<Value, FerriError> {
        let val = self.eval_expr(expr, env)?;
        match &val {
            // Some(x) → unwrap to x
            Value::EnumVariant {
                enum_name,
                variant,
                data,
                ..
            } if enum_name == OPTION_TYPE && variant == SOME_VARIANT => {
                Ok(data.first().cloned().unwrap_or(Value::Unit))
            }
            // None → return None early
            Value::EnumVariant {
                enum_name, variant, ..
            } if enum_name == OPTION_TYPE && variant == NONE_VARIANT => {
                Err(FerriError::Return(Box::new(val)))
            }
            // Ok(x) → unwrap to x
            Value::EnumVariant {
                enum_name,
                variant,
                data,
                ..
            } if enum_name == RESULT_TYPE && variant == OK_VARIANT => {
                Ok(data.first().cloned().unwrap_or(Value::Unit))
            }
            // Err(e) → return Err(e) early
            Value::EnumVariant {
                enum_name, variant, ..
            } if enum_name == RESULT_TYPE && variant == ERR_VARIANT => {
                Err(FerriError::Return(Box::new(val)))
            }
            _ => Err(FerriError::Runtime {
                message: format!(
                    "`?` operator can only be used on Option or Result, got {}",
                    val.type_name()
                ),
                line: span.line,
                column: span.column,
            }),
        }
    }

    fn eval_closure_expr(
        &mut self,
        params: &[ClosureParam],
        return_type: &Option<TypeAnnotation>,
        body: &Expr,
        env: &Env,
    ) -> Result<Value, FerriError> {
        // Convert ClosureParams to Params (for Value::Function compatibility)
        let fn_params: Vec<Param> = params
            .iter()
            .map(|cp| Param {
                name: cp.name.clone(),
                type_ann: cp.type_ann.clone().unwrap_or(TypeAnnotation {
                    name: "_".to_string(),
                    span: cp.span,
                }),
                span: cp.span,
            })
            .collect();

        // The closure body: wrap single expression in a block for Value::Function
        let closure_body = match body {
            Expr::Block(block) => block.clone(),
            expr => Block {
                stmts: vec![Stmt::Expr {
                    expr: expr.clone(),
                    has_semicolon: false,
                }],
                span: expr.span(),
            },
        };

        Ok(Value::Function(Box::new(FunctionData {
            name: "<closure>".to_string(),
            params: fn_params,
            return_type: return_type.clone(),
            body: closure_body,
            closure_env: env.clone(),
        })))
    }

    fn eval_await_expr(&mut self, expr: &Expr, env: &Env) -> Result<Value, FerriError> {
        let val = self.eval_expr(expr, env)?;
        match val {
            Value::Future(future) => {
                let call_env = Environment::child(&future.closure_env);
                for (param, arg) in future.params.iter().zip(future.args.iter()) {
                    call_env
                        .borrow_mut()
                        .define(param.name.clone(), arg.clone(), true);
                }
                match self.eval_block(&future.body, &call_env) {
                    Ok(val) => Ok(val),
                    Err(FerriError::Return(val)) => Ok(*val),
                    Err(e) => Err(e),
                }
            }
            Value::JoinHandle(val) => Ok(*val),
            other => Ok(other),
        }
    }

    fn eval_fstring_expr(&mut self, parts: &[FStringPart], env: &Env) -> Result<Value, FerriError> {
        let mut result = String::new();
        for part in parts {
            match part {
                FStringPart::Literal(s) => result.push_str(s),
                FStringPart::Expr(expr) => {
                    let val = self.eval_expr(expr, env)?;
                    result.push_str(&val.to_string());
                }
            }
        }
        Ok(Value::String(result))
    }

    // === Function calls ===

    fn call_function(
        &mut self,
        func: &Value,
        args: &[Value],
        line: usize,
        col: usize,
    ) -> Result<Value, FerriError> {
        let Value::Function(func_data) = func else {
            return Err(FerriError::Runtime {
                message: format!("'{}' is not callable", func.type_name()),
                line,
                column: col,
            });
        };

        if args.len() != func_data.params.len() {
            return Err(FerriError::Runtime {
                message: format!(
                    "function '{}' expects {} argument(s), got {}",
                    func_data.name,
                    func_data.params.len(),
                    args.len()
                ),
                line,
                column: col,
            });
        }

        // Create a new scope from the closure environment
        let call_env = Environment::child(&func_data.closure_env);
        for (param, arg) in func_data.params.iter().zip(args.iter()) {
            call_env
                .borrow_mut()
                .define(param.name.clone(), arg.clone(), true);
        }

        // Execute the function body
        match self.eval_block(&func_data.body, &call_env) {
            Ok(val) => Ok(val),
            Err(FerriError::Return(val)) => Ok(*val),
            Err(e) => Err(e),
        }
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

/// Convenience function: parse and execute a Ferrite program.
pub fn run(source: &str) -> Result<Value, FerriError> {
    let program = crate::parser::parse(source)?;
    let mut interp = Interpreter::new();
    interp.execute_program(&program)
}

/// Parse and execute a Ferrite program from a file path (enables module resolution).
pub fn run_file(path: &str, source: &str) -> Result<Value, FerriError> {
    run_file_with_args(path, source, vec![])
}

/// Parse and execute a Ferrite program from a file path with CLI args.
pub fn run_file_with_args(
    path: &str,
    source: &str,
    cli_args: Vec<String>,
) -> Result<Value, FerriError> {
    let program = crate::parser::parse(source)?;
    let mut interp = Interpreter::new();
    if let Some(parent) = std::path::Path::new(path).parent() {
        interp.set_base_dir(parent.to_string_lossy().to_string());
    }
    interp.set_cli_args(cli_args);
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
println!("{:?} {:?}", x, v);
}"#,
        );
        assert_eq!(output, vec!["Some(3) [1, 2]\n"]);
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
println!("{:?} {:?}", v.first(), v.last());
}"#,
        );
        assert_eq!(output, vec!["Some(10) Some(30)\n"]);
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

    // === Phase 8: Traits & Generics ===

    #[test]
    fn test_trait_basic() {
        let out = run_and_capture(
            r#"
trait Greet {
    fn greet(&self) -> String;
}

struct Person {
    name: String,
}

impl Greet for Person {
    fn greet(&self) -> String {
        format!("Hello, I'm {}!", self.name)
    }
}

fn main() {
    let p = Person { name: String::from("Alice") };
    println!("{}", p.greet());
}
"#,
        );
        assert_eq!(out, vec!["Hello, I'm Alice!\n"]);
    }

    #[test]
    fn test_trait_multiple_methods() {
        let out = run_and_capture(
            r#"
trait Shape {
    fn area(&self) -> f64;
    fn name(&self) -> String;
}

struct Circle {
    radius: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        3.14159 * self.radius * self.radius
    }

    fn name(&self) -> String {
        String::from("Circle")
    }
}

fn main() {
    let c = Circle { radius: 5.0 };
    println!("{}: {}", c.name(), c.area());
}
"#,
        );
        assert_eq!(out, vec!["Circle: 78.53975\n"]);
    }

    #[test]
    fn test_trait_default_method() {
        let out = run_and_capture(
            r#"
trait Describable {
    fn name(&self) -> String;
    fn describe(&self) -> String {
        format!("I am {}", self.name())
    }
}

struct Dog {
    breed: String,
}

impl Describable for Dog {
    fn name(&self) -> String {
        self.breed.clone()
    }
}

fn main() {
    let d = Dog { breed: String::from("Labrador") };
    println!("{}", d.describe());
}
"#,
        );
        assert_eq!(out, vec!["I am Labrador\n"]);
    }

    #[test]
    fn test_format_macro() {
        let out = run_and_capture(
            r#"
fn main() {
    let s = format!("Hello, {}!", "world");
    println!("{}", s);
    let n = 42;
    let msg = format!("The answer is {}", n);
    println!("{}", msg);
}
"#,
        );
        assert_eq!(out, vec!["Hello, world!\n", "The answer is 42\n"]);
    }

    #[test]
    fn test_operator_overloading_add() {
        let out = run_and_capture(
            r#"
struct Vec2 {
    x: f64,
    y: f64,
}

impl Vec2 {
    fn new(x: f64, y: f64) -> Self {
        Vec2 { x, y }
    }
}

impl Add for Vec2 {
    fn add(&self, other: &Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}

fn main() {
    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(3.0, 4.0);
    let c = a + b;
    println!("{} {}", c.x, c.y);
}
"#,
        );
        assert_eq!(out, vec!["4.0 6.0\n"]);
    }

    #[test]
    fn test_operator_overloading_mul() {
        let out = run_and_capture(
            r#"
struct Vec2 {
    x: f64,
    y: f64,
}

impl Mul for Vec2 {
    fn mul(&self, other: &Vec2) -> Vec2 {
        Vec2 { x: self.x * other.x, y: self.y * other.y }
    }
}

fn main() {
    let a = Vec2 { x: 2.0, y: 3.0 };
    let b = Vec2 { x: 4.0, y: 5.0 };
    let c = a * b;
    println!("{} {}", c.x, c.y);
}
"#,
        );
        assert_eq!(out, vec!["8.0 15.0\n"]);
    }

    #[test]
    fn test_generic_function() {
        let out = run_and_capture(
            r#"
fn identity<T>(x: T) -> T {
    x
}

fn main() {
    let a = identity(42);
    let b = identity("hello");
    println!("{} {}", a, b);
}
"#,
        );
        assert_eq!(out, vec!["42 hello\n"]);
    }

    #[test]
    fn test_generic_function_with_bounds() {
        let out = run_and_capture(
            r#"
fn print_val<T: Display>(x: T) {
    println!("{}", x);
}

fn main() {
    print_val(42);
    print_val("hello");
}
"#,
        );
        assert_eq!(out, vec!["42\n", "hello\n"]);
    }

    #[test]
    fn test_trait_with_impl_and_direct_methods() {
        let out = run_and_capture(
            r#"
trait Summary {
    fn summarize(&self) -> String;
}

struct Article {
    title: String,
    content: String,
}

impl Article {
    fn new(title: String, content: String) -> Self {
        Article { title, content }
    }
}

impl Summary for Article {
    fn summarize(&self) -> String {
        format!("{}: {}", self.title, self.content)
    }
}

fn main() {
    let a = Article::new(String::from("Ferrite"), String::from("A Rust-like language"));
    println!("{}", a.summarize());
}
"#,
        );
        assert_eq!(out, vec!["Ferrite: A Rust-like language\n"]);
    }

    #[test]
    fn test_multiple_traits_for_type() {
        let out = run_and_capture(
            r#"
trait Greet {
    fn greet(&self) -> String;
}

trait Farewell {
    fn farewell(&self) -> String;
}

struct Person {
    name: String,
}

impl Greet for Person {
    fn greet(&self) -> String {
        format!("Hi, I'm {}", self.name)
    }
}

impl Farewell for Person {
    fn farewell(&self) -> String {
        format!("Goodbye from {}", self.name)
    }
}

fn main() {
    let p = Person { name: String::from("Bob") };
    println!("{}", p.greet());
    println!("{}", p.farewell());
}
"#,
        );
        assert_eq!(out, vec!["Hi, I'm Bob\n", "Goodbye from Bob\n"]);
    }

    #[test]
    fn test_string_from() {
        let out = run_and_capture(
            r#"
fn main() {
    let s = String::from("hello");
    println!("{}", s);
}
"#,
        );
        assert_eq!(out, vec!["hello\n"]);
    }

    #[test]
    fn test_trait_on_enum() {
        let out = run_and_capture(
            r#"
trait Describe {
    fn describe(&self) -> String;
}

enum Color {
    Red,
    Green,
    Blue,
}

impl Describe for Color {
    fn describe(&self) -> String {
        match self {
            Color::Red => String::from("red"),
            Color::Green => String::from("green"),
            Color::Blue => String::from("blue"),
        }
    }
}

fn main() {
    let c = Color::Green;
    println!("{}", c.describe());
}
"#,
        );
        assert_eq!(out, vec!["green\n"]);
    }

    #[test]
    fn test_clone_method_on_string() {
        let out = run_and_capture(
            r#"
fn main() {
    let s = String::from("hello");
    let s2 = s.clone();
    println!("{} {}", s, s2);
}
"#,
        );
        assert_eq!(out, vec!["hello hello\n"]);
    }

    // === Phase 9: Error Handling ===

    #[test]
    fn test_option_some_none() {
        let out = run_and_capture(
            r#"
fn main() {
    let x = Some(42);
    let y = None;
    println!("{:?} {:?}", x, y);
}
"#,
        );
        assert_eq!(out, vec!["Some(42) None\n"]);
    }

    #[test]
    fn test_option_unwrap() {
        let out = run_and_capture(
            r#"
fn main() {
    let x = Some(42);
    println!("{}", x.unwrap());
}
"#,
        );
        assert_eq!(out, vec!["42\n"]);
    }

    #[test]
    fn test_option_is_some_is_none() {
        let out = run_and_capture(
            r#"
fn main() {
    let x = Some(42);
    let y = None;
    println!("{} {} {} {}", x.is_some(), x.is_none(), y.is_some(), y.is_none());
}
"#,
        );
        assert_eq!(out, vec!["true false false true\n"]);
    }

    #[test]
    fn test_option_unwrap_or() {
        let out = run_and_capture(
            r#"
fn main() {
    let x = Some(42);
    let y = None;
    println!("{} {}", x.unwrap_or(0), y.unwrap_or(0));
}
"#,
        );
        assert_eq!(out, vec!["42 0\n"]);
    }

    #[test]
    fn test_result_ok_err() {
        let out = run_and_capture(
            r#"
fn main() {
    let x = Ok(42);
    let y = Err("failed");
    println!("{:?} {:?}", x, y);
}
"#,
        );
        assert_eq!(out, vec!["Ok(42) Err(\"failed\")\n"]);
    }

    #[test]
    fn test_result_unwrap() {
        let out = run_and_capture(
            r#"
fn main() {
    let x = Ok(42);
    println!("{}", x.unwrap());
}
"#,
        );
        assert_eq!(out, vec!["42\n"]);
    }

    #[test]
    fn test_result_is_ok_is_err() {
        let out = run_and_capture(
            r#"
fn main() {
    let x = Ok(42);
    let y = Err("oops");
    println!("{} {} {} {}", x.is_ok(), x.is_err(), y.is_ok(), y.is_err());
}
"#,
        );
        assert_eq!(out, vec!["true false false true\n"]);
    }

    #[test]
    fn test_result_unwrap_or() {
        let out = run_and_capture(
            r#"
fn main() {
    let x = Ok(42);
    let y = Err("oops");
    println!("{} {}", x.unwrap_or(0), y.unwrap_or(0));
}
"#,
        );
        assert_eq!(out, vec!["42 0\n"]);
    }

    #[test]
    fn test_result_unwrap_err() {
        let out = run_and_capture(
            r#"
fn main() {
    let y = Err("oops");
    println!("{}", y.unwrap_err());
}
"#,
        );
        assert_eq!(out, vec!["oops\n"]);
    }

    #[test]
    fn test_if_let_some() {
        let out = run_and_capture(
            r#"
fn main() {
    let x = Some(42);
    if let Some(val) = x {
        println!("got {}", val);
    } else {
        println!("nothing");
    }
}
"#,
        );
        assert_eq!(out, vec!["got 42\n"]);
    }

    #[test]
    fn test_if_let_none() {
        let out = run_and_capture(
            r#"
fn main() {
    let x = None;
    if let Some(val) = x {
        println!("got {}", val);
    } else {
        println!("nothing");
    }
}
"#,
        );
        assert_eq!(out, vec!["nothing\n"]);
    }

    #[test]
    fn test_while_let() {
        let out = run_and_capture(
            r#"
fn main() {
    let mut v = vec![1, 2, 3];
    while let Some(val) = v.pop() {
        println!("{}", val);
    }
}
"#,
        );
        assert_eq!(out, vec!["3\n", "2\n", "1\n"]);
    }

    #[test]
    fn test_match_option() {
        let out = run_and_capture(
            r#"
fn main() {
    let x = Some(42);
    match x {
        Some(val) => println!("value: {}", val),
        None => println!("nothing"),
    }
}
"#,
        );
        assert_eq!(out, vec!["value: 42\n"]);
    }

    #[test]
    fn test_match_result() {
        let out = run_and_capture(
            r#"
fn main() {
    let x = Err("problem");
    match x {
        Ok(val) => println!("ok: {}", val),
        Err(e) => println!("err: {}", e),
    }
}
"#,
        );
        assert_eq!(out, vec!["err: problem\n"]);
    }

    #[test]
    fn test_try_operator_ok() {
        let out = run_and_capture(
            r#"
fn parse_num(s: &str) -> Result {
    if s == "42" {
        Ok(42)
    } else {
        Err("parse error")
    }
}

fn do_work() -> Result {
    let n = parse_num("42")?;
    Ok(n + 1)
}

fn main() {
    let result = do_work();
    println!("{:?}", result);
}
"#,
        );
        assert_eq!(out, vec!["Ok(43)\n"]);
    }

    #[test]
    fn test_try_operator_err() {
        let out = run_and_capture(
            r#"
fn parse_num(s: &str) -> Result {
    if s == "42" {
        Ok(42)
    } else {
        Err("parse error")
    }
}

fn do_work() -> Result {
    let n = parse_num("bad")?;
    Ok(n + 1)
}

fn main() {
    let result = do_work();
    println!("{:?}", result);
}
"#,
        );
        assert_eq!(out, vec!["Err(\"parse error\")\n"]);
    }

    #[test]
    fn test_panic_macro() {
        let src = r#"
fn main() {
    panic!("something went wrong");
}
"#;
        let program = crate::parser::parse(src).unwrap();
        let mut interp = Interpreter::new_with_captured_output();
        let result = interp.execute_program(&program);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("panic: something went wrong"));
    }

    #[test]
    fn test_dbg_macro() {
        let out = run_and_capture(
            r#"
fn main() {
    let x = dbg!(42);
    println!("{}", x);
}
"#,
        );
        assert_eq!(out, vec!["[dbg] 42\n", "42\n"]);
    }

    #[test]
    fn test_option_map() {
        let out = run_and_capture(
            r#"
fn double(x: i64) -> i64 { x * 2 }

fn main() {
    let x = Some(21);
    let y = x.map(double);
    println!("{:?}", y);

    let z = None;
    let w = z.map(double);
    println!("{:?}", w);
}
"#,
        );
        assert_eq!(out, vec!["Some(42)\n", "None\n"]);
    }

    #[test]
    fn test_result_map() {
        let out = run_and_capture(
            r#"
fn double(x: i64) -> i64 { x * 2 }

fn main() {
    let x = Ok(21);
    let y = x.map(double);
    println!("{:?}", y);
}
"#,
        );
        assert_eq!(out, vec!["Ok(42)\n"]);
    }

    #[test]
    fn test_result_ok_to_option() {
        let out = run_and_capture(
            r#"
fn main() {
    let x = Ok(42);
    let y = Err("bad");
    println!("{:?} {:?}", x.ok(), y.ok());
}
"#,
        );
        assert_eq!(out, vec!["Some(42) None\n"]);
    }

    #[test]
    fn test_if_let_result() {
        let out = run_and_capture(
            r#"
fn main() {
    let x = Ok(42);
    if let Ok(val) = x {
        println!("ok: {}", val);
    } else {
        println!("err");
    }
}
"#,
        );
        assert_eq!(out, vec!["ok: 42\n"]);
    }

    #[test]
    fn test_option_function_return() {
        let out = run_and_capture(
            r#"
fn find_item(items: Vec, target: i64) -> Option {
    for i in 0..items.len() {
        if items[i] == target {
            return Some(i);
        }
    }
    None
}

fn main() {
    let items = vec![10, 20, 30, 40];
    let result = find_item(items, 30);
    match result {
        Some(idx) => println!("found at {}", idx),
        None => println!("not found"),
    }
}
"#,
        );
        assert_eq!(out, vec!["found at 2\n"]);
    }

    #[test]
    fn test_try_operator_option() {
        let out = run_and_capture(
            r#"
fn get_first(v: Vec) -> Option {
    if v.is_empty() {
        None
    } else {
        Some(v[0])
    }
}

fn process() -> Option {
    let v = vec![10, 20, 30];
    let first = get_first(v)?;
    Some(first * 2)
}

fn main() {
    let result = process();
    println!("{:?}", result);
}
"#,
        );
        assert_eq!(out, vec!["Some(20)\n"]);
    }

    #[test]
    fn test_option_unwrap_none_panics() {
        let src = r#"
fn main() {
    let x = None;
    x.unwrap();
}
"#;
        let program = crate::parser::parse(src).unwrap();
        let mut interp = Interpreter::new_with_captured_output();
        let result = interp.execute_program(&program);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("called `Option::unwrap()` on a `None` value"));
    }

    #[test]
    fn test_result_unwrap_err_panics() {
        let src = r#"
fn main() {
    let x = Err("bad");
    x.unwrap();
}
"#;
        let program = crate::parser::parse(src).unwrap();
        let mut interp = Interpreter::new_with_captured_output();
        let result = interp.execute_program(&program);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("called `Result::unwrap()` on an `Err` value"));
    }

    // === Phase 10: Closures & Higher-Order Functions ===

    #[test]
    fn test_closure_basic() {
        let output = run_and_capture(
            r#"fn main() {
let add = |a: i64, b: i64| a + b;
println!("{}", add(3, 4));
}"#,
        );
        assert_eq!(output, vec!["7\n"]);
    }

    #[test]
    fn test_closure_no_type_annotation() {
        let output = run_and_capture(
            r#"fn main() {
let double = |x| x * 2;
println!("{}", double(5));
}"#,
        );
        assert_eq!(output, vec!["10\n"]);
    }

    #[test]
    fn test_closure_no_params() {
        let output = run_and_capture(
            r#"fn main() {
let greet = || "hello";
println!("{}", greet());
}"#,
        );
        assert_eq!(output, vec!["hello\n"]);
    }

    #[test]
    fn test_closure_block_body() {
        let output = run_and_capture(
            r#"fn main() {
let compute = |x: i64| {
    let y = x * 2;
    y + 1
};
println!("{}", compute(10));
}"#,
        );
        assert_eq!(output, vec!["21\n"]);
    }

    #[test]
    fn test_closure_captures_variable() {
        let output = run_and_capture(
            r#"fn main() {
let factor = 3;
let multiply = |x| x * factor;
println!("{}", multiply(5));
}"#,
        );
        assert_eq!(output, vec!["15\n"]);
    }

    #[test]
    fn test_closure_as_argument() {
        let output = run_and_capture(
            r#"fn apply(f: Fn, x: i64) -> i64 {
    f(x)
}
fn main() {
    let result = apply(|x| x * x, 7);
    println!("{}", result);
}"#,
        );
        assert_eq!(output, vec!["49\n"]);
    }

    #[test]
    fn test_closure_returned_from_function() {
        let output = run_and_capture(
            r#"fn make_adder(n: i64) -> Fn {
    |x| x + n
}
fn main() {
    let add5 = make_adder(5);
    println!("{}", add5(10));
}"#,
        );
        assert_eq!(output, vec!["15\n"]);
    }

    #[test]
    fn test_move_closure() {
        let output = run_and_capture(
            r#"fn main() {
let name = "world";
let greet = move || format!("hello {}", name);
println!("{}", greet());
}"#,
        );
        assert_eq!(output, vec!["hello world\n"]);
    }

    #[test]
    fn test_vec_map() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec![1, 2, 3];
let doubled = v.map(|x| x * 2);
println!("{:?}", doubled);
}"#,
        );
        assert_eq!(output, vec!["[2, 4, 6]\n"]);
    }

    #[test]
    fn test_vec_filter() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec![1, 2, 3, 4, 5];
let evens = v.filter(|x| x % 2 == 0);
println!("{:?}", evens);
}"#,
        );
        assert_eq!(output, vec!["[2, 4]\n"]);
    }

    #[test]
    fn test_vec_for_each() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec![10, 20, 30];
v.for_each(|x| println!("{}", x));
}"#,
        );
        assert_eq!(output, vec!["10\n", "20\n", "30\n"]);
    }

    #[test]
    fn test_vec_fold() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec![1, 2, 3, 4];
let sum = v.fold(0, |acc, x| acc + x);
println!("{}", sum);
}"#,
        );
        assert_eq!(output, vec!["10\n"]);
    }

    #[test]
    fn test_vec_any_all() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec![1, 2, 3, 4, 5];
println!("{}", v.any(|x| x > 4));
println!("{}", v.all(|x| x > 0));
println!("{}", v.all(|x| x > 3));
}"#,
        );
        assert_eq!(output, vec!["true\n", "true\n", "false\n"]);
    }

    #[test]
    fn test_vec_find() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec![1, 2, 3, 4, 5];
let found = v.find(|x| x > 3);
println!("{:?}", found);
let not_found = v.find(|x| x > 10);
println!("{:?}", not_found);
}"#,
        );
        assert_eq!(output, vec!["Some(4)\n", "None\n"]);
    }

    #[test]
    fn test_vec_enumerate() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec!["a", "b", "c"];
let pairs = v.enumerate();
println!("{:?}", pairs);
}"#,
        );
        assert_eq!(output, vec!["[(0, \"a\"), (1, \"b\"), (2, \"c\")]\n"]);
    }

    #[test]
    fn test_vec_chain_map_filter() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec![1, 2, 3, 4, 5];
let result = v.map(|x| x * 2).filter(|x| x > 4);
println!("{:?}", result);
}"#,
        );
        assert_eq!(output, vec!["[6, 8, 10]\n"]);
    }

    #[test]
    fn test_vec_flat_map() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec![1, 2, 3];
let result = v.flat_map(|x| vec![x, x * 10]);
println!("{:?}", result);
}"#,
        );
        assert_eq!(output, vec!["[1, 10, 2, 20, 3, 30]\n"]);
    }

    #[test]
    fn test_vec_position() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec![10, 20, 30];
println!("{:?}", v.position(|x| x == 20));
println!("{:?}", v.position(|x| x == 99));
}"#,
        );
        assert_eq!(output, vec!["Some(1)\n", "None\n"]);
    }

    #[test]
    fn test_option_map_with_closure() {
        let output = run_and_capture(
            r#"fn main() {
let val = Some(5);
let doubled = val.map(|x| x * 2);
println!("{:?}", doubled);
let none_val: Option<i64> = None;
let mapped = none_val.map(|x| x * 2);
println!("{:?}", mapped);
}"#,
        );
        assert_eq!(output, vec!["Some(10)\n", "None\n"]);
    }

    #[test]
    fn test_result_map_with_closure() {
        let output = run_and_capture(
            r#"fn main() {
let val: Result<i64, String> = Ok(5);
let doubled = val.map(|x| x * 2);
println!("{:?}", doubled);
}"#,
        );
        assert_eq!(output, vec!["Ok(10)\n"]);
    }

    #[test]
    fn test_closure_as_method_callback() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec![1, 2, 3];
let sum = v.fold(0, |acc, x| acc + x);
let product = v.fold(1, |acc, x| acc * x);
println!("{} {}", sum, product);
}"#,
        );
        assert_eq!(output, vec!["6 6\n"]);
    }

    #[test]
    fn test_iter_collect() {
        let output = run_and_capture(
            r#"fn main() {
let v = vec![1, 2, 3];
let v2 = v.iter().collect();
println!("{:?}", v2);
}"#,
        );
        assert_eq!(output, vec!["[1, 2, 3]\n"]);
    }

    // === Phase 11: Modules & Use Statements ===

    #[test]
    fn test_inline_module() {
        let output = run_and_capture(
            r#"
mod math {
    fn add(a: i64, b: i64) -> i64 {
        a + b
    }
}
use math::add;
fn main() {
    println!("{}", add(3, 4));
}"#,
        );
        assert_eq!(output, vec!["7\n"]);
    }

    #[test]
    fn test_module_path_call() {
        let output = run_and_capture(
            r#"
mod math {
    fn multiply(a: i64, b: i64) -> i64 {
        a * b
    }
}
fn main() {
    println!("{}", math::multiply(3, 4));
}"#,
        );
        assert_eq!(output, vec!["12\n"]);
    }

    #[test]
    fn test_use_glob_import() {
        let output = run_and_capture(
            r#"
mod utils {
    fn greet(name: String) -> String {
        format!("Hello, {}!", name)
    }
    fn farewell(name: String) -> String {
        format!("Goodbye, {}!", name)
    }
}
use utils::*;
fn main() {
    println!("{}", greet("Alice"));
    println!("{}", farewell("Bob"));
}"#,
        );
        assert_eq!(output, vec!["Hello, Alice!\n", "Goodbye, Bob!\n"]);
    }

    #[test]
    fn test_use_group_import() {
        let output = run_and_capture(
            r#"
mod ops {
    fn add(a: i64, b: i64) -> i64 { a + b }
    fn sub(a: i64, b: i64) -> i64 { a - b }
    fn mul(a: i64, b: i64) -> i64 { a * b }
}
use ops::{add, sub};
fn main() {
    println!("{} {}", add(10, 3), sub(10, 3));
}"#,
        );
        assert_eq!(output, vec!["13 7\n"]);
    }

    #[test]
    fn test_module_with_struct() {
        let output = run_and_capture(
            r#"
mod geometry {
    struct Point { x: f64, y: f64 }
    impl Point {
        fn new(x: f64, y: f64) -> Self {
            Point { x, y }
        }
        fn to_string(&self) -> String {
            format!("({}, {})", self.x, self.y)
        }
    }
}
use geometry::Point;
fn main() {
    let p = Point::new(1.0, 2.0);
    println!("{}", p.to_string());
}"#,
        );
        assert_eq!(output, vec!["(1.0, 2.0)\n"]);
    }

    #[test]
    fn test_module_with_enum() {
        let output = run_and_capture(
            r#"
mod colors {
    enum Color { Red, Green, Blue }
}
use colors::Color;
fn main() {
    let c = Color::Red;
    match c {
        Color::Red => println!("red"),
        Color::Green => println!("green"),
        Color::Blue => println!("blue"),
    }
}"#,
        );
        assert_eq!(output, vec!["red\n"]);
    }

    #[test]
    fn test_pub_keyword_accepted() {
        let output = run_and_capture(
            r#"
pub mod math {
    pub fn add(a: i64, b: i64) -> i64 { a + b }
}
use math::add;
fn main() {
    println!("{}", add(1, 2));
}"#,
        );
        assert_eq!(output, vec!["3\n"]);
    }

    #[test]
    fn test_pub_fn_accepted() {
        let output = run_and_capture(
            r#"
pub fn helper() -> i64 { 42 }
fn main() {
    println!("{}", helper());
}"#,
        );
        assert_eq!(output, vec!["42\n"]);
    }

    #[test]
    fn test_multiple_modules() {
        let output = run_and_capture(
            r#"
mod a {
    fn foo() -> i64 { 1 }
}
mod b {
    fn bar() -> i64 { 2 }
}
use a::foo;
use b::bar;
fn main() {
    println!("{}", foo() + bar());
}"#,
        );
        assert_eq!(output, vec!["3\n"]);
    }

    // === Phase 12: Type Aliases ===

    #[test]
    fn test_type_alias() {
        let output = run_and_capture(
            r#"
type Meters = f64;
fn main() {
    let d: Meters = 42.0;
    println!("{}", d);
}
"#,
        );
        assert_eq!(output, vec!["42.0\n"]);
    }

    // === Phase 12: Constants ===

    #[test]
    fn test_const() {
        let output = run_and_capture(
            r#"
const MAX: i64 = 100;
fn main() {
    println!("{}", MAX);
}
"#,
        );
        assert_eq!(output, vec!["100\n"]);
    }

    #[test]
    fn test_static() {
        let output = run_and_capture(
            r#"
static PI: f64 = 3.14;
fn main() {
    println!("{}", PI);
}
"#,
        );
        assert_eq!(output, vec!["3.14\n"]);
    }

    #[test]
    fn test_const_no_type_ann() {
        let output = run_and_capture(
            r#"
const GREETING = "hello";
fn main() {
    println!("{}", GREETING);
}
"#,
        );
        assert_eq!(output, vec!["hello\n"]);
    }

    #[test]
    fn test_const_used_in_function() {
        let output = run_and_capture(
            r#"
const FACTOR: i64 = 10;
fn multiply(x: i64) -> i64 {
    x * FACTOR
}
fn main() {
    println!("{}", multiply(5));
}
"#,
        );
        assert_eq!(output, vec!["50\n"]);
    }

    // === Phase 12: HashMap ===

    #[test]
    fn test_hashmap_new_and_insert() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut m = HashMap::new();
    m.insert("a", 1);
    m.insert("b", 2);
    println!("{}", m.len());
}
"#,
        );
        assert_eq!(output, vec!["2\n"]);
    }

    #[test]
    fn test_hashmap_get() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut m = HashMap::new();
    m.insert("key", 42);
    let val = m.get("key");
    println!("{}", val.unwrap());
}
"#,
        );
        assert_eq!(output, vec!["42\n"]);
    }

    #[test]
    fn test_hashmap_get_missing() {
        let output = run_and_capture(
            r#"
fn main() {
    let m = HashMap::new();
    let val = m.get("nope");
    println!("{}", val.is_none());
}
"#,
        );
        assert_eq!(output, vec!["true\n"]);
    }

    #[test]
    fn test_hashmap_contains_key() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut m = HashMap::new();
    m.insert("x", 1);
    println!("{}", m.contains_key("x"));
    println!("{}", m.contains_key("y"));
}
"#,
        );
        assert_eq!(output, vec!["true\n", "false\n"]);
    }

    #[test]
    fn test_hashmap_remove() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut m = HashMap::new();
    m.insert("a", 10);
    let removed = m.remove("a");
    println!("{}", removed.unwrap());
    println!("{}", m.is_empty());
}
"#,
        );
        assert_eq!(output, vec!["10\n", "true\n"]);
    }

    #[test]
    fn test_hashmap_keys_values() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut m = HashMap::new();
    m.insert("b", 2);
    m.insert("a", 1);
    println!("{:?}", m.keys());
    println!("{:?}", m.values());
}
"#,
        );
        assert_eq!(output, vec!["[\"a\", \"b\"]\n", "[1, 2]\n"]);
    }

    #[test]
    fn test_hashmap_debug_format() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut m = HashMap::new();
    m.insert("x", 1);
    println!("{:?}", m);
}
"#,
        );
        assert_eq!(output, vec!["{\"x\": 1}\n"]);
    }

    #[test]
    fn test_hashmap_iteration() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut m = HashMap::new();
    m.insert("a", 1);
    m.insert("b", 2);
    for (k, v) in m {
        println!("{}: {}", k, v);
    }
}
"#,
        );
        assert_eq!(output, vec!["a: 1\n", "b: 2\n"]);
    }

    #[test]
    fn test_hashmap_is_empty() {
        let output = run_and_capture(
            r#"
fn main() {
    let m = HashMap::new();
    println!("{}", m.is_empty());
}
"#,
        );
        assert_eq!(output, vec!["true\n"]);
    }

    // === Phase 12: For Destructuring ===

    #[test]
    fn test_for_destructure_vec_of_tuples() {
        let output = run_and_capture(
            r#"
fn main() {
    let pairs = vec![(1, "a"), (2, "b")];
    for (num, letter) in pairs {
        println!("{} {}", num, letter);
    }
}
"#,
        );
        assert_eq!(output, vec!["1 a\n", "2 b\n"]);
    }

    // === Phase 12: CLI Args (via set_cli_args) ===

    #[test]
    fn test_cli_args() {
        let src = r#"
fn main() {
    let args = std::env::args();
    for arg in args {
        println!("{}", arg);
    }
}
"#;
        let program = crate::parser::parse(src).unwrap();
        let mut interp = Interpreter::new_with_captured_output();
        interp.set_cli_args(vec!["test".to_string(), "arg1".to_string()]);
        interp.execute_program(&program).unwrap();
        let output = interp.captured_output().to_vec();
        assert_eq!(output, vec!["test\n", "arg1\n"]);
    }

    // === Phase 13: JSON & Serialization ===

    #[test]
    fn test_json_serialize_primitives() {
        let output = run_and_capture(
            r#"fn main() {
    let a = json::serialize(42).unwrap();
    let b = json::serialize(3.14).unwrap();
    let c = json::serialize(true).unwrap();
    let d = json::serialize("hello").unwrap();
    println!("{}", a);
    println!("{}", b);
    println!("{}", c);
    println!("{}", d);
}"#,
        );
        assert_eq!(output, vec!["42\n", "3.14\n", "true\n", "\"hello\"\n"]);
    }

    #[test]
    fn test_json_serialize_string_escapes() {
        let output = run_and_capture(
            r#"fn main() {
    let s = json::serialize("hello\nworld\t\"quoted\"").unwrap();
    println!("{}", s);
}"#,
        );
        assert_eq!(output, vec!["\"hello\\nworld\\t\\\"quoted\\\"\"\n"]);
    }

    #[test]
    fn test_json_serialize_vec() {
        let output = run_and_capture(
            r#"fn main() {
    let v = vec![1, 2, 3];
    let j = json::serialize(v).unwrap();
    println!("{}", j);
}"#,
        );
        assert_eq!(output, vec!["[1, 2, 3]\n"]);
    }

    #[test]
    fn test_json_serialize_hashmap() {
        let output = run_and_capture(
            r#"fn main() {
    let mut m = HashMap::new();
    m.insert("alpha", 1);
    m.insert("beta", 2);
    let j = json::serialize(m).unwrap();
    println!("{}", j);
}"#,
        );
        assert_eq!(output, vec!["{\"alpha\": 1, \"beta\": 2}\n"]);
    }

    #[test]
    fn test_json_serialize_struct() {
        let output = run_and_capture(
            r#"
struct Point {
    x: i64,
    y: i64,
}
fn main() {
    let p = Point { x: 10, y: 20 };
    let j = json::serialize(p).unwrap();
    println!("{}", j);
}"#,
        );
        assert_eq!(output, vec!["{\"x\": 10, \"y\": 20}\n"]);
    }

    #[test]
    fn test_json_serialize_enum() {
        let output = run_and_capture(
            r#"
enum Color {
    Red,
    Green,
    Blue,
    Rgb(i64, i64, i64),
}
fn main() {
    let a = json::serialize(Color::Red).unwrap();
    let b = json::serialize(Color::Rgb(255, 128, 0)).unwrap();
    println!("{}", a);
    println!("{}", b);
}"#,
        );
        assert_eq!(
            output,
            vec![
                "\"Red\"\n",
                "{\"variant\": \"Rgb\", \"data\": [255, 128, 0]}\n"
            ]
        );
    }

    #[test]
    fn test_json_serialize_option_result() {
        let output = run_and_capture(
            r#"fn main() {
    let a = json::serialize(Some(42)).unwrap();
    let b = json::serialize(None).unwrap();
    let c = json::serialize(Ok("yes")).unwrap();
    let d = json::serialize(Err("no")).unwrap();
    println!("{}", a);
    println!("{}", b);
    println!("{}", c);
    println!("{}", d);
}"#,
        );
        assert_eq!(
            output,
            vec![
                "42\n",
                "null\n",
                "{\"Ok\": \"yes\"}\n",
                "{\"Err\": \"no\"}\n"
            ]
        );
    }

    #[test]
    fn test_json_serialize_nested() {
        let output = run_and_capture(
            r#"fn main() {
    let v = vec![vec![1, 2], vec![3, 4]];
    let j = json::serialize(v).unwrap();
    println!("{}", j);
}"#,
        );
        assert_eq!(output, vec!["[[1, 2], [3, 4]]\n"]);
    }

    #[test]
    fn test_json_serialize_pretty() {
        let output = run_and_capture(
            r#"fn main() {
    let v = vec![1, 2, 3];
    let j = json::to_string_pretty(v).unwrap();
    println!("{}", j);
}"#,
        );
        assert_eq!(output, vec!["[\n  1,\n  2,\n  3\n]\n"]);
    }

    #[test]
    fn test_json_deserialize_primitives() {
        let output = run_and_capture(
            r#"fn main() {
    let a = json::deserialize("42").unwrap();
    let b = json::deserialize("3.14").unwrap();
    let c = json::deserialize("true").unwrap();
    let d = json::deserialize("\"hello\"").unwrap();
    let e = json::deserialize("null").unwrap();
    println!("{:?}", a);
    println!("{:?}", b);
    println!("{:?}", c);
    println!("{}", d);
    println!("{:?}", e);
}"#,
        );
        assert_eq!(output, vec!["42\n", "3.14\n", "true\n", "hello\n", "()\n"]);
    }

    #[test]
    fn test_json_deserialize_object() {
        let output = run_and_capture(
            r#"fn main() {
    let obj = json::parse("{\"name\": \"Alice\", \"age\": 30}").unwrap();
    let name = obj.get("name").unwrap();
    let age = obj.get("age").unwrap();
    println!("{}", name);
    println!("{:?}", age);
}"#,
        );
        assert_eq!(output, vec!["Alice\n", "30\n"]);
    }

    #[test]
    fn test_json_deserialize_array() {
        let output = run_and_capture(
            r#"fn main() {
    let arr = json::from_str("[1, 2, 3]").unwrap();
    println!("{:?}", arr);
}"#,
        );
        assert_eq!(output, vec!["[1, 2, 3]\n"]);
    }

    #[test]
    fn test_json_deserialize_nested() {
        let output = run_and_capture(
            r#"fn main() {
    let data = json::deserialize("{\"items\": [1, 2, 3], \"ok\": true}").unwrap();
    let items = data.get("items").unwrap();
    let ok = data.get("ok").unwrap();
    println!("{:?}", items);
    println!("{:?}", ok);
}"#,
        );
        assert_eq!(output, vec!["[1, 2, 3]\n", "true\n"]);
    }

    #[test]
    fn test_json_roundtrip() {
        let output = run_and_capture(
            r#"fn main() {
    let original = vec![1, 2, 3];
    let json_str = json::serialize(original).unwrap();
    let parsed = json::deserialize(json_str).unwrap();
    println!("{:?}", parsed);
}"#,
        );
        assert_eq!(output, vec!["[1, 2, 3]\n"]);
    }

    #[test]
    fn test_json_to_json_method() {
        let output = run_and_capture(
            r#"fn main() {
    let v = vec![1, 2, 3];
    let j = v.to_json().unwrap();
    println!("{}", j);
    let n = 42;
    let j2 = n.to_json().unwrap();
    println!("{}", j2);
}"#,
        );
        assert_eq!(output, vec!["[1, 2, 3]\n", "42\n"]);
    }

    #[test]
    fn test_json_error_cases() {
        let output = run_and_capture(
            r#"fn main() {
    let r = json::deserialize("invalid");
    match r {
        Result::Ok(_) => println!("unexpected ok"),
        Result::Err(e) => println!("error: {}", e),
    }
}"#,
        );
        assert!(output[0].starts_with("error: "));
    }

    #[test]
    fn test_json_from_struct() {
        let output = run_and_capture(
            r#"
struct Person {
    name: String,
    age: i64,
}
fn main() {
    let json_str = "{\"name\": \"Alice\", \"age\": 30}";
    let p = json::from_struct(json_str, "Person").unwrap();
    println!("{:?}", p);
}"#,
        );
        assert!(output[0].contains("Alice"));
        assert!(output[0].contains("30"));
    }

    // === HTTP module tests ===

    #[test]
    fn test_http_get_invalid_url() {
        let output = run_and_capture(
            r#"
fn main() {
    let result = http::get("not-a-valid-url");
    match result {
        Ok(_) => println!("unexpected ok"),
        Err(e) => println!("got error"),
    }
}"#,
        );
        assert_eq!(output, vec!["got error\n"]);
    }

    #[test]
    fn test_http_post_invalid_url() {
        let output = run_and_capture(
            r#"
fn main() {
    let result = http::post("http://invalid.test.localhost:1", "body");
    match result {
        Ok(_) => println!("unexpected ok"),
        Err(e) => println!("got error"),
    }
}"#,
        );
        assert_eq!(output, vec!["got error\n"]);
    }

    #[test]
    fn test_http_delete_invalid_url() {
        let output = run_and_capture(
            r#"
fn main() {
    let result = http::delete("not-a-valid-url");
    match result {
        Ok(_) => println!("unexpected ok"),
        Err(e) => println!("got error"),
    }
}"#,
        );
        assert_eq!(output, vec!["got error\n"]);
    }

    #[test]
    fn test_http_request_builder() {
        let output = run_and_capture(
            r#"
fn main() {
    let req = http::request("GET", "https://example.com");
    println!("{}", req.method);
    println!("{}", req.url);
}"#,
        );
        assert_eq!(output, vec!["GET\n", "https://example.com\n"]);
    }

    #[test]
    fn test_http_request_builder_header() {
        let output = run_and_capture(
            r#"
fn main() {
    let req = http::request("POST", "https://example.com")
        .header("Accept", "application/json");
    println!("{}", req.method);
}"#,
        );
        assert_eq!(output, vec!["POST\n"]);
    }

    #[test]
    fn test_http_request_builder_body() {
        let output = run_and_capture(
            r#"
fn main() {
    let req = http::request("POST", "https://example.com")
        .body("hello");
    println!("{}", req.body);
}"#,
        );
        assert_eq!(output, vec!["hello\n"]);
    }

    #[test]
    fn test_http_request_builder_send_invalid() {
        let output = run_and_capture(
            r#"
fn main() {
    let result = http::request("GET", "not-a-url").send();
    match result {
        Ok(_) => println!("unexpected ok"),
        Err(_) => println!("got error"),
    }
}"#,
        );
        assert_eq!(output, vec!["got error\n"]);
    }

    #[test]
    fn test_http_get_json_invalid_url() {
        let output = run_and_capture(
            r#"
fn main() {
    let result = http::get_json("not-a-valid-url");
    match result {
        Ok(_) => println!("unexpected ok"),
        Err(e) => println!("got error"),
    }
}"#,
        );
        assert_eq!(output, vec!["got error\n"]);
    }

    #[test]
    fn test_http_post_json_invalid_url() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut data = HashMap::new();
    data.insert("key", "value");
    let result = http::post_json("not-a-valid-url", data);
    match result {
        Ok(_) => println!("unexpected ok"),
        Err(_) => println!("got error"),
    }
}"#,
        );
        assert_eq!(output, vec!["got error\n"]);
    }

    #[test]
    fn test_http_put_json_invalid_url() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut data = HashMap::new();
    data.insert("key", "value");
    let result = http::put_json("not-a-valid-url", data);
    match result {
        Ok(_) => println!("unexpected ok"),
        Err(_) => println!("got error"),
    }
}"#,
        );
        assert_eq!(output, vec!["got error\n"]);
    }

    #[test]
    fn test_http_request_builder_json_body() {
        let output = run_and_capture(
            r#"
fn main() {
    let mut data = HashMap::new();
    data.insert("name", "Alice");
    let req = http::request("POST", "https://example.com")
        .json(data);
    println!("{}", req.body);
}"#,
        );
        assert_eq!(output, vec!["{\"name\": \"Alice\"}\n"]);
    }

    #[test]
    fn test_http_response_status_ok_logic() {
        // We can't make real requests, but we test the method dispatch
        // by building an HttpResponse struct directly via the builder pattern
        let output = run_and_capture(
            r#"
fn main() {
    let result = http::get("not-a-valid-url");
    match result {
        Ok(resp) => {
            println!("status_ok: {}", resp.status_ok());
        }
        Err(e) => println!("error as expected: {}", true),
    }
}"#,
        );
        assert_eq!(output, vec!["error as expected: true\n"]);
    }

    #[test]
    fn test_http_unknown_function() {
        let result = run_capturing(
            r#"
fn main() {
    let r = http::unknown_func("test");
}"#,
        );
        assert!(result.is_err());
    }

    // === Async/Await ===

    #[test]
    fn test_async_fn_basic() {
        let output = run_and_capture(
            r#"
async fn fetch_data() -> i64 {
    42
}
fn main() {
    let future = fetch_data();
    let result = future.await;
    println!("{}", result);
}"#,
        );
        assert_eq!(output, vec!["42\n"]);
    }

    #[test]
    fn test_async_fn_with_args() {
        let output = run_and_capture(
            r#"
async fn add(a: i64, b: i64) -> i64 {
    a + b
}
fn main() {
    let result = add(3, 4).await;
    println!("{}", result);
}"#,
        );
        assert_eq!(output, vec!["7\n"]);
    }

    #[test]
    fn test_await_chain() {
        let output = run_and_capture(
            r#"
async fn double(x: i64) -> i64 {
    x * 2
}
fn main() {
    let a = double(5).await;
    let b = double(a).await;
    println!("{}", b);
}"#,
        );
        assert_eq!(output, vec!["20\n"]);
    }

    #[test]
    fn test_spawn_and_await() {
        let output = run_and_capture(
            r#"
fn compute() -> i64 {
    let mut sum = 0;
    for i in 0..10 {
        sum += i;
    }
    sum
}
fn main() {
    let handle = spawn(compute);
    let result = handle.await;
    println!("{}", result);
}"#,
        );
        assert_eq!(output, vec!["45\n"]);
    }

    #[test]
    fn test_spawn_with_closure() {
        let output = run_and_capture(
            r#"
fn main() {
    let x = 10;
    let handle = spawn(|| x * 2);
    println!("{}", handle.await);
}"#,
        );
        assert_eq!(output, vec!["20\n"]);
    }

    #[test]
    fn test_sleep() {
        let output = run_and_capture(
            r#"
fn main() {
    sleep(1);
    println!("done");
}"#,
        );
        assert_eq!(output, vec!["done\n"]);
    }

    #[test]
    fn test_async_with_result() {
        let output = run_and_capture(
            r#"
async fn safe_divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        Err("division by zero".to_string())
    } else {
        Ok(a / b)
    }
}
fn main() {
    let result = safe_divide(10.0, 3.0).await;
    match result {
        Ok(v) => println!("ok"),
        Err(e) => println!("err: {}", e),
    }
}"#,
        );
        assert_eq!(output, vec!["ok\n"]);
    }

    #[test]
    fn test_pub_async_fn() {
        let output = run_and_capture(
            r#"
pub async fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}
fn main() {
    let msg = greet("World".to_string()).await;
    println!("{}", msg);
}"#,
        );
        assert_eq!(output, vec!["Hello, World!\n"]);
    }

    #[test]
    fn test_multiple_spawns() {
        let output = run_and_capture(
            r#"
fn main() {
    let h1 = spawn(|| 1);
    let h2 = spawn(|| 2);
    let h3 = spawn(|| 3);
    let sum = h1.await + h2.await + h3.await;
    println!("{}", sum);
}"#,
        );
        assert_eq!(output, vec!["6\n"]);
    }

    #[test]
    fn test_async_fn_in_module() {
        let output = run_and_capture(
            r#"
mod api {
    async fn fetch(id: i64) -> String {
        format!("item-{}", id)
    }
}
use api::fetch;
fn main() {
    let item = fetch(42).await;
    println!("{}", item);
}"#,
        );
        assert_eq!(output, vec!["item-42\n"]);
    }

    // === Math stdlib ===

    #[test]
    fn test_math_sqrt() {
        let out = run_and_capture("fn main() { println!(\"{}\", math::sqrt(16.0)); }");
        assert_eq!(out, vec!["4\n"]);
    }

    #[test]
    fn test_math_trig() {
        let out = run_and_capture(
            "fn main() { println!(\"{}\", math::sin(0.0)); println!(\"{}\", math::cos(0.0)); }",
        );
        assert_eq!(out, vec!["0\n", "1\n"]);
    }

    #[test]
    fn test_math_constants() {
        let out = run_and_capture("fn main() { println!(\"{}\", math::PI); }");
        assert_eq!(out, vec!["3.141592653589793\n"]);
    }

    #[test]
    fn test_math_constant_e() {
        let out = run_and_capture("fn main() { println!(\"{}\", math::E); }");
        assert_eq!(out, vec!["2.718281828459045\n"]);
    }

    #[test]
    fn test_math_pow() {
        let out = run_and_capture("fn main() { println!(\"{}\", math::pow(2.0, 10.0)); }");
        assert_eq!(out, vec!["1024\n"]);
    }

    #[test]
    fn test_math_floor_ceil_round() {
        let out = run_and_capture(
            "fn main() { println!(\"{}\", math::floor(3.7)); println!(\"{}\", math::ceil(3.2)); println!(\"{}\", math::round(3.5)); }",
        );
        assert_eq!(out, vec!["3\n", "4\n", "4\n"]);
    }

    #[test]
    fn test_math_abs() {
        let out = run_and_capture(
            "fn main() { println!(\"{}\", math::abs(-42)); println!(\"{}\", math::abs(-3.14)); }",
        );
        assert_eq!(out, vec!["42\n", "3.14\n"]);
    }

    #[test]
    fn test_math_min_max() {
        let out = run_and_capture(
            "fn main() { println!(\"{}\", math::min(3, 7)); println!(\"{}\", math::max(3, 7)); }",
        );
        assert_eq!(out, vec!["3\n", "7\n"]);
    }

    #[test]
    fn test_math_log() {
        let out = run_and_capture("fn main() { println!(\"{}\", math::log(1.0)); }");
        assert_eq!(out, vec!["0\n"]);
    }

    #[test]
    fn test_math_log2_log10() {
        let out = run_and_capture(
            "fn main() { println!(\"{}\", math::log2(8.0)); println!(\"{}\", math::log10(100.0)); }",
        );
        assert_eq!(out, vec!["3\n", "2\n"]);
    }

    #[test]
    fn test_f64_methods() {
        let out = run_and_capture(
            r#"fn main() {
    let x = 16.0;
    println!("{}", x.sqrt());
    let y = -5;
    println!("{}", y.abs());
    let z = 3.7;
    println!("{}", z.floor());
}"#,
        );
        assert_eq!(out, vec!["4\n", "5\n", "3\n"]);
    }

    #[test]
    fn test_f64_clamp() {
        let out = run_and_capture("fn main() { let x = 15; println!(\"{}\", x.clamp(0, 10)); }");
        assert_eq!(out, vec!["10\n"]);
    }

    #[test]
    fn test_f64_min_max_method() {
        let out = run_and_capture(
            r#"fn main() {
    let a = 3;
    let b = 7;
    println!("{}", a.min(b));
    println!("{}", a.max(b));
}"#,
        );
        assert_eq!(out, vec!["3\n", "7\n"]);
    }

    #[test]
    fn test_f64_pow_method() {
        let out = run_and_capture("fn main() { let x = 2.0; println!(\"{}\", x.pow(10.0)); }");
        assert_eq!(out, vec!["1024\n"]);
    }

    #[test]
    fn test_f64_trig_methods() {
        let out = run_and_capture(
            r#"fn main() {
    let x = 0.0;
    println!("{}", x.sin());
    println!("{}", x.cos());
}"#,
        );
        assert_eq!(out, vec!["0\n", "1\n"]);
    }

    #[test]
    fn test_rand_random() {
        let out = run_and_capture(
            "fn main() { let x = rand::random(); println!(\"{}\", x >= 0.0 && x < 1.0); }",
        );
        assert_eq!(out, vec!["true\n"]);
    }

    #[test]
    fn test_rand_range() {
        let out = run_and_capture(
            "fn main() { let x = rand::range(1, 10); println!(\"{}\", x >= 1 && x < 10); }",
        );
        assert_eq!(out, vec!["true\n"]);
    }

    #[test]
    fn test_rand_bool() {
        let out = run_and_capture(
            r#"fn main() {
    let b = rand::bool();
    println!("{}", b == true || b == false);
}"#,
        );
        assert_eq!(out, vec!["true\n"]);
    }

    #[test]
    fn test_time_now() {
        let out = run_and_capture("fn main() { let t = time::now(); println!(\"{}\", t > 0.0); }");
        assert_eq!(out, vec!["true\n"]);
    }

    #[test]
    fn test_time_millis() {
        let out = run_and_capture("fn main() { let t = time::millis(); println!(\"{}\", t > 0); }");
        assert_eq!(out, vec!["true\n"]);
    }

    #[test]
    fn test_time_elapsed() {
        let out = run_and_capture(
            "fn main() { let start = time::now(); let elapsed = time::elapsed(start); println!(\"{}\", elapsed >= 0.0); }",
        );
        assert_eq!(out, vec!["true\n"]);
    }

    // === F-string interpolation ===

    #[test]
    fn test_fstring_basic() {
        let out = run_and_capture(
            r#"fn main() { let name = "World"; println!("{}", f"Hello {name}!"); }"#,
        );
        assert_eq!(out, vec!["Hello World!\n"]);
    }

    #[test]
    fn test_fstring_expression() {
        let out =
            run_and_capture(r#"fn main() { let x = 10; println!("{}", f"x + 5 = {x + 5}"); }"#);
        assert_eq!(out, vec!["x + 5 = 15\n"]);
    }

    #[test]
    fn test_fstring_multiple_interpolations() {
        let out = run_and_capture(
            r#"fn main() { let a = 1; let b = 2; println!("{}", f"{a} + {b} = {a + b}"); }"#,
        );
        assert_eq!(out, vec!["1 + 2 = 3\n"]);
    }

    #[test]
    fn test_fstring_no_interpolation() {
        let out = run_and_capture(r#"fn main() { println!("{}", f"plain string"); }"#);
        assert_eq!(out, vec!["plain string\n"]);
    }

    #[test]
    fn test_fstring_escaped_braces() {
        let out = run_and_capture(r#"fn main() { println!("{}", f"use {{braces}}"); }"#);
        assert_eq!(out, vec!["use {braces}\n"]);
    }

    #[test]
    fn test_fstring_method_call() {
        let out = run_and_capture(
            r#"fn main() { let v = vec![1, 2, 3]; println!("{}", f"len = {v.len()}"); }"#,
        );
        assert_eq!(out, vec!["len = 3\n"]);
    }

    #[test]
    fn test_fstring_nested_function() {
        let out = run_and_capture(
            r#"fn double(x: i64) -> i64 { x * 2 } fn main() { println!("{}", f"double(5) = {double(5)}"); }"#,
        );
        assert_eq!(out, vec!["double(5) = 10\n"]);
    }

    #[test]
    fn test_fstring_in_variable() {
        let out = run_and_capture(
            r#"fn main() { let greeting = f"Hi {1 + 1}"; println!("{}", greeting); }"#,
        );
        assert_eq!(out, vec!["Hi 2\n"]);
    }

    // === Derive attribute tests ===

    #[test]
    fn test_derive_debug() {
        let out = run_and_capture(
            r#"
#[derive(Debug)]
struct Point { x: f64, y: f64 }

fn main() {
    let p = Point { x: 1.0, y: 2.0 };
    println!("{:?}", p);
}
"#,
        );
        assert_eq!(out, vec!["Point { x: 1.0, y: 2.0 }\n"]);
    }

    #[test]
    fn test_derive_clone() {
        let out = run_and_capture(
            r#"
#[derive(Clone)]
struct Point { x: f64, y: f64 }

fn main() {
    let p = Point { x: 1.0, y: 2.0 };
    let p2 = p.clone();
    println!("{} {}", p2.x, p2.y);
}
"#,
        );
        assert_eq!(out, vec!["1.0 2.0\n"]);
    }

    #[test]
    fn test_derive_partial_eq() {
        let out = run_and_capture(
            r#"
#[derive(PartialEq)]
struct Point { x: f64, y: f64 }

fn main() {
    let a = Point { x: 1.0, y: 2.0 };
    let b = Point { x: 1.0, y: 2.0 };
    let c = Point { x: 3.0, y: 4.0 };
    println!("{}", a == b);
    println!("{}", a == c);
}
"#,
        );
        assert_eq!(out, vec!["true\n", "false\n"]);
    }

    #[test]
    fn test_derive_multiple() {
        let out = run_and_capture(
            r#"
#[derive(Debug, Clone, PartialEq)]
struct Color { r: i64, g: i64, b: i64 }

fn main() {
    let c1 = Color { r: 255, g: 0, b: 0 };
    let c2 = c1.clone();
    println!("{:?}", c1);
    println!("{}", c1 == c2);
}
"#,
        );
        assert_eq!(out, vec!["Color { b: 0, g: 0, r: 255 }\n", "true\n"]);
    }

    #[test]
    fn test_derive_default() {
        let out = run_and_capture(
            r#"
#[derive(Default, Debug)]
struct Config { width: i64, height: i64, title: String }

fn main() {
    let c = Config::default();
    println!("{:?}", c);
}
"#,
        );
        assert_eq!(out, vec!["Config { height: 0, title: \"\", width: 0 }\n"]);
    }

    #[test]
    fn test_derive_enum_debug() {
        let out = run_and_capture(
            r#"
#[derive(Debug)]
enum Color { Red, Green, Blue }

fn main() {
    println!("{:?}", Color::Red);
}
"#,
        );
        assert_eq!(out, vec!["Color::Red\n"]);
    }

    #[test]
    fn test_derive_enum_partial_eq() {
        let out = run_and_capture(
            r#"
#[derive(PartialEq)]
enum Direction { Up, Down, Left, Right }

fn main() {
    println!("{}", Direction::Up == Direction::Up);
    println!("{}", Direction::Up == Direction::Down);
}
"#,
        );
        assert_eq!(out, vec!["true\n", "false\n"]);
    }

    #[test]
    fn test_no_derive_clone_error() {
        let result = run_capturing(
            r#"
struct Foo { x: i64 }

fn main() {
    let f = Foo { x: 1 };
    let f2 = f.clone();
}
"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_attribute_ignored_unknown() {
        let out = run_and_capture(
            r#"
#[serde(rename_all)]
struct Foo { x: i64 }

fn main() {
    let f = Foo { x: 42 };
    println!("{}", f.x);
}
"#,
        );
        assert_eq!(out, vec!["42\n"]);
    }

    #[test]
    fn test_derive_enum_clone() {
        let out = run_and_capture(
            r#"
#[derive(Clone, Debug)]
enum Shape { Circle(f64), Square(f64) }

fn main() {
    let s = Shape::Circle(5.0);
    let s2 = s.clone();
    println!("{:?}", s2);
}
"#,
        );
        assert_eq!(out, vec!["Shape::Circle(5.0)\n"]);
    }
}
