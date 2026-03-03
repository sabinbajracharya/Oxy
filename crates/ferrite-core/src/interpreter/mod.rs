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
    /// Create a new interpreter with a fresh global environment.
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
            output: None,
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
            trait_defs: HashMap::new(),
            trait_impls: HashMap::new(),
            current_self_type: None,
            modules: HashMap::new(),
            base_dir: None,
            type_aliases: HashMap::new(),
            cli_args: Vec::new(),
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
            trait_defs: HashMap::new(),
            trait_impls: HashMap::new(),
            current_self_type: None,
            modules: HashMap::new(),
            base_dir: None,
            type_aliases: HashMap::new(),
            cli_args: Vec::new(),
        }
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
                    let value = Value::Function {
                        name: f.name.clone(),
                        params: f.params.clone(),
                        return_type: f.return_type.clone(),
                        body: f.body.clone(),
                        closure_env: Rc::clone(&mod_env),
                    };
                    mod_env.borrow_mut().define(f.name.clone(), value, false);
                }
                Item::Struct(s) => {
                    mod_struct_defs.insert(s.name.clone(), s.clone());
                }
                Item::Enum(e) => {
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

        // Try `name.fe` first, then `name/mod.fe`
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
                // Built-in constants
                if name == "None" {
                    return Ok(Value::EnumVariant {
                        enum_name: "Option".to_string(),
                        variant: "None".to_string(),
                        data: vec![],
                    });
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

            Expr::Call { callee, args, span } => {
                // Handle built-in constructors: Some(x), Ok(x), Err(e)
                if let Expr::Ident(name, _) = callee.as_ref() {
                    match name.as_str() {
                        "Some" => {
                            if args.len() != 1 {
                                return Err(FerriError::Runtime {
                                    message: format!(
                                        "Some() takes exactly 1 argument, got {}",
                                        args.len()
                                    ),
                                    line: span.line,
                                    column: span.column,
                                });
                            }
                            let val = self.eval_expr(&args[0], env)?;
                            return Ok(Value::EnumVariant {
                                enum_name: "Option".to_string(),
                                variant: "Some".to_string(),
                                data: vec![val],
                            });
                        }
                        "Ok" => {
                            if args.len() != 1 {
                                return Err(FerriError::Runtime {
                                    message: format!(
                                        "Ok() takes exactly 1 argument, got {}",
                                        args.len()
                                    ),
                                    line: span.line,
                                    column: span.column,
                                });
                            }
                            let val = self.eval_expr(&args[0], env)?;
                            return Ok(Value::EnumVariant {
                                enum_name: "Result".to_string(),
                                variant: "Ok".to_string(),
                                data: vec![val],
                            });
                        }
                        "Err" => {
                            if args.len() != 1 {
                                return Err(FerriError::Runtime {
                                    message: format!(
                                        "Err() takes exactly 1 argument, got {}",
                                        args.len()
                                    ),
                                    line: span.line,
                                    column: span.column,
                                });
                            }
                            let val = self.eval_expr(&args[0], env)?;
                            return Ok(Value::EnumVariant {
                                enum_name: "Result".to_string(),
                                variant: "Err".to_string(),
                                data: vec![val],
                            });
                        }
                        _ => {}
                    }
                }
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

            Expr::IfLet {
                pattern,
                expr,
                then_block,
                else_block,
                ..
            } => {
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

            Expr::Try { expr, span } => {
                let val = self.eval_expr(expr, env)?;
                match &val {
                    // Some(x) → unwrap to x
                    Value::EnumVariant {
                        enum_name,
                        variant,
                        data,
                        ..
                    } if enum_name == "Option" && variant == "Some" => {
                        Ok(data.first().cloned().unwrap_or(Value::Unit))
                    }
                    // None → return None early
                    Value::EnumVariant {
                        enum_name, variant, ..
                    } if enum_name == "Option" && variant == "None" => {
                        Err(FerriError::Return(Box::new(val)))
                    }
                    // Ok(x) → unwrap to x
                    Value::EnumVariant {
                        enum_name,
                        variant,
                        data,
                        ..
                    } if enum_name == "Result" && variant == "Ok" => {
                        Ok(data.first().cloned().unwrap_or(Value::Unit))
                    }
                    // Err(e) → return Err(e) early
                    Value::EnumVariant {
                        enum_name, variant, ..
                    } if enum_name == "Result" && variant == "Err" => {
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
            Expr::Closure {
                params,
                return_type,
                body,
                ..
            } => {
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
                let closure_body = match body.as_ref() {
                    Expr::Block(block) => block.clone(),
                    expr => Block {
                        stmts: vec![Stmt::Expr {
                            expr: expr.clone(),
                            has_semicolon: false,
                        }],
                        span: expr.span(),
                    },
                };

                Ok(Value::Function {
                    name: "<closure>".to_string(),
                    params: fn_params,
                    return_type: return_type.clone(),
                    body: closure_body,
                    closure_env: env.clone(),
                })
            }
        }
    }

    // === Binary operations ===

    fn eval_binary_op(
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
                // Try operator overloading for user types
                let trait_name = match op {
                    BinOp::Add => Some("Add"),
                    BinOp::Sub => Some("Sub"),
                    BinOp::Mul => Some("Mul"),
                    BinOp::Div => Some("Div"),
                    _ => None,
                };
                let method_name = match op {
                    BinOp::Add => Some("add"),
                    BinOp::Sub => Some("sub"),
                    BinOp::Mul => Some("mul"),
                    BinOp::Div => Some("div"),
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
            Value::HashMap(m) => {
                // Iterate as (key, value) tuples
                let mut pairs: Vec<_> = m
                    .iter()
                    .map(|(k, v)| Value::Tuple(vec![Value::String(k.clone()), v.clone()]))
                    .collect();
                pairs.sort_by(|a, b| {
                    if let (Value::Tuple(a), Value::Tuple(b)) = (a, b) {
                        a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal)
                    } else {
                        std::cmp::Ordering::Equal
                    }
                });
                Ok(pairs)
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
            "vec" => {
                let vals: Vec<Value> = args
                    .iter()
                    .map(|a| self.eval_expr(a, env))
                    .collect::<Result<_, _>>()?;
                Ok(Value::Vec(vals))
            }
            "format" => {
                let output = self.format_macro_args(args, env, line, col)?;
                Ok(Value::String(output))
            }
            "eprintln" => {
                let output = self.format_macro_args(args, env, line, col)?;
                eprintln!("{output}");
                Ok(Value::Unit)
            }
            "panic" => {
                let output = if args.is_empty() {
                    "explicit panic".to_string()
                } else {
                    self.format_macro_args(args, env, line, col)?
                };
                Err(FerriError::Runtime {
                    message: format!("panic: {output}"),
                    line,
                    column: col,
                })
            }
            "todo" => Err(FerriError::Runtime {
                message: "not yet implemented".to_string(),
                line,
                column: col,
            }),
            "unimplemented" => Err(FerriError::Runtime {
                message: "not implemented".to_string(),
                line,
                column: col,
            }),
            "dbg" => {
                // dbg! prints debug output and returns the value
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("dbg!() takes 1 argument, got {}", args.len()),
                        line,
                        column: col,
                    });
                }
                let val = self.eval_expr(&args[0], env)?;
                let debug = debug_format(&val);
                self.write_output(&format!("[dbg] {debug}\n"));
                Ok(val)
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
            Value::HashMap(_) => {
                self.call_hashmap_method(receiver, method, args, receiver_expr, env, span)
            }
            Value::Tuple(_) => self.try_to_json_method(receiver, method, span, "tuple"),
            Value::Struct { name, .. }
            | Value::EnumVariant {
                enum_name: name, ..
            } => {
                // Built-in Option/Result methods
                if let Value::EnumVariant { enum_name, .. } = &receiver {
                    if enum_name == "Option" || enum_name == "Result" {
                        if let Some(result) =
                            self.call_option_result_method(&receiver, method, &args, span)?
                        {
                            return Ok(result);
                        }
                    }
                }
                let type_name = name.clone();
                self.call_user_method(receiver, &type_name, method, args, receiver_expr, env, span)
            }
            _ => {
                // Built-in .to_json() and .to_json_pretty() on all values
                if method == "to_json" || method == "to_json_pretty" {
                    let result = if method == "to_json" {
                        crate::json::serialize(&receiver)
                    } else {
                        crate::json::serialize_pretty(&receiver)
                    };
                    return match result {
                        Ok(json) => Ok(Value::EnumVariant {
                            enum_name: "Result".to_string(),
                            variant: "Ok".to_string(),
                            data: vec![Value::String(json)],
                        }),
                        Err(e) => Ok(Value::EnumVariant {
                            enum_name: "Result".to_string(),
                            variant: "Err".to_string(),
                            data: vec![Value::String(e)],
                        }),
                    };
                }
                Err(FerriError::Runtime {
                    message: format!("no method `{method}` on type {}", receiver.type_name()),
                    line: span.line,
                    column: span.column,
                })
            }
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
                    Some(val) => Ok(Value::EnumVariant {
                        enum_name: "Option".to_string(),
                        variant: "Some".to_string(),
                        data: vec![val],
                    }),
                    None => Ok(Value::EnumVariant {
                        enum_name: "Option".to_string(),
                        variant: "None".to_string(),
                        data: vec![],
                    }),
                }
            }
            "first" => {
                let result = v.first().cloned();
                match result {
                    Some(val) => Ok(Value::EnumVariant {
                        enum_name: "Option".to_string(),
                        variant: "Some".to_string(),
                        data: vec![val],
                    }),
                    None => Ok(Value::EnumVariant {
                        enum_name: "Option".to_string(),
                        variant: "None".to_string(),
                        data: vec![],
                    }),
                }
            }
            "last" => {
                let result = v.last().cloned();
                match result {
                    Some(val) => Ok(Value::EnumVariant {
                        enum_name: "Option".to_string(),
                        variant: "Some".to_string(),
                        data: vec![val],
                    }),
                    None => Ok(Value::EnumVariant {
                        enum_name: "Option".to_string(),
                        variant: "None".to_string(),
                        data: vec![],
                    }),
                }
            }
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
            // iter() returns the Vec itself (we don't have a separate Iterator type)
            "iter" | "into_iter" | "iter_mut" => Ok(Value::Vec(v)),
            // map(|x| expr) — applies closure to each element
            "map" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Vec::map() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let func = &args[0];
                let mut result = Vec::new();
                for item in &v {
                    let mapped =
                        self.call_function(func, &[item.clone()], span.line, span.column)?;
                    result.push(mapped);
                }
                Ok(Value::Vec(result))
            }
            // filter(|x| bool_expr) — keeps elements where closure returns true
            "filter" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Vec::filter() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let func = &args[0];
                let mut result = Vec::new();
                for item in &v {
                    let keep = self.call_function(func, &[item.clone()], span.line, span.column)?;
                    if keep.is_truthy() {
                        result.push(item.clone());
                    }
                }
                Ok(Value::Vec(result))
            }
            // for_each(|x| { ... }) — runs closure on each element
            "for_each" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Vec::for_each() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let func = &args[0];
                for item in &v {
                    self.call_function(func, &[item.clone()], span.line, span.column)?;
                }
                Ok(Value::Unit)
            }
            // fold(init, |acc, x| expr) — reduces to a single value
            "fold" => {
                if args.len() != 2 {
                    return Err(FerriError::Runtime {
                        message: format!("Vec::fold() takes 2 arguments, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let mut acc = args[0].clone();
                let func = &args[1];
                for item in &v {
                    acc = self.call_function(func, &[acc, item.clone()], span.line, span.column)?;
                }
                Ok(acc)
            }
            // any(|x| bool_expr) — returns true if any element matches
            "any" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Vec::any() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let func = &args[0];
                for item in &v {
                    let result =
                        self.call_function(func, &[item.clone()], span.line, span.column)?;
                    if result.is_truthy() {
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(false))
            }
            // all(|x| bool_expr) — returns true if all elements match
            "all" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Vec::all() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let func = &args[0];
                for item in &v {
                    let result =
                        self.call_function(func, &[item.clone()], span.line, span.column)?;
                    if !result.is_truthy() {
                        return Ok(Value::Bool(false));
                    }
                }
                Ok(Value::Bool(true))
            }
            // find(|x| bool_expr) — returns Option<T>
            "find" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Vec::find() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let func = &args[0];
                for item in &v {
                    let result =
                        self.call_function(func, &[item.clone()], span.line, span.column)?;
                    if result.is_truthy() {
                        return Ok(Value::EnumVariant {
                            enum_name: "Option".to_string(),
                            variant: "Some".to_string(),
                            data: vec![item.clone()],
                        });
                    }
                }
                Ok(Value::EnumVariant {
                    enum_name: "Option".to_string(),
                    variant: "None".to_string(),
                    data: vec![],
                })
            }
            // enumerate() — returns Vec of (index, element) tuples
            "enumerate" => {
                let result: Vec<Value> = v
                    .iter()
                    .enumerate()
                    .map(|(i, item)| Value::Tuple(vec![Value::Integer(i as i64), item.clone()]))
                    .collect();
                Ok(Value::Vec(result))
            }
            // collect() — identity on Vec (already collected)
            "collect" => Ok(Value::Vec(v)),
            // flat_map(|x| vec_expr) — maps and flattens
            "flat_map" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Vec::flat_map() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let func = &args[0];
                let mut result = Vec::new();
                for item in &v {
                    let mapped =
                        self.call_function(func, &[item.clone()], span.line, span.column)?;
                    match mapped {
                        Value::Vec(inner) => result.extend(inner),
                        other => result.push(other),
                    }
                }
                Ok(Value::Vec(result))
            }
            // position(|x| bool_expr) — returns Option<usize>
            "position" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("Vec::position() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let func = &args[0];
                for (i, item) in v.iter().enumerate() {
                    let result =
                        self.call_function(func, &[item.clone()], span.line, span.column)?;
                    if result.is_truthy() {
                        return Ok(Value::EnumVariant {
                            enum_name: "Option".to_string(),
                            variant: "Some".to_string(),
                            data: vec![Value::Integer(i as i64)],
                        });
                    }
                }
                Ok(Value::EnumVariant {
                    enum_name: "Option".to_string(),
                    variant: "None".to_string(),
                    data: vec![],
                })
            }
            _ => self.try_to_json_method(Value::Vec(v), method, span, "Vec"),
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
            "clone" => Ok(Value::String(s)),
            "to_string" => Ok(Value::String(s)),
            _ => self.try_to_json_method(Value::String(s), method, span, "String"),
        }
    }

    fn call_hashmap_method(
        &mut self,
        receiver: Value,
        method: &str,
        args: Vec<Value>,
        receiver_expr: &Expr,
        env: &Env,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let Value::HashMap(m) = receiver else {
            unreachable!()
        };
        match method {
            "len" => Ok(Value::Integer(m.len() as i64)),
            "is_empty" => Ok(Value::Bool(m.is_empty())),
            "insert" => {
                if args.len() != 2 {
                    return Err(FerriError::Runtime {
                        message: format!("HashMap::insert() takes 2 arguments, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let key = format!("{}", args[0]);
                let value = args[1].clone();
                let mut new_m = m;
                new_m.insert(key, value);
                self.mutate_variable(receiver_expr, Value::HashMap(new_m), env, span)?;
                Ok(Value::Unit)
            }
            "get" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("HashMap::get() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let key = format!("{}", args[0]);
                match m.get(&key) {
                    Some(val) => Ok(Value::EnumVariant {
                        enum_name: "Option".to_string(),
                        variant: "Some".to_string(),
                        data: vec![val.clone()],
                    }),
                    None => Ok(Value::EnumVariant {
                        enum_name: "Option".to_string(),
                        variant: "None".to_string(),
                        data: vec![],
                    }),
                }
            }
            "remove" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("HashMap::remove() takes 1 argument, got {}", args.len()),
                        line: span.line,
                        column: span.column,
                    });
                }
                let key = format!("{}", args[0]);
                let mut new_m = m;
                let removed = new_m.remove(&key);
                self.mutate_variable(receiver_expr, Value::HashMap(new_m), env, span)?;
                match removed {
                    Some(val) => Ok(Value::EnumVariant {
                        enum_name: "Option".to_string(),
                        variant: "Some".to_string(),
                        data: vec![val],
                    }),
                    None => Ok(Value::EnumVariant {
                        enum_name: "Option".to_string(),
                        variant: "None".to_string(),
                        data: vec![],
                    }),
                }
            }
            "contains_key" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!(
                            "HashMap::contains_key() takes 1 argument, got {}",
                            args.len()
                        ),
                        line: span.line,
                        column: span.column,
                    });
                }
                let key = format!("{}", args[0]);
                Ok(Value::Bool(m.contains_key(&key)))
            }
            "keys" => {
                let mut keys: Vec<String> = m.keys().cloned().collect();
                keys.sort();
                Ok(Value::Vec(keys.into_iter().map(Value::String).collect()))
            }
            "values" => {
                let mut pairs: Vec<(&String, &Value)> = m.iter().collect();
                pairs.sort_by_key(|(k, _)| (*k).clone());
                Ok(Value::Vec(
                    pairs.into_iter().map(|(_, v)| v.clone()).collect(),
                ))
            }
            _ => self.try_to_json_method(Value::HashMap(m), method, span, "HashMap"),
        }
    }

    /// Handle built-in Option/Result methods.
    /// Returns `Ok(Some(value))` if the method was handled, `Ok(None)` if not found.
    fn call_option_result_method(
        &mut self,
        receiver: &Value,
        method: &str,
        args: &[Value],
        span: &Span,
    ) -> Result<Option<Value>, FerriError> {
        let Value::EnumVariant {
            enum_name,
            variant,
            data,
            ..
        } = receiver
        else {
            return Ok(None);
        };

        match (enum_name.as_str(), method) {
            // === Option methods ===
            ("Option", "is_some") => Ok(Some(Value::Bool(variant == "Some"))),
            ("Option", "is_none") => Ok(Some(Value::Bool(variant == "None"))),
            ("Option", "unwrap") => {
                if variant == "Some" {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else {
                    Err(FerriError::Runtime {
                        message: "called `Option::unwrap()` on a `None` value".to_string(),
                        line: span.line,
                        column: span.column,
                    })
                }
            }
            ("Option", "expect") => {
                if variant == "Some" {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else {
                    let msg = args
                        .first()
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "Option::expect failed".to_string());
                    Err(FerriError::Runtime {
                        message: msg,
                        line: span.line,
                        column: span.column,
                    })
                }
            }
            ("Option", "unwrap_or") => {
                if variant == "Some" {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else {
                    Ok(Some(args.first().cloned().unwrap_or(Value::Unit)))
                }
            }
            ("Option", "unwrap_or_else") => {
                if variant == "Some" {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else if let Some(Value::Function { .. }) = args.first() {
                    let result = self.call_function(&args[0], &[], span.line, span.column)?;
                    Ok(Some(result))
                } else {
                    Ok(Some(Value::Unit))
                }
            }
            ("Option", "map") => {
                if variant == "Some" {
                    if let Some(func) = args.first() {
                        let inner = data.first().cloned().unwrap_or(Value::Unit);
                        let result = self.call_function(func, &[inner], span.line, span.column)?;
                        Ok(Some(Value::EnumVariant {
                            enum_name: "Option".to_string(),
                            variant: "Some".to_string(),
                            data: vec![result],
                        }))
                    } else {
                        Ok(Some(receiver.clone()))
                    }
                } else {
                    // None.map(f) → None
                    Ok(Some(receiver.clone()))
                }
            }
            ("Option", "and_then") => {
                if variant == "Some" {
                    if let Some(func) = args.first() {
                        let inner = data.first().cloned().unwrap_or(Value::Unit);
                        let result = self.call_function(func, &[inner], span.line, span.column)?;
                        Ok(Some(result))
                    } else {
                        Ok(Some(receiver.clone()))
                    }
                } else {
                    Ok(Some(receiver.clone()))
                }
            }

            // === Result methods ===
            ("Result", "is_ok") => Ok(Some(Value::Bool(variant == "Ok"))),
            ("Result", "is_err") => Ok(Some(Value::Bool(variant == "Err"))),
            ("Result", "unwrap") => {
                if variant == "Ok" {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else {
                    let err_val = data
                        .first()
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "unknown error".to_string());
                    Err(FerriError::Runtime {
                        message: format!("called `Result::unwrap()` on an `Err` value: {err_val}"),
                        line: span.line,
                        column: span.column,
                    })
                }
            }
            ("Result", "expect") => {
                if variant == "Ok" {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else {
                    let msg = args
                        .first()
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "Result::expect failed".to_string());
                    Err(FerriError::Runtime {
                        message: msg,
                        line: span.line,
                        column: span.column,
                    })
                }
            }
            ("Result", "unwrap_err") => {
                if variant == "Err" {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else {
                    Err(FerriError::Runtime {
                        message: "called `Result::unwrap_err()` on an `Ok` value".to_string(),
                        line: span.line,
                        column: span.column,
                    })
                }
            }
            ("Result", "unwrap_or") => {
                if variant == "Ok" {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else {
                    Ok(Some(args.first().cloned().unwrap_or(Value::Unit)))
                }
            }
            ("Result", "unwrap_or_else") => {
                if variant == "Ok" {
                    Ok(Some(data.first().cloned().unwrap_or(Value::Unit)))
                } else if let Some(Value::Function { .. }) = args.first() {
                    let err_val = data.first().cloned().unwrap_or(Value::Unit);
                    let result =
                        self.call_function(&args[0], &[err_val], span.line, span.column)?;
                    Ok(Some(result))
                } else {
                    Ok(Some(Value::Unit))
                }
            }
            ("Result", "map") => {
                if variant == "Ok" {
                    if let Some(func) = args.first() {
                        let inner = data.first().cloned().unwrap_or(Value::Unit);
                        let result = self.call_function(func, &[inner], span.line, span.column)?;
                        Ok(Some(Value::EnumVariant {
                            enum_name: "Result".to_string(),
                            variant: "Ok".to_string(),
                            data: vec![result],
                        }))
                    } else {
                        Ok(Some(receiver.clone()))
                    }
                } else {
                    // Err(e).map(f) → Err(e)
                    Ok(Some(receiver.clone()))
                }
            }
            ("Result", "map_err") => {
                if variant == "Err" {
                    if let Some(func) = args.first() {
                        let inner = data.first().cloned().unwrap_or(Value::Unit);
                        let result = self.call_function(func, &[inner], span.line, span.column)?;
                        Ok(Some(Value::EnumVariant {
                            enum_name: "Result".to_string(),
                            variant: "Err".to_string(),
                            data: vec![result],
                        }))
                    } else {
                        Ok(Some(receiver.clone()))
                    }
                } else {
                    Ok(Some(receiver.clone()))
                }
            }
            ("Result", "and_then") => {
                if variant == "Ok" {
                    if let Some(func) = args.first() {
                        let inner = data.first().cloned().unwrap_or(Value::Unit);
                        let result = self.call_function(func, &[inner], span.line, span.column)?;
                        Ok(Some(result))
                    } else {
                        Ok(Some(receiver.clone()))
                    }
                } else {
                    Ok(Some(receiver.clone()))
                }
            }
            ("Result", "ok") => {
                if variant == "Ok" {
                    Ok(Some(Value::EnumVariant {
                        enum_name: "Option".to_string(),
                        variant: "Some".to_string(),
                        data: data.clone(),
                    }))
                } else {
                    Ok(Some(Value::EnumVariant {
                        enum_name: "Option".to_string(),
                        variant: "None".to_string(),
                        data: vec![],
                    }))
                }
            }
            ("Result", "err") => {
                if variant == "Err" {
                    Ok(Some(Value::EnumVariant {
                        enum_name: "Option".to_string(),
                        variant: "Some".to_string(),
                        data: data.clone(),
                    }))
                } else {
                    Ok(Some(Value::EnumVariant {
                        enum_name: "Option".to_string(),
                        variant: "None".to_string(),
                        data: vec![],
                    }))
                }
            }

            _ => Ok(None),
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

            // Check for associated functions in trait impls
            for ((tn, _), methods) in &self.trait_impls.clone() {
                if tn == type_name {
                    for method_def in methods {
                        if method_def.name == *method_name {
                            let is_method =
                                method_def.params.first().is_some_and(|p| p.name == "self");
                            if !is_method {
                                let func_env = Environment::child(env);
                                for (param, arg) in method_def.params.iter().zip(args.iter()) {
                                    func_env.borrow_mut().define(
                                        param.name.clone(),
                                        arg.clone(),
                                        true,
                                    );
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
            }

            // Built-in String::from
            if type_name == "String" && method_name == "from" && args.len() == 1 {
                return Ok(Value::String(format!("{}", args[0])));
            }

            // Built-in HashMap::new
            if type_name == "HashMap" && method_name == "new" && args.is_empty() {
                return Ok(Value::HashMap(HashMap::new()));
            }

            // Built-in json:: pseudo-module
            if type_name == "json" {
                return self.call_json_function(method_name, args, span);
            }

            // Check for module function call: `module::function(args)`
            if let Some(module) = self.modules.get(type_name).cloned() {
                if let Ok(val) = module.env.borrow().get(method_name) {
                    if let Value::Function { .. } = &val {
                        return self.call_function(&val, args, span.line, span.column);
                    }
                }
                // Check for enum variant in module
                if let Some(edef) = module.enum_defs.get(method_name) {
                    // This handles module::EnumName — but the variant is the next segment
                    // For 2-segment, this is module::function, already handled above
                    let _ = edef; // suppress unused
                }
            }
        }

        // Handle 3-segment paths: `module::Type::method(args)`
        if path.len() == 3 {
            let mod_name = &path[0];
            let type_name = &path[1];
            let method_name = &path[2];

            if let Some(module) = self.modules.get(mod_name).cloned() {
                // Check for enum variant constructor in module
                if let Some(edef) = module.enum_defs.get(type_name) {
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
                // Check for associated function in module
                if let Some(methods) = module.impl_methods.get(type_name) {
                    for method_def in methods {
                        if method_def.name == *method_name {
                            let func_env = Environment::child(env);
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
        }

        // Handle std:: paths
        if path.len() == 3 && path[0] == "std" {
            let module = &path[1];
            let func = &path[2];
            match (module.as_str(), func.as_str()) {
                ("fs", "read_to_string") => {
                    if args.len() != 1 {
                        return Err(FerriError::Runtime {
                            message: "std::fs::read_to_string() takes 1 argument".into(),
                            line: span.line,
                            column: span.column,
                        });
                    }
                    let path_str = format!("{}", args[0]);
                    return match std::fs::read_to_string(&path_str) {
                        Ok(content) => Ok(Value::EnumVariant {
                            enum_name: "Result".to_string(),
                            variant: "Ok".to_string(),
                            data: vec![Value::String(content)],
                        }),
                        Err(e) => Ok(Value::EnumVariant {
                            enum_name: "Result".to_string(),
                            variant: "Err".to_string(),
                            data: vec![Value::String(e.to_string())],
                        }),
                    };
                }
                ("fs", "write") => {
                    if args.len() != 2 {
                        return Err(FerriError::Runtime {
                            message: "std::fs::write() takes 2 arguments".into(),
                            line: span.line,
                            column: span.column,
                        });
                    }
                    let path_str = format!("{}", args[0]);
                    let content = format!("{}", args[1]);
                    return match std::fs::write(&path_str, &content) {
                        Ok(()) => Ok(Value::EnumVariant {
                            enum_name: "Result".to_string(),
                            variant: "Ok".to_string(),
                            data: vec![Value::Unit],
                        }),
                        Err(e) => Ok(Value::EnumVariant {
                            enum_name: "Result".to_string(),
                            variant: "Err".to_string(),
                            data: vec![Value::String(e.to_string())],
                        }),
                    };
                }
                ("env", "args") => {
                    let args_vec: Vec<Value> = self
                        .cli_args
                        .iter()
                        .map(|a| Value::String(a.clone()))
                        .collect();
                    return Ok(Value::Vec(args_vec));
                }
                ("process", "exit") => {
                    if args.len() != 1 {
                        return Err(FerriError::Runtime {
                            message: "std::process::exit() takes 1 argument".into(),
                            line: span.line,
                            column: span.column,
                        });
                    }
                    if let Value::Integer(code) = &args[0] {
                        std::process::exit(*code as i32);
                    } else {
                        return Err(FerriError::Runtime {
                            message: "std::process::exit() requires an integer argument".into(),
                            line: span.line,
                            column: span.column,
                        });
                    }
                }
                _ => {}
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

            // Module value access: `module::value`
            if let Some(module) = self.modules.get(type_name) {
                if let Ok(val) = module.env.borrow().get(variant_name) {
                    return Ok(val);
                }
            }
        }

        // 3-segment: `module::Type::Variant`
        if segments.len() == 3 {
            let mod_name = &segments[0];
            let type_name = &segments[1];
            let variant_name = &segments[2];

            if let Some(module) = self.modules.get(mod_name) {
                if let Some(edef) = module.enum_defs.get(type_name) {
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
        // First, search direct impl methods
        if let Some(methods) = self.impl_methods.get(type_name).cloned() {
            for method_def in &methods {
                if method_def.name == method {
                    return self.dispatch_method(
                        &method_def.clone(),
                        receiver,
                        type_name,
                        args,
                        receiver_expr,
                        env,
                        span,
                    );
                }
            }
        }

        // Then search trait impl methods
        if let Some(method_def) = self.find_trait_method(type_name, method) {
            return self.dispatch_method(
                &method_def,
                receiver,
                type_name,
                args,
                receiver_expr,
                env,
                span,
            );
        }

        // Built-in to_json / to_json_pretty
        if method == "to_json" || method == "to_json_pretty" {
            return self.try_to_json_method(receiver, method, span, type_name);
        }

        Err(FerriError::Runtime {
            message: format!("no method `{method}` found for type `{type_name}`"),
            line: span.line,
            column: span.column,
        })
    }

    fn try_to_json_method(
        &self,
        receiver: Value,
        method: &str,
        span: &Span,
        type_name: &str,
    ) -> Result<Value, FerriError> {
        if method == "to_json" || method == "to_json_pretty" {
            let result = if method == "to_json" {
                crate::json::serialize(&receiver)
            } else {
                crate::json::serialize_pretty(&receiver)
            };
            return match result {
                Ok(json) => Ok(Value::EnumVariant {
                    enum_name: "Result".to_string(),
                    variant: "Ok".to_string(),
                    data: vec![Value::String(json)],
                }),
                Err(e) => Ok(Value::EnumVariant {
                    enum_name: "Result".to_string(),
                    variant: "Err".to_string(),
                    data: vec![Value::String(e)],
                }),
            };
        }
        Err(FerriError::Runtime {
            message: format!("no method `{method}` on {type_name}"),
            line: span.line,
            column: span.column,
        })
    }

    fn call_json_function(
        &self,
        func_name: &str,
        args: &[Value],
        span: &Span,
    ) -> Result<Value, FerriError> {
        match func_name {
            "serialize" | "to_string" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("json::{func_name}() takes 1 argument"),
                        line: span.line,
                        column: span.column,
                    });
                }
                match crate::json::serialize(&args[0]) {
                    Ok(json) => Ok(Value::EnumVariant {
                        enum_name: "Result".to_string(),
                        variant: "Ok".to_string(),
                        data: vec![Value::String(json)],
                    }),
                    Err(e) => Ok(Value::EnumVariant {
                        enum_name: "Result".to_string(),
                        variant: "Err".to_string(),
                        data: vec![Value::String(e)],
                    }),
                }
            }
            "to_string_pretty" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: "json::to_string_pretty() takes 1 argument".into(),
                        line: span.line,
                        column: span.column,
                    });
                }
                match crate::json::serialize_pretty(&args[0]) {
                    Ok(json) => Ok(Value::EnumVariant {
                        enum_name: "Result".to_string(),
                        variant: "Ok".to_string(),
                        data: vec![Value::String(json)],
                    }),
                    Err(e) => Ok(Value::EnumVariant {
                        enum_name: "Result".to_string(),
                        variant: "Err".to_string(),
                        data: vec![Value::String(e)],
                    }),
                }
            }
            "deserialize" | "parse" | "from_str" => {
                if args.len() != 1 {
                    return Err(FerriError::Runtime {
                        message: format!("json::{func_name}() takes 1 argument"),
                        line: span.line,
                        column: span.column,
                    });
                }
                let s = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!(
                                "json::{func_name}() expects a string, got {}",
                                other.type_name()
                            ),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                match crate::json::deserialize(&s) {
                    Ok(val) => Ok(Value::EnumVariant {
                        enum_name: "Result".to_string(),
                        variant: "Ok".to_string(),
                        data: vec![val],
                    }),
                    Err(e) => Ok(Value::EnumVariant {
                        enum_name: "Result".to_string(),
                        variant: "Err".to_string(),
                        data: vec![Value::String(e)],
                    }),
                }
            }
            "from_struct" => {
                if args.len() != 2 {
                    return Err(FerriError::Runtime {
                        message:
                            "json::from_struct() takes 2 arguments (json_string, \"StructName\")"
                                .into(),
                        line: span.line,
                        column: span.column,
                    });
                }
                let json_str = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!(
                                "json::from_struct() first argument must be a string, got {}",
                                other.type_name()
                            ),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                let struct_name = match &args[1] {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(FerriError::Runtime {
                            message: format!(
                                "json::from_struct() second argument must be a string, got {}",
                                other.type_name()
                            ),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                self.json_from_struct(&json_str, &struct_name, span)
            }
            _ => Err(FerriError::Runtime {
                message: format!("unknown json function: {func_name}"),
                line: span.line,
                column: span.column,
            }),
        }
    }

    fn json_from_struct(
        &self,
        json_str: &str,
        struct_name: &str,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let parsed = match crate::json::deserialize(json_str) {
            Ok(val) => val,
            Err(e) => {
                return Ok(Value::EnumVariant {
                    enum_name: "Result".to_string(),
                    variant: "Err".to_string(),
                    data: vec![Value::String(e)],
                })
            }
        };

        let map = match parsed {
            Value::HashMap(m) => m,
            _ => {
                return Ok(Value::EnumVariant {
                    enum_name: "Result".to_string(),
                    variant: "Err".to_string(),
                    data: vec![Value::String("JSON value is not an object".to_string())],
                })
            }
        };

        let sdef = match self.struct_defs.get(struct_name) {
            Some(sd) => sd.clone(),
            None => {
                return Err(FerriError::Runtime {
                    message: format!("unknown struct type: {struct_name}"),
                    line: span.line,
                    column: span.column,
                })
            }
        };

        use crate::ast::StructKind;
        match &sdef.kind {
            StructKind::Named(fields) => {
                let mut result_fields = std::collections::HashMap::new();
                for field in fields {
                    if let Some(val) = map.get(&field.name) {
                        result_fields.insert(field.name.clone(), val.clone());
                    } else {
                        result_fields.insert(field.name.clone(), Value::Unit);
                    }
                }
                Ok(Value::EnumVariant {
                    enum_name: "Result".to_string(),
                    variant: "Ok".to_string(),
                    data: vec![Value::Struct {
                        name: struct_name.to_string(),
                        fields: result_fields,
                    }],
                })
            }
            _ => Ok(Value::EnumVariant {
                enum_name: "Result".to_string(),
                variant: "Err".to_string(),
                data: vec![Value::String(format!(
                    "json::from_struct only supports named-field structs, not {struct_name}"
                ))],
            }),
        }
    }

    /// Search trait impls for a method, including default implementations.
    fn find_trait_method(&self, type_name: &str, method: &str) -> Option<FnDef> {
        // Search all trait impls for this type
        for ((tn, trait_name), methods) in &self.trait_impls {
            if tn == type_name {
                // Check explicit impl methods first
                for m in methods {
                    if m.name == method {
                        return Some(m.clone());
                    }
                }
                // Check default methods from the trait definition
                if let Some(trait_def) = self.trait_defs.get(trait_name) {
                    for m in &trait_def.default_methods {
                        if m.name == method {
                            return Some(m.clone());
                        }
                    }
                }
            }
        }
        None
    }

    /// Dispatch a method call: bind self, params, execute body.
    #[allow(clippy::too_many_arguments)]
    fn dispatch_method(
        &mut self,
        method_def: &FnDef,
        receiver: Value,
        type_name: &str,
        args: Vec<Value>,
        receiver_expr: &Expr,
        env: &Env,
        span: &Span,
    ) -> Result<Value, FerriError> {
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

        match result {
            Err(FerriError::Return(val)) => Ok(*val),
            other => other,
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
            // Built-in Option/Result: show without enum prefix
            let prefix = if enum_name == "Option" || enum_name == "Result" {
                String::new()
            } else {
                format!("{enum_name}::")
            };
            if data.is_empty() {
                format!("{prefix}{variant}")
            } else {
                let items: Vec<String> = data.iter().map(debug_format).collect();
                format!("{prefix}{variant}({})", items.join(", "))
            }
        }
        Value::HashMap(m) => {
            let mut sorted: Vec<_> = m.iter().collect();
            sorted.sort_by_key(|(k, _)| (*k).clone());
            let items: Vec<String> = sorted
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}: {}",
                        debug_format(&Value::String(k.to_string())),
                        debug_format(v)
                    )
                })
                .collect();
            format!("{{{}}}", items.join(", "))
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
}
