//! Compiler: walks the Oxy AST and emits stack-based bytecode for the VM.
//!
//! The compiler is single-pass. It resolves local variable names to stack
//! slot indices and emits [`OpCode`]s into a [`Chunk`]. Forward jumps
//! (for `if`, `while`, `loop`) are backpatched after the target is known.

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::errors::FerriError;
use crate::lexer::{FloatSuffix, IntegerSuffix};
use crate::types::{FloatWidth, IntegerWidth};
use crate::vm::{Chunk, OpCode};

/// Symbol table tracking local variables in the current scope.
#[derive(Clone)]
struct SymTable {
    /// Variable name → stack slot index.
    locals: HashMap<String, usize>,
    /// Mutable variables (declared with `let mut`).
    mutable: HashSet<String>,
    /// Next available slot index.
    next_slot: usize,
}

impl SymTable {
    fn new(start_slot: usize) -> Self {
        Self {
            locals: HashMap::new(),
            mutable: HashSet::new(),
            next_slot: start_slot,
        }
    }

    fn define(&mut self, name: &str) -> usize {
        let slot = self.next_slot;
        self.locals.insert(name.to_string(), slot);
        self.next_slot += 1;
        slot
    }

    fn define_mut(&mut self, name: &str) -> usize {
        self.mutable.insert(name.to_string());
        self.define(name)
    }

    /// Register a variable at a specific slot (for captured closure vars).
    fn define_at(&mut self, name: &str, slot: usize) {
        self.locals.insert(name.to_string(), slot);
        if slot >= self.next_slot {
            self.next_slot = slot + 1;
        }
    }

    fn is_mutable(&self, name: &str) -> bool {
        self.mutable.contains(name)
    }

    fn get(&self, name: &str) -> Option<usize> {
        self.locals.get(name).copied()
    }

    fn build_slot_names(&self) -> Vec<String> {
        let max_slot = self.locals.values().max().copied().unwrap_or(0);
        let size = (max_slot + 1).max(self.next_slot);
        let mut names = vec![String::new(); size];
        for (name, slot) in &self.locals {
            names[*slot] = name.clone();
        }
        names
    }
}

/// Tracks loop nesting for break/continue backpatching.
struct LoopContext {
    /// Label for this loop (for labeled break/continue resolution).
    label: Option<String>,
    /// Instruction index where `continue` should jump.
    continue_target: usize,
    /// Instruction indices of `Jump(0)` emitted for `break` statements.
    break_patches: Vec<usize>,
    /// Instruction indices of `Jump(0)` emitted for `continue` statements.
    continue_patches: Vec<usize>,
}

/// The Oxy bytecode compiler.
pub struct Compiler {
    /// The output code buffer.
    code: Vec<OpCode>,
    /// Current scope's symbol table.
    sym: SymTable,
    /// Function entry points: name → instruction index.
    functions: HashMap<String, usize>,
    /// Stack of enclosing loop contexts (for break/continue).
    loop_stack: Vec<LoopContext>,
    /// Closure metadata: (param_names, body_expr, captured_vars_with_slots_and_mutability).
    closure_meta: Vec<(Vec<String>, crate::ast::Expr, Vec<(String, usize, bool)>)>,
    /// Snapshot of main's local variable names (for Eval env reconstruction).
    main_local_names: Vec<String>,
    /// Registered struct definitions.
    struct_defs: HashMap<String, StructDef>,
    /// Registered enum definitions.
    enum_defs: HashMap<String, EnumDef>,
    /// Impl methods: type_name → method definitions.
    impl_methods: HashMap<String, Vec<FnDef>>,
    /// Compiled method entry points: (type_name, method_name) → instruction index.
    method_ips: HashMap<(String, String), usize>,
    /// Directory of the source file (for resolving file-based modules).
    source_dir: Option<std::path::PathBuf>,
    /// Use aliases: alias_name → qualified_name (e.g., "add" → "math::add").
    use_aliases: HashMap<String, String>,
    /// Const/static values: name → value (inlined at reference sites).
    const_values: HashMap<String, crate::types::Value>,
    /// Function metadata for named function references: name → (params, body, return_type).
    fn_meta: HashMap<
        String,
        (
            Vec<crate::ast::Param>,
            Box<crate::ast::Expr>,
            Option<crate::ast::TypeAnnotation>,
        ),
    >,
    /// Per-function local variable names: function entry IP → slot_names.
    fn_local_names: HashMap<usize, Vec<String>>,
    /// Mutable variables captured by closures (for targeted Cell wrapping).
    captured_mutable: HashSet<String>,
    /// If true, compilation fails when no `main` function exists.
    /// Set to false for test runners and library code.
    require_main: bool,
    /// Current impl type name (for resolving `Self` in method bodies).
    current_impl_type: Option<String>,
    /// Trait definitions: trait_name → methods (for default method inheritance).
    trait_defs: HashMap<String, Vec<FnDef>>,
    /// Type aliases: alias_name → actual_type_name (e.g., P → Point).
    type_aliases: HashMap<String, String>,
    /// Forward calls that need target patching: (bytecode_index, function_name).
    forward_calls: Vec<(usize, String)>,
    /// Module name stack for resolving `self`, `super`, `crate` in paths.
    module_stack: Vec<String>,
    /// Deferred use resolutions processed in post-pass: (qualified_path, reexport_name_or_empty).
    /// Empty reexport_name means glob import; non-empty means pub use re-export.
    deferred_globs: Vec<(String, String)>,
    /// Qualified names of public functions (for visibility-aware glob filters).
    pub_fns: HashSet<String>,
    /// Qualified names of public structs (for visibility-aware glob filters).
    pub_structs: HashSet<String>,
    /// Qualified names of public enums (for visibility-aware glob filters).
    pub_enums: HashSet<String>,
}

impl Compiler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a compiler that does not require a `main` function (for test runners, etc.).
    pub fn new_for_tests(source_path: Option<&str>) -> Self {
        let source_dir =
            source_path.and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_path_buf()));
        Self {
            source_dir,
            require_main: false,
            ..Self::default()
        }
    }

    /// Create a compiler that can resolve file-based modules relative to `source_path`.
    pub fn new_with_source_dir(source_path: Option<&str>) -> Self {
        let source_dir =
            source_path.and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_path_buf()));
        Self {
            source_dir,
            ..Self::default()
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self {
            code: Vec::new(),
            sym: SymTable::new(0),
            functions: HashMap::new(),
            loop_stack: Vec::new(),
            closure_meta: Vec::new(),
            main_local_names: Vec::new(),
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            impl_methods: HashMap::new(),
            method_ips: HashMap::new(),
            source_dir: None,
            use_aliases: HashMap::new(),
            const_values: HashMap::new(),
            fn_meta: HashMap::new(),
            fn_local_names: HashMap::new(),
            captured_mutable: HashSet::new(),
            require_main: true,
            current_impl_type: None,
            trait_defs: HashMap::new(),
            type_aliases: HashMap::new(),
            forward_calls: Vec::new(),
            module_stack: Vec::new(),
            deferred_globs: Vec::new(),
            pub_fns: HashSet::new(),
            pub_structs: HashSet::new(),
            pub_enums: HashSet::new(),
        }
    }
}

impl Compiler {
    /// Compile a full program. Returns a [`Chunk`] ready for the VM.
    pub fn compile(mut self, program: &Program) -> Result<Chunk, FerriError> {
        // Pre-scan: register all function names so forward references resolve.
        // Sentinel IP usize::MAX marks functions not yet compiled.
        // Also drill into inline modules so glob imports can find them.
        fn prescan_items(
            items: &[Item],
            functions: &mut HashMap<String, usize>,
            fn_meta: &mut HashMap<
                String,
                (
                    Vec<crate::ast::Param>,
                    Box<crate::ast::Expr>,
                    Option<crate::ast::TypeAnnotation>,
                ),
            >,
            struct_defs: &mut HashMap<String, StructDef>,
            enum_defs: &mut HashMap<String, EnumDef>,
            prefix: &str,
        ) {
            for item in items {
                match item {
                    Item::Function(f) => {
                        let name = if prefix.is_empty() {
                            f.name.clone()
                        } else {
                            format!("{}::{}", prefix, f.name)
                        };
                        functions.insert(name.clone(), usize::MAX);
                        let body_expr = f
                            .body
                            .stmts
                            .last()
                            .and_then(|stmt| match stmt {
                                crate::ast::Stmt::Expr { expr, .. } => Some(expr.clone()),
                                _ => None,
                            })
                            .unwrap_or(crate::ast::Expr::IntLiteral(
                                0,
                                crate::lexer::IntegerSuffix::None,
                                f.body.span,
                            ));
                        fn_meta.insert(
                            name,
                            (f.params.clone(), Box::new(body_expr), f.return_type.clone()),
                        );
                    }
                    Item::Struct(s) => {
                        let name = if prefix.is_empty() {
                            s.name.clone()
                        } else {
                            format!("{}::{}", prefix, s.name)
                        };
                        struct_defs.insert(name, s.clone());
                    }
                    Item::Enum(e) => {
                        let name = if prefix.is_empty() {
                            e.name.clone()
                        } else {
                            format!("{}::{}", prefix, e.name)
                        };
                        enum_defs.insert(name, e.clone());
                    }
                    Item::Module(m) => {
                        if let Some(body) = &m.body {
                            let nested = if prefix.is_empty() {
                                m.name.clone()
                            } else {
                                format!("{}::{}", prefix, m.name)
                            };
                            prescan_items(
                                body,
                                functions,
                                fn_meta,
                                struct_defs,
                                enum_defs,
                                &nested,
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
        prescan_items(
            &program.items,
            &mut self.functions,
            &mut self.fn_meta,
            &mut self.struct_defs,
            &mut self.enum_defs,
            "",
        );

        // Pre-resolve globs: process all use statements eagerly against pre-scanned data.
        // This ensures glob aliases exist before function bodies are compiled.
        self.preresolve_uses(&program.items)?;

        // Compile function bodies
        for item in &program.items {
            self.compile_item(item)?;
        }

        // Patch forward calls: replace sentinel targets with actual function IPs
        for (call_idx, fn_name) in &self.forward_calls {
            let actual_ip = self.functions.get(fn_name).copied().unwrap_or(0);
            self.code[*call_idx] = match &self.code[*call_idx] {
                OpCode::Call { arg_count, .. } => OpCode::Call {
                    target: actual_ip,
                    arg_count: *arg_count,
                },
                _ => continue,
            };
        }

        // Resolve deferred use declarations (pub use re-exports and late globs)
        self.resolve_deferred_uses();

        // Check if any function has #[test] attribute (test runner mode)
        let has_test_fns = program.items.iter().any(|item| {
            if let Item::Function(f) = item {
                f.attributes.iter().any(|a| a.name == "test")
            } else {
                false
            }
        });

        // Require a `main` function for executable programs
        let entry_point = match self.functions.get("main").copied() {
            Some(ip) => ip,
            None if !self.require_main || has_test_fns => 0,
            None => {
                return Err(FerriError::Runtime {
                    message: "no `main` function found".into(),
                    line: 0,
                    column: 0,
                });
            }
        };

        Ok(Chunk {
            code: self.code,
            local_count: 0,
            entry_point,
            functions: self.functions,
            closure_meta: self.closure_meta,
            local_names: self.main_local_names,
            fn_local_names: self.fn_local_names,
            struct_defs: self.struct_defs,
            enum_defs: self.enum_defs,
            impl_methods: self.impl_methods,
            method_ips: self.method_ips,
        })
    }

    fn emit(&mut self, op: OpCode) -> usize {
        let idx = self.code.len();
        self.code.push(op);
        idx
    }

    /// Report a compile error for features not yet supported in native bytecode.
    fn not_yet_supported(&self, feature: &str, span: crate::lexer::Span) -> FerriError {
        FerriError::Runtime {
            message: format!("{feature} not yet supported in native bytecode"),
            line: span.line,
            column: span.column,
        }
    }

    /// Patch a previously emitted instruction at `idx` with a new opcode.
    fn patch(&mut self, idx: usize, op: OpCode) {
        self.code[idx] = op;
    }

    fn compile_item(&mut self, item: &Item) -> Result<(), FerriError> {
        match item {
            Item::Function(f) => {
                self.pub_fns.insert(f.name.clone());
                self.compile_fn_item(f, None)?;
                Ok(())
            }
            Item::Struct(s) => {
                self.struct_defs.insert(s.name.clone(), s.clone());
                self.pub_structs.insert(s.name.clone());
                // Handle #[derive(Default)] by generating a default() constructor
                if s.attributes
                    .iter()
                    .any(|a| a.name == "derive" && a.args.iter().any(|arg| arg == "Default"))
                {
                    self.compile_derive_default(s)?;
                }
                Ok(())
            }
            Item::Enum(e) => {
                self.enum_defs.insert(e.name.clone(), e.clone());
                self.pub_enums.insert(e.name.clone());
                Ok(())
            }
            Item::Impl(i) => {
                // Register method definitions
                self.impl_methods
                    .entry(i.type_name.clone())
                    .or_default()
                    .extend(i.methods.clone());
                // Compile each method body
                for method in &i.methods {
                    let type_name = i.type_name.clone();
                    self.compile_fn_item(method, Some(&type_name))?;
                }
                Ok(())
            }
            Item::ImplTrait(i) => {
                // Collect method names explicitly defined in this impl
                let explicit: HashSet<String> = i.methods.iter().map(|m| m.name.clone()).collect();
                let mut all_methods = i.methods.clone();
                // Inherit default methods from trait that aren't explicitly overridden
                if let Some(trait_methods) = self.trait_defs.get(&i.trait_name) {
                    for tm in trait_methods {
                        if !explicit.contains(&tm.name) && !tm.body.stmts.is_empty() {
                            all_methods.push(tm.clone());
                        }
                    }
                }
                self.impl_methods
                    .entry(i.type_name.clone())
                    .or_default()
                    .extend(all_methods.clone());
                for method in &all_methods {
                    let type_name = i.type_name.clone();
                    self.compile_fn_item(method, Some(&type_name))?;
                }
                Ok(())
            }
            Item::Trait(t) => {
                // Store trait default methods for inheritance in impl blocks
                self.trait_defs
                    .insert(t.name.clone(), t.default_methods.clone());
                Ok(())
            }
            Item::Module(m) => {
                self.compile_module(m)?;
                Ok(())
            }
            Item::Use(u) => {
                self.compile_use(u)?;
                Ok(())
            }
            Item::TypeAlias { name, target, .. } => {
                self.type_aliases.insert(name.clone(), target.name.clone());
                Ok(())
            }
            Item::Const { name, value, .. } => {
                // Evaluate at compile time and store for inlining
                if let Some(val) = try_eval_const(&value) {
                    self.const_values.insert(name.clone(), val);
                }
                Ok(())
            }
        }
    }

    /// Generate a `fn default() -> Self` for #[derive(Default)] structs.
    fn compile_derive_default(&mut self, s: &StructDef) -> Result<(), FerriError> {
        let fields = match &s.kind {
            StructKind::Named(fields) => fields,
            _ => return Ok(()), // tuple/unit structs: skip for now
        };
        // Build default field expressions
        let default_span = s.span;
        let mut field_exprs: Vec<(String, Expr)> = Vec::new();
        for field in fields {
            let default_val = match field.type_ann.name.as_str() {
                "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "usize" => {
                    Expr::IntLiteral(0, IntegerSuffix::None, default_span)
                }
                "f64" | "f32" | "Float" => Expr::FloatLiteral(0.0, FloatSuffix::None, default_span),
                "String" | "str" => Expr::StringLiteral(String::new(), default_span),
                "bool" => Expr::BoolLiteral(false, default_span),
                "char" => Expr::CharLiteral('\0', default_span),
                _ => {
                    // For unknown types, use a zero-ish default
                    Expr::IntLiteral(0, IntegerSuffix::None, default_span)
                }
            };
            field_exprs.push((field.name.clone(), default_val));
        }
        // Build the struct init expression: StructName { field1: val1, ... }
        let body_expr = Expr::StructInit {
            name: s.name.clone(),
            fields: field_exprs,
            span: default_span,
        };
        // Build synthetic FnDef
        let fn_def = FnDef {
            name: "default".to_string(),
            is_async: false,
            generic_params: vec![],
            params: vec![],
            return_type: Some(TypeAnnotation {
                name: "Self".to_string(),
                span: default_span,
            }),
            body: Block {
                stmts: vec![Stmt::Expr {
                    expr: body_expr,
                    has_semicolon: false,
                }],
                span: default_span,
            },
            attributes: vec![],
            visibility: Visibility::Private,
            span: default_span,
        };
        self.compile_fn_item(&fn_def, Some(&s.name))
    }

    /// Compile a function or method body.
    fn compile_fn_item(&mut self, f: &FnDef, type_name: Option<&str>) -> Result<(), FerriError> {
        let ip = self.code.len();
        // Track current impl type so `Self` can be resolved inside method bodies
        let saved_impl_type = self.current_impl_type.clone();
        if let Some(tn) = type_name {
            self.current_impl_type = Some(tn.to_string());
        }
        // Register as a plain function and as a method if applicable
        self.functions.insert(f.name.clone(), ip);
        if let Some(tn) = type_name {
            // Also register qualified name so PathCall can resolve Type::method
            let qualified = format!("{}::{}", tn, f.name);
            self.functions.insert(qualified.clone(), ip);
            if f.visibility.is_pub() {
                self.pub_fns.insert(qualified);
            }
            self.method_ips.insert((tn.to_string(), f.name.clone()), ip);
        }
        // Store metadata for function-reference-as-value support
        let body_expr = f
            .body
            .stmts
            .last()
            .and_then(|stmt| match stmt {
                crate::ast::Stmt::Expr { expr, .. } => Some(expr.clone()),
                _ => None,
            })
            .unwrap_or(crate::ast::Expr::IntLiteral(
                0,
                IntegerSuffix::None,
                f.body.span,
            ));
        let meta = (f.params.clone(), Box::new(body_expr), f.return_type.clone());
        self.fn_meta.insert(f.name.clone(), meta.clone());
        if let Some(tn) = type_name {
            self.fn_meta.insert(format!("{}::{}", tn, f.name), meta);
        }

        let saved_sym = self.sym.clone();
        for param in &f.params {
            self.sym.define(&param.name);
        }

        // Pre-scan: find mutable variables captured by closures
        let param_names: Vec<String> = f.params.iter().map(|p| p.name.clone()).collect();
        self.captured_mutable = find_captured_mutable(&f.body, &param_names);

        self.compile_block(&f.body)?;
        // For methods with self parameter and no explicit tail expression,
        // implicitly return self so mutations propagate to caller.
        let has_tail_expr = f.body.stmts.last().map_or(false, |s| {
            matches!(
                s,
                crate::ast::Stmt::Expr {
                    has_semicolon: false,
                    ..
                }
            )
        });
        if !has_tail_expr && f.params.first().map(|p| p.name.as_str()) == Some("self") {
            self.emit(OpCode::LoadLocal(0));
        }
        self.emit(OpCode::Return);

        if f.name == "main" {
            self.main_local_names = self.sym.build_slot_names();
        }
        self.fn_local_names.insert(ip, self.sym.build_slot_names());

        self.sym = saved_sym;
        self.current_impl_type = saved_impl_type;
        Ok(())
    }

    /// Compile an inline or file-based module recursively.
    fn compile_module(&mut self, module: &ModuleDef) -> Result<(), FerriError> {
        let items = if let Some(body) = &module.body {
            body.clone()
        } else {
            let source = self.load_module_file(&module.name, module.span)?;
            let program = crate::parser::parse(&source)?;
            program.items
        };
        let prefix = module.name.clone();
        self.module_stack.push(module.name.clone());
        self.compile_module_items(&items, &prefix)?;
        self.module_stack.pop();
        Ok(())
    }

    /// Check whether a qualified path refers to a publicly visible item.
    /// Returns true if the item exists and is pub, or if the item is not tracked
    /// (allowing built-in/stdlib paths through).
    fn is_visible(&self, qualified: &str) -> bool {
        let in_fns = self.functions.contains_key(qualified);
        let in_structs = self.struct_defs.contains_key(qualified);
        let in_enums = self.enum_defs.contains_key(qualified);
        if !in_fns && !in_structs && !in_enums {
            return true; // not tracked — allow (builtin, forward ref)
        }
        self.pub_fns.contains(qualified)
            || self.pub_structs.contains(qualified)
            || self.pub_enums.contains(qualified)
    }

    /// Process a `use` declaration.
    fn compile_use(&mut self, use_def: &UseDef) -> Result<(), FerriError> {
        let resolved_path = Self::resolve_use_path(&use_def.path, &self.module_stack);
        let base_path = resolved_path.join("::");
        let module_prefix = self.module_stack.join("::");
        match &use_def.tree {
            UseTree::Simple(alias) => {
                let name = alias
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| use_def.path.last().cloned().unwrap_or_default());
                if self.is_visible(&base_path) {
                    self.use_aliases.insert(name.clone(), base_path.clone());
                    if use_def.visibility.is_pub() {
                        let reexport_name = if module_prefix.is_empty() {
                            name.clone()
                        } else {
                            format!("{}::{}", module_prefix, name)
                        };
                        self.use_aliases
                            .insert(reexport_name.clone(), base_path.clone());
                        self.deferred_globs.push((base_path, reexport_name));
                    }
                }
            }
            UseTree::Group(items) => {
                for (name, alias) in items {
                    let local_name = alias.as_ref().unwrap_or(name);
                    let qualified = format!("{}::{}", base_path, name);
                    if self.is_visible(&qualified) {
                        self.use_aliases
                            .insert(local_name.clone(), qualified.clone());
                        if use_def.visibility.is_pub() {
                            let reexport_name = if module_prefix.is_empty() {
                                local_name.clone()
                            } else {
                                format!("{}::{}", module_prefix, local_name)
                            };
                            self.use_aliases
                                .insert(reexport_name.clone(), qualified.clone());
                            self.deferred_globs.push((qualified, reexport_name));
                        }
                    }
                }
            }
            UseTree::Glob => {
                // Eager: alias all currently-known pub items
                let prefix = &base_path;
                for qualified_name in self.functions.keys() {
                    if let Some(stripped) = qualified_name.strip_prefix(&format!("{}::", prefix)) {
                        if !stripped.contains("::") && self.pub_fns.contains(qualified_name) {
                            self.use_aliases
                                .insert(stripped.to_string(), qualified_name.clone());
                            if use_def.visibility.is_pub() {
                                let reexport_name = if module_prefix.is_empty() {
                                    stripped.to_string()
                                } else {
                                    format!("{}::{}", module_prefix, stripped)
                                };
                                self.use_aliases
                                    .insert(reexport_name.clone(), qualified_name.clone());
                                self.deferred_globs
                                    .push((qualified_name.clone(), reexport_name));
                            }
                        }
                    }
                }
                for qualified_name in self.struct_defs.keys() {
                    if let Some(stripped) = qualified_name.strip_prefix(&format!("{}::", prefix)) {
                        if !stripped.contains("::") && self.pub_structs.contains(qualified_name) {
                            self.use_aliases
                                .insert(stripped.to_string(), qualified_name.clone());
                            if use_def.visibility.is_pub() {
                                let reexport_name = if module_prefix.is_empty() {
                                    stripped.to_string()
                                } else {
                                    format!("{}::{}", module_prefix, stripped)
                                };
                                self.use_aliases
                                    .insert(reexport_name.clone(), qualified_name.clone());
                                self.deferred_globs
                                    .push((qualified_name.clone(), reexport_name));
                            }
                        }
                    }
                }
                for qualified_name in self.enum_defs.keys() {
                    if let Some(stripped) = qualified_name.strip_prefix(&format!("{}::", prefix)) {
                        if !stripped.contains("::") && self.pub_enums.contains(qualified_name) {
                            self.use_aliases
                                .insert(stripped.to_string(), qualified_name.clone());
                            if use_def.visibility.is_pub() {
                                let reexport_name = if module_prefix.is_empty() {
                                    stripped.to_string()
                                } else {
                                    format!("{}::{}", module_prefix, stripped)
                                };
                                self.use_aliases
                                    .insert(reexport_name.clone(), qualified_name.clone());
                                self.deferred_globs
                                    .push((qualified_name.clone(), reexport_name));
                            }
                        }
                    }
                }
                // Also defer so items registered later are included (glob aliases)
                self.deferred_globs.push((base_path, String::new()));
            }
        }
        Ok(())
    }

    /// Resolve `self`, `super`, `crate` across ALL segments of a use path.
    /// Returns the resolved path with special keywords replaced relative to module_stack.
    fn resolve_use_path(path: &[String], module_stack: &[String]) -> Vec<String> {
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

    /// Pre-resolve all use statements against pre-scanned data, before function bodies
    /// are compiled. This ensures use aliases (especially globs) are available.
    fn preresolve_uses(&mut self, items: &[Item]) -> Result<(), FerriError> {
        let module_prefix = self.module_stack.join("::");
        for item in items {
            match item {
                Item::Use(u) => {
                    let resolved_path = Self::resolve_use_path(&u.path, &self.module_stack);
                    let base_path = resolved_path.join("::");
                    match &u.tree {
                        UseTree::Simple(alias) => {
                            let name = alias
                                .as_ref()
                                .cloned()
                                .unwrap_or_else(|| u.path.last().cloned().unwrap_or_default());
                            if self.is_visible(&base_path) {
                                self.use_aliases.insert(name.clone(), base_path.clone());
                                if u.visibility.is_pub() {
                                    let reexport = if module_prefix.is_empty() {
                                        name.clone()
                                    } else {
                                        format!("{}::{}", module_prefix, name)
                                    };
                                    self.use_aliases.insert(reexport.clone(), base_path.clone());
                                    self.deferred_globs.push((base_path, reexport));
                                }
                            }
                        }
                        UseTree::Group(items) => {
                            for (name, alias) in items {
                                let local_name = alias.as_ref().unwrap_or(name);
                                let qualified = format!("{}::{}", base_path, name);
                                if self.is_visible(&qualified) {
                                    self.use_aliases
                                        .insert(local_name.clone(), qualified.clone());
                                    if u.visibility.is_pub() {
                                        let reexport = if module_prefix.is_empty() {
                                            local_name.clone()
                                        } else {
                                            format!("{}::{}", module_prefix, local_name)
                                        };
                                        self.use_aliases
                                            .insert(reexport.clone(), qualified.clone());
                                        self.deferred_globs.push((qualified, reexport));
                                    }
                                }
                            }
                        }
                        UseTree::Glob => {
                            let prefix = &base_path;
                            for qualified_name in self.functions.keys() {
                                if let Some(stripped) =
                                    qualified_name.strip_prefix(&format!("{}::", prefix))
                                {
                                    if !stripped.contains("::") {
                                        self.use_aliases
                                            .insert(stripped.to_string(), qualified_name.clone());
                                        if u.visibility.is_pub() {
                                            let reexport_name = if module_prefix.is_empty() {
                                                stripped.to_string()
                                            } else {
                                                format!("{}::{}", module_prefix, stripped)
                                            };
                                            self.use_aliases.insert(
                                                reexport_name.clone(),
                                                qualified_name.clone(),
                                            );
                                            self.deferred_globs
                                                .push((qualified_name.clone(), reexport_name));
                                        }
                                    }
                                }
                            }
                            for qualified_name in self.struct_defs.keys() {
                                if let Some(stripped) =
                                    qualified_name.strip_prefix(&format!("{}::", prefix))
                                {
                                    if !stripped.contains("::") {
                                        self.use_aliases
                                            .insert(stripped.to_string(), qualified_name.clone());
                                        if u.visibility.is_pub() {
                                            let reexport_name = if module_prefix.is_empty() {
                                                stripped.to_string()
                                            } else {
                                                format!("{}::{}", module_prefix, stripped)
                                            };
                                            self.use_aliases.insert(
                                                reexport_name.clone(),
                                                qualified_name.clone(),
                                            );
                                            self.deferred_globs
                                                .push((qualified_name.clone(), reexport_name));
                                        }
                                    }
                                }
                            }
                            for qualified_name in self.enum_defs.keys() {
                                if let Some(stripped) =
                                    qualified_name.strip_prefix(&format!("{}::", prefix))
                                {
                                    if !stripped.contains("::") {
                                        self.use_aliases
                                            .insert(stripped.to_string(), qualified_name.clone());
                                        if u.visibility.is_pub() {
                                            let reexport_name = if module_prefix.is_empty() {
                                                stripped.to_string()
                                            } else {
                                                format!("{}::{}", module_prefix, stripped)
                                            };
                                            self.use_aliases.insert(
                                                reexport_name.clone(),
                                                qualified_name.clone(),
                                            );
                                            self.deferred_globs
                                                .push((qualified_name.clone(), reexport_name));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Item::Module(m) => {
                    if let Some(body) = &m.body {
                        self.module_stack.push(m.name.clone());
                        self.preresolve_uses(body)?;
                        self.module_stack.pop();
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Resolve deferred use declarations: glob imports and pub use re-exports.
    /// Called after all items have been compiled, so forward references are resolved.
    fn resolve_deferred_uses(&mut self) {
        // Take deferred_globs to avoid borrow issues, process, then discard
        let deferred: Vec<(String, String)> = std::mem::take(&mut self.deferred_globs);
        for (base_path, reexport_name) in &deferred {
            if reexport_name.is_empty() {
                // Glob import: alias all pub items under base_path
                let prefix = base_path;
                for qualified_name in self.functions.keys() {
                    if let Some(stripped) = qualified_name.strip_prefix(&format!("{}::", prefix)) {
                        if !stripped.contains("::") && self.pub_fns.contains(qualified_name) {
                            self.use_aliases
                                .insert(stripped.to_string(), qualified_name.clone());
                        }
                    }
                }
                for qualified_name in self.struct_defs.keys() {
                    if let Some(stripped) = qualified_name.strip_prefix(&format!("{}::", prefix)) {
                        if !stripped.contains("::") && self.pub_structs.contains(qualified_name) {
                            self.use_aliases
                                .insert(stripped.to_string(), qualified_name.clone());
                        }
                    }
                }
                for qualified_name in self.enum_defs.keys() {
                    if let Some(stripped) = qualified_name.strip_prefix(&format!("{}::", prefix)) {
                        if !stripped.contains("::") && self.pub_enums.contains(qualified_name) {
                            self.use_aliases
                                .insert(stripped.to_string(), qualified_name.clone());
                        }
                    }
                }
            } else {
                // pub use re-export: register qualified item under alias name
                if let Some(&ip) = self.functions.get(base_path) {
                    self.functions.insert(reexport_name.clone(), ip);
                    self.pub_fns.insert(reexport_name.clone());
                }
                if let Some(def) = self.struct_defs.get(base_path).cloned() {
                    self.struct_defs.insert(reexport_name.clone(), def);
                    self.pub_structs.insert(reexport_name.clone());
                }
                if let Some(def) = self.enum_defs.get(base_path).cloned() {
                    self.enum_defs.insert(reexport_name.clone(), def);
                    self.pub_enums.insert(reexport_name.clone());
                }
            }
        }
    }

    /// Compile items with a module prefix (qualified names).
    fn compile_module_items(&mut self, items: &[Item], prefix: &str) -> Result<(), FerriError> {
        for item in items {
            match item {
                Item::Function(f) => {
                    let qualified = format!("{}::{}", prefix, f.name);
                    let ip = self.code.len();
                    self.functions.insert(qualified.clone(), ip);
                    if f.visibility.is_pub() {
                        self.pub_fns.insert(qualified.clone());
                    }
                    // Store metadata for function-reference-as-value
                    let body_expr = f
                        .body
                        .stmts
                        .last()
                        .and_then(|stmt| match stmt {
                            crate::ast::Stmt::Expr { expr, .. } => Some(expr.clone()),
                            _ => None,
                        })
                        .unwrap_or(crate::ast::Expr::IntLiteral(
                            0,
                            IntegerSuffix::None,
                            f.body.span,
                        ));
                    self.fn_meta.insert(
                        qualified,
                        (f.params.clone(), Box::new(body_expr), f.return_type.clone()),
                    );
                    let saved_sym = self.sym.clone();
                    for param in &f.params {
                        self.sym.define(&param.name);
                    }
                    self.compile_block(&f.body)?;
                    self.emit(OpCode::Return);
                    self.fn_local_names.insert(ip, self.sym.build_slot_names());
                    self.sym = saved_sym;
                }
                Item::Struct(s) => {
                    let qualified = format!("{}::{}", prefix, s.name);
                    self.struct_defs.insert(qualified.clone(), s.clone());
                    if s.visibility.is_pub() {
                        self.pub_structs.insert(qualified);
                    }
                }
                Item::Enum(e) => {
                    let qualified = format!("{}::{}", prefix, e.name);
                    self.enum_defs.insert(qualified.clone(), e.clone());
                    if e.visibility.is_pub() {
                        self.pub_enums.insert(qualified);
                    }
                }
                Item::Impl(i) => {
                    let qualified_type = format!("{}::{}", prefix, i.type_name);
                    self.impl_methods
                        .entry(qualified_type.clone())
                        .or_default()
                        .extend(i.methods.clone());
                    for method in &i.methods {
                        // Register as qualified_type::method (e.g. geometry::Point::new)
                        let mname = format!("{}::{}", qualified_type, method.name);
                        let ip = self.code.len();
                        self.functions.insert(mname.clone(), ip);
                        if method.visibility.is_pub() {
                            self.pub_fns.insert(mname.clone());
                        }
                        // Store fn_meta for arg count checking
                        let body_expr = method
                            .body
                            .stmts
                            .last()
                            .and_then(|stmt| match stmt {
                                crate::ast::Stmt::Expr { expr, .. } => Some(expr.clone()),
                                _ => None,
                            })
                            .unwrap_or(crate::ast::Expr::IntLiteral(
                                0,
                                IntegerSuffix::None,
                                method.body.span,
                            ));
                        self.fn_meta.insert(
                            mname.clone(),
                            (
                                method.params.clone(),
                                Box::new(body_expr),
                                method.return_type.clone(),
                            ),
                        );
                        // Register under both qualified and unqualified type names
                        self.method_ips
                            .insert((qualified_type.clone(), method.name.clone()), ip);
                        self.method_ips
                            .insert((i.type_name.clone(), method.name.clone()), ip);
                        let saved_sym = self.sym.clone();
                        for param in &method.params {
                            self.sym.define(&param.name);
                        }
                        self.compile_block(&method.body)?;
                        self.emit(OpCode::Return);
                        self.fn_local_names.insert(ip, self.sym.build_slot_names());
                        self.sym = saved_sym;
                    }
                }
                Item::Module(m) => {
                    let nested_prefix = format!("{}::{}", prefix, m.name);
                    self.module_stack.push(m.name.clone());
                    if let Some(body) = &m.body {
                        self.compile_module_items(body, &nested_prefix)?;
                    } else {
                        let source = self.load_module_file(&m.name, m.span)?;
                        let program = crate::parser::parse(&source)?;
                        self.compile_module_items(&program.items, &nested_prefix)?;
                    }
                    self.module_stack.pop();
                }
                Item::Use(u) => {
                    self.compile_use(u)?;
                }
                Item::Trait(t) => {
                    let qualified = format!("{}::{}", prefix, t.name);
                    self.trait_defs.insert(qualified, t.default_methods.clone());
                }
                Item::ImplTrait(i) => {
                    let qualified_type = format!("{}::{}", prefix, i.type_name);
                    let explicit: HashSet<String> =
                        i.methods.iter().map(|m| m.name.clone()).collect();
                    let mut all_methods = i.methods.clone();
                    // Try both unqualified and qualified trait name
                    let trait_key = self
                        .trait_defs
                        .contains_key(&i.trait_name)
                        .then_some(i.trait_name.clone())
                        .or_else(|| {
                            let q = format!("{}::{}", prefix, i.trait_name);
                            self.trait_defs.contains_key(&q).then_some(q)
                        });
                    if let Some(tk) = &trait_key {
                        if let Some(trait_methods) = self.trait_defs.get(tk) {
                            for tm in trait_methods {
                                if !explicit.contains(&tm.name) && !tm.body.stmts.is_empty() {
                                    all_methods.push(tm.clone());
                                }
                            }
                        }
                    }
                    self.impl_methods
                        .entry(qualified_type.clone())
                        .or_default()
                        .extend(all_methods.clone());
                    for method in &all_methods {
                        let mname = format!("{}::{}", qualified_type, method.name);
                        let ip = self.code.len();
                        self.functions.insert(mname.clone(), ip);
                        if method.visibility.is_pub() {
                            self.pub_fns.insert(mname.clone());
                        }
                        let body_expr = method
                            .body
                            .stmts
                            .last()
                            .and_then(|stmt| match stmt {
                                crate::ast::Stmt::Expr { expr, .. } => Some(expr.clone()),
                                _ => None,
                            })
                            .unwrap_or(crate::ast::Expr::IntLiteral(
                                0,
                                IntegerSuffix::None,
                                method.body.span,
                            ));
                        self.fn_meta.insert(
                            mname.clone(),
                            (
                                method.params.clone(),
                                Box::new(body_expr),
                                method.return_type.clone(),
                            ),
                        );
                        self.method_ips
                            .insert((qualified_type.clone(), method.name.clone()), ip);
                        self.method_ips
                            .insert((i.type_name.clone(), method.name.clone()), ip);
                        let saved_sym = self.sym.clone();
                        for param in &method.params {
                            self.sym.define(&param.name);
                        }
                        self.compile_block(&method.body)?;
                        self.emit(OpCode::Return);
                        self.fn_local_names.insert(ip, self.sym.build_slot_names());
                        self.sym = saved_sym;
                    }
                }
                Item::TypeAlias { name, target, .. } => {
                    let qualified = format!("{}::{}", prefix, name);
                    self.type_aliases.insert(qualified, target.name.clone());
                }
                Item::Const { name, value, .. } => {
                    if let Some(val) = try_eval_const(value) {
                        self.const_values.insert(name.clone(), val);
                    }
                }
            }
        }
        Ok(())
    }

    /// Load a file-based module's source code.
    fn load_module_file(&self, name: &str, span: crate::lexer::Span) -> Result<String, FerriError> {
        let base_ref = self
            .source_dir
            .as_ref()
            .map(|b| b.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());

        let path1 = format!("{base_ref}/{name}.ox");
        let path2 = format!("{base_ref}/{name}/mod.ox");

        if let Ok(source) = std::fs::read_to_string(&path1) {
            return Ok(source);
        }
        if let Ok(source) = std::fs::read_to_string(&path2) {
            return Ok(source);
        }
        if let Some((source, _pkg_name)) = crate::package::find_module_in_packages(name) {
            return Ok(source);
        }
        Err(FerriError::Runtime {
            message: format!("could not find module `{name}`: tried '{path1}' and '{path2}'"),
            line: span.line,
            column: span.column,
        })
    }

    /// Compile a pattern check. Leaves scrutinee on stack with Bool+data on top.
    /// Expects: [scrutinee]
    /// Leaves:  [scrutinee, true, data...] or [scrutinee, false]
    fn compile_pattern(
        &mut self,
        pattern: &Pattern,
        _next_arm_labels: &mut Vec<usize>,
        _is_last: bool,
    ) -> Result<(), FerriError> {
        match pattern {
            Pattern::Wildcard(_) => {
                self.emit(OpCode::ConstBool(true));
                Ok(())
            }
            Pattern::Ident(name, _) => {
                // Always matches — the binding happens in bind_pattern_data
                self.emit(OpCode::ConstBool(true));
                // Push a copy of the scrutinee as the "data" to bind
                self.emit(OpCode::Dup);
                // Define a slot for this binding (will be bound in bind_pattern_data)
                self.sym.define(name);
                Ok(())
            }
            Pattern::Literal(expr) => {
                self.emit(OpCode::Dup);
                self.compile_expr(expr)?;
                self.emit(OpCode::Eq);
                Ok(())
            }
            Pattern::EnumVariant {
                enum_name,
                variant,
                fields,
                ..
            } => {
                // Resolve enum_name via type aliases and use aliases
                let resolved_enum = self
                    .type_aliases
                    .get(enum_name)
                    .cloned()
                    .or_else(|| self.use_aliases.get(enum_name).cloned())
                    .unwrap_or_else(|| enum_name.clone());
                self.emit(OpCode::EnumVariantEqual {
                    enum_name: resolved_enum,
                    variant: variant.clone(),
                });
                // Pre-define slots for field pattern bindings
                self.define_pattern_slots(fields);
                Ok(())
            }
            Pattern::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                // Stack: [scrutinee] → After: [scrutinee, Bool]
                match (start, end) {
                    (Some(s), None) => {
                        self.emit(OpCode::Dup);
                        self.emit(OpCode::ConstInt(*s, IntegerWidth::I64));
                        self.emit(OpCode::Ge);
                    }
                    (None, Some(e)) => {
                        self.emit(OpCode::Dup);
                        self.emit(OpCode::ConstInt(*e, IntegerWidth::I64));
                        if *inclusive {
                            self.emit(OpCode::Le);
                        } else {
                            self.emit(OpCode::Lt);
                        }
                    }
                    (Some(s), Some(e)) => {
                        // Compute both bounds. Use the lowest possible slot (0) as
                        // temp storage. The first local variable is always at slot 0,
                        // so the stack always has at least 1 element. Storing at
                        // slot 0 overwrites a local but that's OK — it's the first
                        // local which we don't need after the pattern check.
                        self.emit(OpCode::Dup); // [s, s_copy]
                        self.emit(OpCode::ConstInt(*s, IntegerWidth::I64));
                        self.emit(OpCode::Ge); // [s, lower]
                        self.emit(OpCode::StoreLocal(0)); // store at slot 0 (always exists)
                        self.emit(OpCode::Dup); // [s, s]
                        self.emit(OpCode::ConstInt(*e, IntegerWidth::I64));
                        if *inclusive {
                            self.emit(OpCode::Le);
                        } else {
                            self.emit(OpCode::Lt);
                        } // [s, upper]
                        self.emit(OpCode::LoadLocal(0)); // [s, upper, lower]
                        self.emit(OpCode::And); // [s, result]
                    }
                    (None, None) => {
                        self.emit(OpCode::ConstBool(true));
                    }
                }
                Ok(())
            }
            _ => {
                // For Struct, Tuple, Or, Slice, Rest — fall back to const false
                // (will be handled properly in subsequent iterations)
                self.emit(OpCode::ConstBool(false));
                Ok(())
            }
        }
    }

    /// Pre-define slots for pattern variables (called during pattern compilation).
    fn define_pattern_slots(&mut self, patterns: &[Pattern]) {
        for p in patterns {
            match p {
                Pattern::Ident(name, _) => {
                    self.sym.define(name);
                }
                Pattern::EnumVariant { fields, .. } | Pattern::Tuple(fields, _) => {
                    self.define_pattern_slots(fields);
                }
                _ => {}
            }
        }
    }

    /// Bind pattern variables after a successful match.
    /// Expects: [scrutinee, data...] on stack (scrutinee and match bool already popped)
    /// Actually: called after Pop(scrutinee), so stack has [data...]
    fn bind_pattern_data(&mut self, pattern: &Pattern) -> Result<(), FerriError> {
        match pattern {
            Pattern::Wildcard(_) => Ok(()),
            Pattern::Ident(name, _) => {
                // Pop the data value and bind it
                if let Some(slot) = self.sym.get(name) {
                    self.emit(OpCode::BindIdent(slot));
                }
                Ok(())
            }
            Pattern::EnumVariant { fields, .. } => {
                // For each field pattern, bind the corresponding data value
                for field_pat in fields {
                    self.bind_pattern_data(field_pat)?;
                }
                Ok(())
            }
            Pattern::Literal(_) => Ok(()), // no binding needed
            Pattern::Tuple(patterns, _) => {
                // Stack: [Tuple_value]. Extract elements by index and bind sub-patterns.
                let temp = self.sym.define("__tuple_tmp");
                self.emit(OpCode::StoreLocal(temp));
                for (i, pat) in patterns.iter().enumerate() {
                    self.emit(OpCode::LoadLocal(temp));
                    self.emit(OpCode::ConstInt(i as i64, IntegerWidth::I64));
                    self.emit(OpCode::VecIndex);
                    self.bind_pattern_data(pat)?;
                }
                Ok(())
            }
            _ => Ok(()), // other patterns defer to Eval or not yet supported
        }
    }

    /// Native destructuring for tuple and slice patterns.
    fn compile_destructure(
        &mut self,
        value: &Expr,
        patterns: &[Pattern],
        span: crate::lexer::Span,
    ) -> Result<(), FerriError> {
        self.compile_expr(value)?;
        let temp_slot = self.sym.define("__destructure_tmp");
        self.emit(OpCode::StoreLocal(temp_slot));
        for (i, pat) in patterns.iter().enumerate() {
            match pat {
                Pattern::Ident(name, _) => {
                    self.emit(OpCode::LoadLocal(temp_slot));
                    self.emit(OpCode::ConstInt(i as i64, IntegerWidth::I64));
                    self.emit(OpCode::VecIndex);
                    let slot = self.sym.define(name);
                    self.emit(OpCode::BindIdent(slot));
                }
                Pattern::Wildcard(_) | Pattern::Rest(_) => {
                    // Skip — no binding needed
                }
                _ => {
                    // Nested pattern — not supported yet
                    return Err(FerriError::Runtime {
                        message: "complex destructure patterns not yet supported natively".into(),
                        line: span.line,
                        column: span.column,
                    });
                }
            }
        }
        Ok(())
    }

    /// Complex let-patterns not yet supported in native bytecode.
    fn compile_letpattern_unsupported(
        &mut self,
        _pattern: &Box<Pattern>,
        _value: &Expr,
        span: crate::lexer::Span,
        _mutable: bool,
    ) -> Result<(), FerriError> {
        Err(self.not_yet_supported("Complex destructure patterns", span))
    }

    fn compile_block(&mut self, block: &Block) -> Result<(), FerriError> {
        for (i, stmt) in block.stmts.iter().enumerate() {
            let is_last = i == block.stmts.len() - 1;
            self.compile_stmt(stmt, is_last)?;
        }
        Ok(())
    }

    /// Walk up the loop_stack to find the loop matching `label`.
    /// - `None` (unlabeled) → innermost loop
    /// - `Some(name)` → first loop with that label (searching from innermost outward)
    fn resolve_label(&mut self, label: &Option<String>) -> Option<&mut LoopContext> {
        match label {
            None => self.loop_stack.last_mut(),
            Some(name) => self
                .loop_stack
                .iter_mut()
                .rev()
                .find(|ctx| ctx.label.as_deref() == Some(name)),
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt, is_last: bool) -> Result<(), FerriError> {
        match stmt {
            Stmt::Let {
                name,
                mutable,
                type_ann,
                value,
                ..
            } => {
                // `let _ = expr;` — evaluate the expression and discard the result
                if name == "_" {
                    if let Some(expr) = value {
                        self.compile_expr(expr)?;
                        self.emit(OpCode::Pop);
                    }
                    return Ok(());
                }
                if let Some(expr) = value {
                    // Check literal out-of-range before compilation
                    if let Some(ann) = type_ann {
                        check_literal_fits_type(expr, &ann.name, ann.span)?;
                    }
                    self.compile_expr(expr)?;
                    // Narrow to the annotated type if it specifies a width
                    if let Some(ann) = type_ann {
                        emit_narrowing_cast(self, &ann.name);
                    }
                } else {
                    self.emit(OpCode::ConstUnit);
                }
                let slot = if *mutable {
                    self.sym.define_mut(name)
                } else {
                    self.sym.define(name)
                };
                self.emit(OpCode::StoreLocal(slot));
                if *mutable && self.captured_mutable.contains(name) {
                    self.emit(OpCode::MakeCell(slot));
                }
                Ok(())
            }

            Stmt::Expr {
                expr,
                has_semicolon,
            } => {
                self.compile_expr(expr)?;
                if *has_semicolon {
                    // Expression value not used, pop it
                    self.emit(OpCode::Pop);
                } else if is_last {
                    // Tail expression: leave on stack as return value
                    // Remove the implicit Return's ConstUnit if present
                }
                Ok(())
            }

            Stmt::Return { value, .. } => {
                if let Some(expr) = value {
                    self.compile_expr(expr)?;
                } else {
                    self.emit(OpCode::ConstUnit);
                }
                self.emit(OpCode::Return);
                Ok(())
            }

            Stmt::While {
                label,
                condition,
                body,
                ..
            } => {
                let loop_start = self.code.len();
                self.loop_stack.push(LoopContext {
                    label: label.clone(),
                    continue_target: loop_start,
                    break_patches: vec![],
                    continue_patches: vec![],
                });
                self.compile_expr(condition)?;
                let jump_out = self.emit(OpCode::JumpIfFalse(0));
                self.compile_block(body)?;
                self.emit(OpCode::Jump(loop_start));
                let loop_end = self.code.len();
                self.patch(jump_out, OpCode::JumpIfFalse(loop_end));
                let ctx = self.loop_stack.pop().unwrap();
                for idx in &ctx.break_patches {
                    self.patch(*idx, OpCode::Jump(loop_end));
                }
                for idx in &ctx.continue_patches {
                    self.patch(*idx, OpCode::Jump(loop_start));
                }
                Ok(())
            }

            Stmt::Loop { label, body, .. } => {
                let loop_start = self.code.len();
                self.loop_stack.push(LoopContext {
                    label: label.clone(),
                    continue_target: loop_start,
                    break_patches: vec![],
                    continue_patches: vec![],
                });
                self.compile_block(body)?;
                self.emit(OpCode::Jump(loop_start));
                let loop_end = self.code.len();
                let ctx = self.loop_stack.pop().unwrap();
                for idx in &ctx.break_patches {
                    self.patch(*idx, OpCode::Jump(loop_end));
                }
                for idx in &ctx.continue_patches {
                    self.patch(*idx, OpCode::Jump(ctx.continue_target));
                }
                Ok(())
            }

            Stmt::For {
                label,
                name,
                iterable,
                body,
                ..
            } => {
                let saved_sym = self.sym.clone();
                let vec_slot = self.sym.define("__for_vec");
                let idx_slot = self.sym.define("__for_idx");
                let var_slot = self.sym.define(name);

                // Preamble: evaluate iterable, materialize as Vec
                self.compile_expr(iterable)?;
                self.emit(OpCode::MakeIter);
                self.emit(OpCode::StoreLocal(vec_slot));
                self.emit(OpCode::ConstInt(0, IntegerWidth::I64));
                self.emit(OpCode::StoreLocal(idx_slot));

                // Jump to condition check on first iteration
                let jump_to_check = self.emit(OpCode::Jump(0));

                // --- Body: load current element ---
                let body_start = self.code.len();
                self.emit(OpCode::LoadLocal(vec_slot));
                self.emit(OpCode::LoadLocal(idx_slot));
                self.emit(OpCode::VecIndex);
                self.emit(OpCode::StoreLocal(var_slot));

                // Push loop context (continue_target is placeholder, set after body)
                self.loop_stack.push(LoopContext {
                    label: label.clone(),
                    continue_target: 0,
                    break_patches: vec![],
                    continue_patches: vec![],
                });

                self.compile_block(body)?;

                let ctx = self.loop_stack.pop().unwrap();

                // --- Advance: increment index (continue jumps here) ---
                let advance_start = self.code.len();
                self.emit(OpCode::LoadLocal(idx_slot));
                self.emit(OpCode::ConstInt(1, IntegerWidth::I64));
                self.emit(OpCode::Add);
                self.emit(OpCode::StoreLocal(idx_slot));

                // --- Condition check ---
                let check_start = self.code.len();
                self.emit(OpCode::LoadLocal(idx_slot));
                self.emit(OpCode::LoadLocal(vec_slot));
                self.emit(OpCode::IterLen);
                self.emit(OpCode::Lt);
                self.emit(OpCode::JumpIfTrue(body_start));

                // --- Exit ---
                let loop_end = self.code.len();
                self.patch(jump_to_check, OpCode::Jump(check_start));
                for idx in &ctx.break_patches {
                    self.patch(*idx, OpCode::Jump(loop_end));
                }
                for idx in &ctx.continue_patches {
                    self.patch(*idx, OpCode::Jump(advance_start));
                }

                self.sym = saved_sym;
                Ok(())
            }

            Stmt::Break { label, value, span } => {
                if self.loop_stack.is_empty() {
                    return Err(FerriError::Runtime {
                        message: "break outside of loop".into(),
                        line: span.line,
                        column: span.column,
                    });
                }
                if let Some(expr) = value {
                    self.compile_expr(expr)?;
                }
                let patch = self.emit(OpCode::Jump(0));
                // Walk up loop_stack to find matching label
                let target = self.resolve_label(label);
                match target {
                    Some(ctx) => ctx.break_patches.push(patch),
                    None => {
                        return Err(FerriError::Runtime {
                            message: format!(
                                "use of undeclared label `{}`",
                                label.as_deref().unwrap_or("")
                            ),
                            line: span.line,
                            column: span.column,
                        });
                    }
                }
                Ok(())
            }

            Stmt::Continue { label, span } => {
                if self.loop_stack.is_empty() {
                    return Err(FerriError::Runtime {
                        message: "continue outside of loop".into(),
                        line: span.line,
                        column: span.column,
                    });
                }
                let patch = self.emit(OpCode::Jump(0));
                let target = self.resolve_label(label);
                match target {
                    Some(ctx) => ctx.continue_patches.push(patch),
                    None => {
                        return Err(FerriError::Runtime {
                            message: format!(
                                "use of undeclared label `{}`",
                                label.as_deref().unwrap_or("")
                            ),
                            line: span.line,
                            column: span.column,
                        });
                    }
                }
                Ok(())
            }

            Stmt::ForDestructure {
                label,
                names,
                iterable,
                body,
                ..
            } => {
                let saved_sym = self.sym.clone();
                let vec_slot = self.sym.define("__for_vec");
                let idx_slot = self.sym.define("__for_idx");
                let tmp_slot = self.sym.define("__for_tmp");
                let name_slots: Vec<usize> = names.iter().map(|n| self.sym.define(n)).collect();

                // Preamble
                self.compile_expr(iterable)?;
                self.emit(OpCode::MakeIter);
                self.emit(OpCode::StoreLocal(vec_slot));
                self.emit(OpCode::ConstInt(0, IntegerWidth::I64));
                self.emit(OpCode::StoreLocal(idx_slot));
                let jump_to_check = self.emit(OpCode::Jump(0));

                // Body: load current tuple, destructure by index
                let body_start = self.code.len();
                self.emit(OpCode::LoadLocal(vec_slot));
                self.emit(OpCode::LoadLocal(idx_slot));
                self.emit(OpCode::VecIndex);
                self.emit(OpCode::StoreLocal(tmp_slot));
                for (i, &slot) in name_slots.iter().enumerate() {
                    self.emit(OpCode::LoadLocal(tmp_slot));
                    self.emit(OpCode::ConstInt(i as i64, IntegerWidth::I64));
                    self.emit(OpCode::VecIndex);
                    self.emit(OpCode::StoreLocal(slot));
                }

                self.loop_stack.push(LoopContext {
                    label: label.clone(),
                    continue_target: 0,
                    break_patches: vec![],
                    continue_patches: vec![],
                });
                self.compile_block(body)?;
                let ctx = self.loop_stack.pop().unwrap();

                let advance_start = self.code.len();
                self.emit(OpCode::LoadLocal(idx_slot));
                self.emit(OpCode::ConstInt(1, IntegerWidth::I64));
                self.emit(OpCode::Add);
                self.emit(OpCode::StoreLocal(idx_slot));

                let check_start = self.code.len();
                self.emit(OpCode::LoadLocal(idx_slot));
                self.emit(OpCode::LoadLocal(vec_slot));
                self.emit(OpCode::IterLen);
                self.emit(OpCode::Lt);
                self.emit(OpCode::JumpIfTrue(body_start));

                let loop_end = self.code.len();
                self.patch(jump_to_check, OpCode::Jump(check_start));
                for idx in &ctx.break_patches {
                    self.patch(*idx, OpCode::Jump(loop_end));
                }
                for idx in &ctx.continue_patches {
                    self.patch(*idx, OpCode::Jump(advance_start));
                }

                self.sym = saved_sym;
                Ok(())
            }

            // Statements without native bytecode — fall back to interpreter
            Stmt::WhileLet {
                pattern,
                expr,
                body,
                label,
                span: _,
            } => {
                let loop_start = self.code.len();
                // Evaluate expression, store in temp
                self.compile_expr(expr)?;
                let scrut_slot = self.sym.define("__whilelet_scrutinee");
                let current_slot = self.sym.next_slot;
                self.emit(OpCode::StoreLocal(scrut_slot));
                // Pattern check
                self.emit(OpCode::LoadLocal(scrut_slot));
                let consumes = matches!(pattern.as_ref(), Pattern::EnumVariant { .. });
                self.compile_pattern(pattern, &mut vec![], true)?;
                let jump_to_end = self.emit(OpCode::JumpIfFalse(0));
                // Matched: clean up, bind, compile body
                if !consumes {
                    self.emit(OpCode::Pop);
                }
                self.bind_pattern_data(pattern)?;
                // Loop context for break/continue
                self.loop_stack.push(LoopContext {
                    label: label.clone(),
                    continue_target: loop_start,
                    break_patches: vec![],
                    continue_patches: vec![],
                });
                self.compile_block(body)?;
                let ctx = self.loop_stack.pop().unwrap();
                // Jump back to loop start
                self.emit(OpCode::Jump(loop_start));
                // End: patch exit jump
                let loop_end = self.code.len();
                self.patch(jump_to_end, OpCode::JumpIfFalse(loop_end));
                // Patch break/continue
                for idx in &ctx.break_patches {
                    self.patch(*idx, OpCode::Jump(loop_end));
                }
                for idx in &ctx.continue_patches {
                    self.patch(*idx, OpCode::Jump(loop_start));
                }
                self.sym.next_slot = current_slot;
                Ok(())
            }
            Stmt::LetPattern {
                pattern,
                value,
                span,
                mutable,
            } => {
                // Try native tuple destructuring: let (a, b, ...) = expr;
                if let Pattern::Tuple(patterns, _) = pattern.as_ref() {
                    return self.compile_destructure(value, patterns, *span);
                }
                // Try native slice destructuring: let [a, b, ...] = expr;
                if let Pattern::Slice(patterns, _) = pattern.as_ref() {
                    return self.compile_destructure(value, patterns, *span);
                }
                // For other patterns, not yet supported natively
                self.compile_letpattern_unsupported(pattern, value, *span, *mutable)
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), FerriError> {
        match expr {
            Expr::IntLiteral(n, suffix, span) => {
                let width = match suffix {
                    IntegerSuffix::I8 => IntegerWidth::I8,
                    IntegerSuffix::I16 => IntegerWidth::I16,
                    IntegerSuffix::I32 => IntegerWidth::I32,
                    IntegerSuffix::I64 => IntegerWidth::I64,
                    IntegerSuffix::U8 => IntegerWidth::U8,
                    IntegerSuffix::U16 => IntegerWidth::U16,
                    IntegerSuffix::U32 => IntegerWidth::U32,
                    IntegerSuffix::U64 => IntegerWidth::U64,
                    IntegerSuffix::None => IntegerWidth::I64,
                };
                // Check that suffixed literals fit in their declared type
                if *suffix != IntegerSuffix::None {
                    validate_int_literal(*n, &width, *span)?;
                }
                self.emit(OpCode::ConstInt(*n, width));
                Ok(())
            }
            Expr::FloatLiteral(n, suffix, _) => {
                let width = match suffix {
                    FloatSuffix::F32 => FloatWidth::F32,
                    FloatSuffix::F64 => FloatWidth::F64,
                    FloatSuffix::None => FloatWidth::F64,
                };
                self.emit(OpCode::ConstFloat(*n, width));
                Ok(())
            }
            Expr::BoolLiteral(b, _) => {
                self.emit(OpCode::ConstBool(*b));
                Ok(())
            }
            Expr::StringLiteral(s, _) => {
                self.emit(OpCode::ConstString(s.clone()));
                Ok(())
            }
            Expr::CharLiteral(c, _) => {
                self.emit(OpCode::ConstChar(*c));
                Ok(())
            }

            Expr::Ident(name, span) => {
                // Handle bare enum variant constructors without parens
                match name.as_str() {
                    "None" => {
                        self.emit(OpCode::MakeEnumVariant {
                            enum_name: "Option".to_string(),
                            variant: "None".to_string(),
                            arg_count: 0,
                        });
                        return Ok(());
                    }
                    _ => {}
                }
                // Check const values first (compile-time inlined)
                if let Some(val) = self.const_values.get(name) {
                    match val {
                        crate::types::Value::I64(n) => {
                            self.emit(OpCode::ConstInt(*n, IntegerWidth::I64));
                        }
                        crate::types::Value::F64(n) => {
                            self.emit(OpCode::ConstFloat(*n, FloatWidth::F64));
                        }
                        crate::types::Value::Bool(b) => {
                            self.emit(OpCode::ConstBool(*b));
                        }
                        crate::types::Value::String(s) => {
                            self.emit(OpCode::ConstString(s.clone()));
                        }
                        crate::types::Value::Char(c) => {
                            self.emit(OpCode::ConstChar(*c));
                        }
                        crate::types::Value::Unit | _ => {
                            self.emit(OpCode::ConstUnit);
                        }
                    }
                    return Ok(());
                }
                if let Some(slot) = self.sym.get(name) {
                    self.emit(OpCode::LoadLocal(slot));
                    Ok(())
                } else {
                    let resolved = self
                        .use_aliases
                        .get(name)
                        .cloned()
                        .unwrap_or_else(|| name.clone());
                    if let Some(target) = self.functions.get(&resolved).copied() {
                        // Emit a function reference as a Value::Function pointing to the
                        // existing compiled function body at `target`.
                        let (params, body_expr, _return_type) =
                            self.fn_meta.get(&resolved).cloned().unwrap_or_else(|| {
                                (
                                    vec![],
                                    Box::new(crate::ast::Expr::IntLiteral(
                                        0,
                                        IntegerSuffix::None,
                                        *span,
                                    )),
                                    None,
                                )
                            });
                        let meta_idx = self.closure_meta.len();
                        let param_names: Vec<String> =
                            params.iter().map(|p| p.name.clone()).collect();
                        self.closure_meta.push((param_names, *body_expr, vec![]));
                        self.emit(OpCode::Closure {
                            target_ip: target,
                            param_count: params.len(),
                            meta_idx,
                        });
                        Ok(())
                    } else {
                        // Suggest similar variable names
                        let suggestion = self
                            .sym
                            .build_slot_names()
                            .into_iter()
                            .filter(|n| !n.is_empty())
                            .map(|n| (crate::errors::edit_distance(name, &n), n))
                            .filter(|(d, _)| *d <= 2)
                            .min_by_key(|(d, _)| *d);
                        let msg = if let Some((_, suggestion)) = suggestion {
                            format!("undefined variable '{name}'; did you mean '{suggestion}'?")
                        } else {
                            format!("undefined variable '{name}'")
                        };
                        Err(FerriError::Runtime {
                            message: msg,
                            line: span.line,
                            column: span.column,
                        })
                    }
                }
            }

            Expr::BinaryOp {
                left,
                op,
                right,
                span: _,
            } => {
                // Short-circuit && and ||
                if *op == BinOp::And {
                    self.compile_expr(left)?;
                    self.emit(OpCode::Dup); // preserve left for false case
                    let jump = self.emit(OpCode::JumpIfFalse(0));
                    self.emit(OpCode::Pop); // discard dup; left is false, keep it
                    self.compile_expr(right)?;
                    self.patch(jump, OpCode::JumpIfFalse(self.code.len()));
                    return Ok(());
                }
                if *op == BinOp::Or {
                    self.compile_expr(left)?;
                    self.emit(OpCode::Dup); // preserve left for true case
                    let jump = self.emit(OpCode::JumpIfTrue(0));
                    self.emit(OpCode::Pop); // discard left; evaluate right
                    self.compile_expr(right)?;
                    self.patch(jump, OpCode::JumpIfTrue(self.code.len()));
                    return Ok(());
                }
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                let opcode = match op {
                    BinOp::Add => OpCode::Add,
                    BinOp::Sub => OpCode::Sub,
                    BinOp::Mul => OpCode::Mul,
                    BinOp::Div => OpCode::Div,
                    BinOp::Mod => OpCode::Mod,
                    BinOp::Eq => OpCode::Eq,
                    BinOp::NotEq => OpCode::Neq,
                    BinOp::Lt => OpCode::Lt,
                    BinOp::Gt => OpCode::Gt,
                    BinOp::LtEq => OpCode::Le,
                    BinOp::GtEq => OpCode::Ge,
                    BinOp::BitAnd => OpCode::BitAnd,
                    BinOp::BitOr => OpCode::BitOr,
                    BinOp::BitXor => OpCode::BitXor,
                    BinOp::Shl => OpCode::Shl,
                    BinOp::Shr => OpCode::Shr,
                    BinOp::And | BinOp::Or => unreachable!(),
                };
                self.emit(opcode);
                Ok(())
            }

            Expr::UnaryOp {
                op,
                expr: inner,
                span,
            } => {
                self.compile_expr(inner)?;
                match op {
                    UnaryOp::Neg => self.emit(OpCode::Neg),
                    UnaryOp::Not => self.emit(OpCode::Not),
                    UnaryOp::BitNot => self.emit(OpCode::BitNot),
                    UnaryOp::Ref => return Ok(()), // & is a no-op in Oxy (no ownership/borrowing)
                    _ => {
                        return Err(FerriError::Runtime {
                            message: format!("unsupported unary op in compiler: {:?}", op),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                Ok(())
            }

            Expr::Call { callee, args, .. } => {
                // Handle bare enum constructors: Some(val), None, Ok(val), Err(val)
                if let Expr::Ident(name, _) = callee.as_ref() {
                    let enum_info: Option<(&str, &str)> = match name.as_str() {
                        "Some" => Some(("Option", "Some")),
                        "None" => Some(("Option", "None")),
                        "Ok" => Some(("Result", "Ok")),
                        "Err" => Some(("Result", "Err")),
                        _ => None,
                    };
                    if let Some((enum_name, variant)) = enum_info {
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit(OpCode::MakeEnumVariant {
                            enum_name: enum_name.to_string(),
                            variant: variant.to_string(),
                            arg_count: args.len(),
                        });
                        return Ok(());
                    }
                }

                // Determine if this is a direct function call (compile-time resolved)
                let direct_target: Option<usize> = if let Expr::Ident(name, _) = callee.as_ref() {
                    if name == "println!" || name == "print!" {
                        // Compile args first, then emit print
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        if name == "println!" {
                            self.emit(OpCode::PrintLn);
                        } else {
                            self.emit(OpCode::Print);
                        }
                        return Ok(());
                    }
                    // Follow use_aliases chain (handles pub use re-exports)
                    let mut resolved = name.clone();
                    let mut seen: HashSet<&str> = HashSet::new();
                    while let Some(alias_target) = self.use_aliases.get(&resolved) {
                        if !seen.insert(alias_target) {
                            break; // cycle guard
                        }
                        resolved = alias_target.clone();
                    }
                    self.functions
                        .get(&resolved)
                        .copied()
                        .or_else(|| self.functions.get(name).copied())
                } else {
                    None
                };

                if let Some(target) = direct_target {
                    // Check argument count against function definition
                    if let Expr::Ident(name, _) = callee.as_ref() {
                        let resolved = self
                            .use_aliases
                            .get(name)
                            .cloned()
                            .unwrap_or_else(|| name.clone());
                        if let Some((params, _, _)) = self
                            .fn_meta
                            .get(&resolved)
                            .or_else(|| self.fn_meta.get(name))
                        {
                            if args.len() != params.len() {
                                return Err(FerriError::Runtime {
                                    message: format!(
                                        "function '{}' expects {} argument{}, but {} {} provided",
                                        resolved,
                                        params.len(),
                                        if params.len() == 1 { "" } else { "s" },
                                        args.len(),
                                        if args.len() == 1 { "was" } else { "were" },
                                    ),
                                    line: expr.span().line,
                                    column: expr.span().column,
                                });
                            }
                        }
                    }
                    // Direct call: compile args first, emit Call
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    let call_idx = self.emit(OpCode::Call {
                        target,
                        arg_count: args.len(),
                    });
                    // Forward reference: record for patching after all functions compiled
                    if target == usize::MAX {
                        if let Expr::Ident(name, _) = callee.as_ref() {
                            let resolved = self
                                .use_aliases
                                .get(name)
                                .cloned()
                                .unwrap_or_else(|| name.clone());
                            self.forward_calls.push((call_idx, resolved));
                        }
                    }
                } else {
                    // Check if callee is a local variable (closure/function value)
                    if let Expr::Ident(name, _) = callee.as_ref() {
                        if self.sym.get(name).is_some() {
                            // Native CallClosure for indirect calls
                            self.compile_expr(callee)?;
                            for arg in args {
                                self.compile_expr(arg)?;
                            }
                            self.emit(OpCode::CallClosure {
                                arg_count: args.len(),
                            });
                            return Ok(());
                        }
                    }
                    // Unknown function — not yet supported in native bytecode
                    return Err(self.not_yet_supported("Call to unknown function", expr.span()));
                }
                Ok(())
            }

            Expr::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.compile_expr(condition)?;
                let jump_else = self.emit(OpCode::JumpIfFalse(0)); // placeholder
                self.compile_block(then_block)?;
                let jump_end = if else_block.is_some() {
                    Some(self.emit(OpCode::Jump(0))) // placeholder
                } else {
                    None
                };
                // Patch jump_else to point here (after then_block)
                let after_then = self.code.len();
                self.patch(jump_else, OpCode::JumpIfFalse(after_then));
                if let Some(else_expr) = else_block {
                    self.compile_expr(else_expr)?;
                    let after_else = self.code.len();
                    self.patch(jump_end.unwrap(), OpCode::Jump(after_else));
                }
                Ok(())
            }

            Expr::Block(block) => self.compile_block(block),

            Expr::Grouped(inner, _) => self.compile_expr(inner),

            Expr::Assign {
                target,
                value,
                span,
            } => {
                if let Expr::Ident(name, _) = target.as_ref() {
                    // Check immutability: variable already defined but not mutable
                    if self.sym.get(name).is_some() && !self.sym.is_mutable(name) {
                        return Err(FerriError::Runtime {
                            message: format!("cannot assign to immutable variable `{name}`"),
                            line: span.line,
                            column: span.column,
                        });
                    }
                    self.compile_expr(value)?;
                    if let Some(slot) = self.sym.get(name) {
                        self.emit(OpCode::Dup);
                        self.emit(OpCode::StoreLocal(slot));
                    } else {
                        let slot = self.sym.define(name);
                        self.emit(OpCode::Dup);
                        self.emit(OpCode::StoreLocal(slot));
                    }
                    Ok(())
                } else if let Expr::FieldAccess { object, field, .. } = target.as_ref() {
                    // Field assignment: compile object, push value, emit FieldStore,
                    // then store back to the original variable if it's a local.
                    self.compile_expr(object)?;
                    self.compile_expr(value)?;
                    self.emit(OpCode::FieldStore(field.clone()));
                    // If the object is a local/SelfRef, store the updated struct back
                    match object.as_ref() {
                        Expr::Ident(name, _) => {
                            if let Some(slot) = self.sym.get(name) {
                                self.emit(OpCode::Dup);
                                self.emit(OpCode::StoreLocal(slot));
                            }
                        }
                        Expr::SelfRef(_) => {
                            // self is always at slot 0 in methods
                            self.emit(OpCode::Dup);
                            self.emit(OpCode::StoreLocal(0));
                        }
                        _ => {}
                    }
                    Ok(())
                } else if let Expr::Index { object, index, .. } = target.as_ref() {
                    self.compile_expr(object)?;
                    self.compile_expr(index)?;
                    self.compile_expr(value)?;
                    self.emit(OpCode::VecIndexStore);
                    Ok(())
                } else {
                    Err(FerriError::Runtime {
                        message: "compiled: only simple variable assignment supported".into(),
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
                    if let Some(slot) = self.sym.get(name) {
                        if !self.sym.is_mutable(name) {
                            return Err(FerriError::Runtime {
                                message: format!("cannot assign to immutable variable `{name}`"),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        self.emit(OpCode::LoadLocal(slot));
                        self.compile_expr(value)?;
                        let opcode = match op {
                            BinOp::Add => OpCode::Add,
                            BinOp::Sub => OpCode::Sub,
                            BinOp::Mul => OpCode::Mul,
                            BinOp::Div => OpCode::Div,
                            BinOp::Mod => OpCode::Mod,
                            _ => {
                                return Err(FerriError::Runtime {
                                    message: format!(
                                        "unsupported compound op in compiler: {:?}",
                                        op
                                    ),
                                    line: span.line,
                                    column: span.column,
                                })
                            }
                        };
                        self.emit(opcode);
                        self.emit(OpCode::StoreLocal(slot));
                        Ok(())
                    } else {
                        Err(FerriError::Runtime {
                            message: format!("compiled: undefined variable '{}'", name),
                            line: span.line,
                            column: span.column,
                        })
                    }
                } else {
                    Err(self.not_yet_supported("Compound assign on field/index", expr.span()))
                }
            }

            Expr::Try { expr: inner, .. } => {
                self.compile_expr(inner)?;
                self.emit(OpCode::TryPop);
                Ok(())
            }

            Expr::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                if let Some(s) = start {
                    self.compile_expr(s)?;
                } else {
                    self.emit(OpCode::ConstInt(i64::MIN, IntegerWidth::I64));
                }
                if let Some(e) = end {
                    self.compile_expr(e)?;
                } else {
                    self.emit(OpCode::ConstInt(i64::MAX, IntegerWidth::I64));
                }
                if *inclusive {
                    self.emit(OpCode::ConstInt(1, IntegerWidth::I64));
                    self.emit(OpCode::Add);
                }
                self.emit(OpCode::MakeRange);
                Ok(())
            }

            Expr::Array { elements, .. } => {
                let count = elements.len();
                for elem in elements {
                    self.compile_expr(elem)?;
                }
                self.emit(OpCode::MakeArray { count });
                Ok(())
            }

            Expr::Tuple { elements, .. } => {
                let count = elements.len();
                for elem in elements {
                    self.compile_expr(elem)?;
                }
                self.emit(OpCode::MakeTuple { count });
                Ok(())
            }

            Expr::Index { object, index, .. } => {
                self.compile_expr(object)?;
                self.compile_expr(index)?;
                self.emit(OpCode::VecIndex);
                Ok(())
            }

            Expr::FieldAccess { object, field, .. } => {
                self.compile_expr(object)?;
                if let Ok(idx) = field.parse::<i64>() {
                    self.emit(OpCode::ConstInt(idx, IntegerWidth::I64));
                    self.emit(OpCode::VecIndex);
                } else {
                    self.emit(OpCode::FieldAccess {
                        field_name: field.clone(),
                    });
                }
                Ok(())
            }

            Expr::FString { parts, .. } => {
                let mut count = 0usize;
                for part in parts {
                    match part {
                        FStringPart::Literal(s) => {
                            self.emit(OpCode::ConstString(s.clone()));
                            count += 1;
                        }
                        FStringPart::Expr(expr) => {
                            self.compile_expr(expr)?;
                            self.emit(OpCode::ToString);
                            count += 1;
                        }
                    }
                }
                self.emit(OpCode::FStringConcat { count });
                Ok(())
            }

            Expr::SelfRef(_) => {
                // `self` is always the first parameter → slot 0.
                self.emit(OpCode::LoadLocal(0));
                Ok(())
            }

            Expr::StructInit { name, fields, .. } => {
                // Resolve `Self` to the current impl type name, then type aliases, then use aliases
                let resolved_name = if name == "Self" {
                    self.current_impl_type
                        .clone()
                        .unwrap_or_else(|| name.clone())
                } else {
                    self.type_aliases
                        .get(name)
                        .cloned()
                        .or_else(|| self.use_aliases.get(name).cloned())
                        .unwrap_or_else(|| name.clone())
                };
                // Check if this is an enum variant constructor (e.g. Message::Move { x, y })
                if resolved_name.contains("::") {
                    let parts: Vec<&str> = resolved_name.split("::").collect();
                    if parts.len() == 2 {
                        let enum_name = parts[0].to_string();
                        let variant = parts[1].to_string();
                        if self.enum_defs.contains_key(&enum_name) {
                            // Compile field values in order
                            for (_, expr) in fields {
                                self.compile_expr(expr)?;
                            }
                            self.emit(OpCode::MakeEnumVariant {
                                enum_name,
                                variant,
                                arg_count: fields.len(),
                            });
                            return Ok(());
                        }
                    }
                }
                let field_names: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
                for (_, expr) in fields {
                    self.compile_expr(expr)?;
                }
                self.emit(OpCode::StructInit {
                    name: resolved_name,
                    field_count: fields.len(),
                    field_names,
                });
                Ok(())
            }

            Expr::MethodCall {
                object,
                method,
                args,
                ..
            } => {
                // If the receiver is a local variable, check if this is an
                // &mut self method so we can write the result back.
                let receiver_slot = if let Expr::Ident(name, _) = object.as_ref() {
                    self.sym.get(name).filter(|_| {
                        // Only write back for &mut self methods (return_type is None
                        // and first param is "self").
                        self.fn_meta.get(method).map_or(false, |(params, _, ret)| {
                            ret.is_none() && params.first().map_or(false, |p| p.name == "self")
                        })
                    })
                } else {
                    None
                };
                self.compile_expr(object)?;
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit(OpCode::MethodCall {
                    method_name: method.clone(),
                    arg_count: args.len(),
                });
                if let Some(slot) = receiver_slot {
                    self.emit(OpCode::Dup);
                    self.emit(OpCode::StoreLocal(slot));
                }
                Ok(())
            }

            Expr::Path { segments, .. } => {
                if segments.len() == 2 {
                    let enum_name = &segments[0];
                    let variant = &segments[1];
                    // Resolve via type aliases and use aliases
                    let resolved_enum = self
                        .type_aliases
                        .get(enum_name)
                        .cloned()
                        .or_else(|| self.use_aliases.get(enum_name).cloned())
                        .unwrap_or_else(|| enum_name.clone());
                    let enum_key = self
                        .enum_defs
                        .get(enum_name)
                        .or_else(|| self.enum_defs.get(&resolved_enum));
                    if let Some(ed) = enum_key {
                        for v in &ed.variants {
                            if &v.name == variant {
                                self.emit(OpCode::ConstEnumVariant {
                                    enum_name: resolved_enum.clone(),
                                    variant: variant.clone(),
                                    data: vec![],
                                });
                                return Ok(());
                            }
                        }
                    }
                    if enum_name == "math" {
                        match variant.as_str() {
                            "PI" => {
                                self.emit(OpCode::ConstFloat(
                                    std::f64::consts::PI,
                                    FloatWidth::F64,
                                ));
                                return Ok(());
                            }
                            "E" => {
                                self.emit(OpCode::ConstFloat(std::f64::consts::E, FloatWidth::F64));
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                }
                Err(self.not_yet_supported("Unknown path", expr.span()))
            }

            Expr::PathCall { path, args, .. } => {
                for arg in args {
                    self.compile_expr(arg)?;
                }
                if path.len() == 2 {
                    let enum_name = &path[0];
                    let variant = &path[1];
                    if self.enum_defs.contains_key(enum_name) {
                        self.emit(OpCode::MakeEnumVariant {
                            enum_name: enum_name.clone(),
                            variant: variant.clone(),
                            arg_count: args.len(),
                        });
                        return Ok(());
                    }
                    let qualified = format!("{}::{}", &path[0], &path[1]);
                    if let Some(&target) = self.functions.get(&qualified) {
                        let call_idx = self.emit(OpCode::Call {
                            target,
                            arg_count: args.len(),
                        });
                        if target == usize::MAX {
                            self.forward_calls.push((call_idx, qualified));
                        }
                        return Ok(());
                    }
                    // Try type alias + use-aliased prefix
                    let prefix = &path[0];
                    let resolved_prefix = self
                        .type_aliases
                        .get(prefix)
                        .cloned()
                        .or_else(|| self.use_aliases.get(prefix).cloned());
                    if let Some(rp) = resolved_prefix {
                        let aliased = format!("{}::{}", rp, &path[1]);
                        if let Some(&target) = self.functions.get(&aliased) {
                            let call_idx = self.emit(OpCode::Call {
                                target,
                                arg_count: args.len(),
                            });
                            if target == usize::MAX {
                                self.forward_calls.push((call_idx, aliased));
                            }
                            return Ok(());
                        }
                    }
                    // Try resolving the full qualified name through use_aliases
                    // (handles pub use re-exports like middle::msg -> inner::msg)
                    if let Some(aliased_target) = self.use_aliases.get(&qualified).cloned() {
                        if let Some(&target) = self.functions.get(&aliased_target) {
                            let call_idx = self.emit(OpCode::Call {
                                target,
                                arg_count: args.len(),
                            });
                            if target == usize::MAX {
                                self.forward_calls.push((call_idx, aliased_target));
                            }
                            return Ok(());
                        }
                    }
                    // Try qualifying with current module prefix (sibling module calls)
                    let module_prefix = self.module_stack.join("::");
                    if !module_prefix.is_empty() {
                        let module_qualified =
                            format!("{}::{}::{}", module_prefix, &path[0], &path[1]);
                        if let Some(&target) = self.functions.get(&module_qualified) {
                            let call_idx = self.emit(OpCode::Call {
                                target,
                                arg_count: args.len(),
                            });
                            if target == usize::MAX {
                                self.forward_calls.push((call_idx, module_qualified));
                            }
                            return Ok(());
                        }
                    }
                    if is_builtin_path(path) {
                        self.emit(OpCode::PathCallBuiltin {
                            segments: path.clone(),
                            arg_count: args.len(),
                        });
                        return Ok(());
                    }
                }
                if path.len() == 3 {
                    let qualified = format!("{}::{}::{}", &path[0], &path[1], &path[2]);
                    if let Some(&target) = self.functions.get(&qualified) {
                        let call_idx = self.emit(OpCode::Call {
                            target,
                            arg_count: args.len(),
                        });
                        if target == usize::MAX {
                            self.forward_calls.push((call_idx, qualified));
                        }
                        return Ok(());
                    }
                    // Try qualifying with current module prefix
                    let module_prefix = self.module_stack.join("::");
                    if !module_prefix.is_empty() {
                        let module_qualified = format!(
                            "{}::{}::{}::{}",
                            module_prefix, &path[0], &path[1], &path[2]
                        );
                        if let Some(&target) = self.functions.get(&module_qualified) {
                            let call_idx = self.emit(OpCode::Call {
                                target,
                                arg_count: args.len(),
                            });
                            if target == usize::MAX {
                                self.forward_calls.push((call_idx, module_qualified));
                            }
                            return Ok(());
                        }
                    }
                }
                // Check is_builtin_path for any path length (catches
                // std::env::args(), etc. that are >2 segments)
                if is_builtin_path(path) {
                    self.emit(OpCode::PathCallBuiltin {
                        segments: path.clone(),
                        arg_count: args.len(),
                    });
                    return Ok(());
                }
                Err(self.not_yet_supported("Unknown path call", expr.span()))
            }

            Expr::Closure { params, body, .. } => {
                // Emit a jump to skip over the closure body in the instruction stream
                let skip_jump_idx = self.emit(OpCode::Jump(0));
                let target_ip = self.code.len();
                // Swap in a fresh sym table so closure params start at slot 0.
                // Outer sym is needed to resolve captured variable slots.
                let saved_sym = std::mem::replace(&mut self.sym, SymTable::new(0));
                // Pre-scan: find which outer variables the closure body references.
                // Register them in the fresh sym at their outer slot positions so
                // LoadLocal in the closure body emits the correct frame offset.
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                let captured_names = find_free_vars(body, &param_names);
                let captured: Vec<(String, usize, bool)> = captured_names
                    .iter()
                    .filter_map(|name| {
                        saved_sym.get(name).map(|slot| {
                            let is_mut = saved_sym.is_mutable(name);
                            // Register captured var in closure sym at its outer slot,
                            // preserving mutability so assignments work inside the closure.
                            self.sym.define_at(name, slot);
                            if is_mut {
                                self.sym.mutable.insert(name.clone());
                            }
                            (name.clone(), slot, is_mut)
                        })
                    })
                    .collect();
                // Now define params — they get slots above the captured vars
                for param in params {
                    self.sym.define(&param.name);
                }
                self.compile_expr(body)?;
                self.emit(OpCode::Return);
                self.sym = saved_sym;
                // Patch the skip jump to land after the Return
                self.patch(skip_jump_idx, OpCode::Jump(self.code.len()));
                let meta_idx = self.closure_meta.len();
                self.closure_meta
                    .push((param_names, *body.clone(), captured));
                self.emit(OpCode::Closure {
                    target_ip,
                    param_count: params.len(),
                    meta_idx,
                });
                Ok(())
            }

            Expr::Match {
                expr: scrutinee,
                arms,
                ..
            } => {
                // Exhaustiveness check: require wildcard for integer literal matches.
                // Enum variants, bool, and ident patterns are fine without catch-all.
                let has_catch_all = arms
                    .iter()
                    .any(|a| matches!(a.pattern, Pattern::Wildcard(_) | Pattern::Ident(..)));
                let has_enum = arms
                    .iter()
                    .any(|a| matches!(a.pattern, Pattern::EnumVariant { .. }));
                let has_int_literal = arms
                    .iter()
                    .any(|a| matches!(a.pattern, Pattern::Literal(Expr::IntLiteral(..))));
                if !has_catch_all && !has_enum && has_int_literal {
                    return Err(FerriError::Runtime {
                        message: "non-exhaustive patterns: missing wildcard `_` arm".into(),
                        line: expr.span().line,
                        column: expr.span().column,
                    });
                }
                // Evaluate scrutinee once, store in temp slot
                self.compile_expr(scrutinee)?;
                let scrutinee_slot = self.sym.define("__match_scrutinee");
                let current_slot = self.sym.next_slot;
                self.emit(OpCode::StoreLocal(scrutinee_slot));

                let _match_end_label = self.code.len(); // placeholder
                let mut arm_jumps: Vec<usize> = vec![];
                let mut guard_fail_jumps: Vec<usize> = vec![];

                for (i, arm) in arms.iter().enumerate() {
                    let is_last = i == arms.len() - 1;

                    // Pop leftover scrutinee from a previous failed arm
                    self.emit(OpCode::Pop);
                    // Push scrutinee for this arm
                    self.emit(OpCode::LoadLocal(scrutinee_slot));

                    // Compile pattern check → leaves Bool(true)+data or Bool(false)
                    let consumes_scrutinee = matches!(arm.pattern, Pattern::EnumVariant { .. });
                    self.compile_pattern(&arm.pattern, &mut vec![], is_last)?;

                    // JumpIfFalse to next arm if pattern didn't match
                    let jump_to_next = self.emit(OpCode::JumpIfFalse(0));

                    // Pattern matched. EnumVariant consumed the scrutinee;
                    // other patterns left it on stack → Pop it.
                    if !consumes_scrutinee {
                        self.emit(OpCode::Pop); // discard scrutinee
                    }
                    self.bind_pattern_data(&arm.pattern)?;

                    // Compile guard if present
                    if let Some(guard) = &arm.guard {
                        self.compile_expr(guard)?;
                        let guard_jump = self.emit(OpCode::JumpIfFalse(0));
                        guard_fail_jumps.push(guard_jump);
                        // Clean up guard bindings before next arm
                        self.sym.next_slot = current_slot;
                    }

                    // Compile arm body
                    self.compile_expr(&arm.body)?;

                    // Jump to match end
                    arm_jumps.push(self.emit(OpCode::Jump(0)));

                    // Patch the "jump to next arm" from pattern check
                    self.patch(jump_to_next, OpCode::JumpIfFalse(self.code.len()));
                    // Patch guard-fail jumps to the next arm too
                    for gj in &guard_fail_jumps {
                        self.patch(*gj, OpCode::JumpIfFalse(self.code.len()));
                    }
                    guard_fail_jumps.clear();

                    // Clean up sym for bindings in this arm
                    self.sym.next_slot = current_slot;
                }

                // If we reach here, no arm matched → runtime error
                self.emit(OpCode::ConstString("match: no arm matched".into()));
                self.emit(OpCode::PrintLn);

                // Match end: patch all arm jumps
                let end = self.code.len();
                for j in &arm_jumps {
                    self.patch(*j, OpCode::Jump(end));
                }
                // Patch the match_end placeholder
                // (Actually the placeholder was never used since we emit labels dynamically)

                Ok(())
            }

            Expr::IfLet {
                pattern,
                expr: scrutinee,
                then_block,
                else_block,
                ..
            } => {
                // Evaluate scrutinee, store in temp
                self.compile_expr(scrutinee)?;
                let scrut_slot = self.sym.define("__iflet_scrutinee");
                let current_slot = self.sym.next_slot;
                self.emit(OpCode::StoreLocal(scrut_slot));

                // Pattern check
                self.emit(OpCode::LoadLocal(scrut_slot));
                let consumes = matches!(pattern.as_ref(), Pattern::EnumVariant { .. });
                self.compile_pattern(pattern, &mut vec![], true)?;
                let jump_to_else = self.emit(OpCode::JumpIfFalse(0));

                // Matched: clean up, bind, compile then block
                if !consumes {
                    self.emit(OpCode::Pop);
                }
                self.bind_pattern_data(pattern)?;
                self.compile_block(then_block)?;

                // Jump over else block
                let jump_to_end = self.emit(OpCode::Jump(0));

                // Else block
                self.patch(jump_to_else, OpCode::JumpIfFalse(self.code.len()));
                self.sym.next_slot = current_slot;
                if let Some(else_expr) = else_block {
                    self.compile_expr(else_expr)?;
                } else {
                    self.emit(OpCode::ConstUnit);
                }

                // End
                self.patch(jump_to_end, OpCode::Jump(self.code.len()));
                Ok(())
            }

            Expr::Await { .. } => Err(self.not_yet_supported("await", expr.span())),

            Expr::MacroCall { name, args, .. } => {
                // For println!/print!/format! with simple {} format strings,
                // emit native DisplayArg for each arg to enable Display::fmt dispatch.
                let is_println = name == "println" || name == "print";
                let is_format = name == "format";
                if (is_println || is_format) && args.len() > 1 {
                    // Parse format string: split on "{}" and emit parts + DisplayArg
                    let fmt = match &args[0] {
                        Expr::StringLiteral(s, _) => s.clone(),
                        Expr::FString { .. } => String::new(), // f-strings handled elsewhere
                        _ => String::new(),
                    };
                    let parts: Vec<&str> = fmt.split("{}").collect();
                    // If there are {:?} placeholders, fall back to Format opcode
                    if fmt.contains("{:?}") {
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit(OpCode::Format {
                            arg_count: args.len(),
                        });
                    } else if parts.len() == args.len() {
                        // Interleave format parts and args: part0, arg1, part1, arg2, part2, ...
                        let mut concat_count = 0usize;
                        for i in 0..parts.len() {
                            // Emit the literal part
                            if !parts[i].is_empty() {
                                self.emit(OpCode::ConstString(parts[i].to_string()));
                                concat_count += 1;
                            }
                            // Emit the arg (except for the last part)
                            if i < args.len() - 1 {
                                self.compile_expr(&args[i + 1])?;
                                self.emit(OpCode::DisplayArg);
                                concat_count += 1;
                            }
                        }
                        if concat_count > 1 {
                            self.emit(OpCode::FStringConcat {
                                count: concat_count,
                            });
                        }
                    } else {
                        // Mismatched {} count — fall back to Format
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit(OpCode::Format {
                            arg_count: args.len(),
                        });
                    }
                    if is_println {
                        if name == "println" {
                            self.emit(OpCode::PrintLn);
                        } else {
                            self.emit(OpCode::Print);
                        }
                    }
                } else if (is_println || is_format) && args.len() == 1 {
                    // No format args — just print/format the literal
                    self.compile_expr(&args[0])?;
                    if name == "println" {
                        self.emit(OpCode::PrintLn);
                    } else if name == "print" {
                        self.emit(OpCode::Print);
                    }
                    // format! with no args just returns the string
                } else if name == "vec" {
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    self.emit(OpCode::MakeArray { count: args.len() });
                } else if name == "format" {
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    self.emit(OpCode::Format {
                        arg_count: args.len(),
                    });
                } else if name == "panic" {
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    self.emit(OpCode::Panic);
                } else if name == "assert" {
                    // assert!(cond) or assert!(cond, "message")
                    self.compile_expr(&args[0])?; // compile condition
                    let skip = self.emit(OpCode::JumpIfTrue(0));
                    if args.len() > 1 {
                        self.compile_expr(&args[1])?; // custom message
                    } else {
                        self.emit(OpCode::ConstString("assertion failed".to_string()));
                    }
                    self.emit(OpCode::Panic);
                    self.patch(skip, OpCode::JumpIfTrue(self.code.len()));
                } else if name == "assert_eq" {
                    // assert_eq!(left, right) or assert_eq!(left, right, "message")
                    self.compile_expr(&args[0])?;
                    self.compile_expr(&args[1])?;
                    self.emit(OpCode::Eq);
                    let skip = self.emit(OpCode::JumpIfTrue(0));
                    if args.len() > 2 {
                        self.compile_expr(&args[2])?;
                    } else {
                        self.emit(OpCode::ConstString(
                            "assertion failed: left != right".to_string(),
                        ));
                    }
                    self.emit(OpCode::Panic);
                    self.patch(skip, OpCode::JumpIfTrue(self.code.len()));
                } else if name == "assert_ne" {
                    // assert_ne!(left, right) or assert_ne!(left, right, "message")
                    self.compile_expr(&args[0])?;
                    self.compile_expr(&args[1])?;
                    self.emit(OpCode::Neq);
                    let skip = self.emit(OpCode::JumpIfTrue(0));
                    if args.len() > 2 {
                        self.compile_expr(&args[2])?;
                    } else {
                        self.emit(OpCode::ConstString(
                            "assertion failed: left == right".to_string(),
                        ));
                    }
                    self.emit(OpCode::Panic);
                    self.patch(skip, OpCode::JumpIfTrue(self.code.len()));
                } else if name == "dbg" {
                    // dbg!(expr) — print debug representation and return the value
                    self.compile_expr(&args[0])?;
                    self.emit(OpCode::Dup);
                    self.emit(OpCode::PrintLn);
                } else {
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    return Err(self.not_yet_supported("Unknown macro", expr.span()));
                }
                Ok(())
            }
            Expr::As {
                expr: inner,
                type_name,
                ..
            } => {
                self.compile_expr(inner)?;
                match type_name.as_str() {
                    "i8" => self.emit(OpCode::CastInt(IntegerWidth::I8)),
                    "i16" => self.emit(OpCode::CastInt(IntegerWidth::I16)),
                    "i32" => self.emit(OpCode::CastInt(IntegerWidth::I32)),
                    "i64" | "Integer" => self.emit(OpCode::CastInt(IntegerWidth::I64)),
                    "u8" => self.emit(OpCode::CastInt(IntegerWidth::U8)),
                    "u16" => self.emit(OpCode::CastInt(IntegerWidth::U16)),
                    "u32" => self.emit(OpCode::CastInt(IntegerWidth::U32)),
                    "u64" => self.emit(OpCode::CastInt(IntegerWidth::U64)),
                    "f32" => self.emit(OpCode::CastFloat(FloatWidth::F32)),
                    "f64" | "Float" => self.emit(OpCode::CastFloat(FloatWidth::F64)),
                    "char" => self.emit(OpCode::CastToChar),
                    _ => return Ok(()),
                };
                Ok(())
            }
            Expr::Return { value, .. } => {
                if let Some(expr) = value {
                    self.compile_expr(expr)?;
                } else {
                    self.emit(OpCode::ConstUnit);
                }
                self.emit(OpCode::Return);
                Ok(())
            }
        }
    }
}

/// Find all free variables in an expression (variables used but not defined in `params`).
/// Pre-scan: find names of free variables in all closures in the function body.
fn find_captured_mutable(block: &crate::ast::Block, params: &[String]) -> HashSet<String> {
    let mut captured = HashSet::new();
    collect_closure_free_vars_in_block(block, params, &mut captured);
    captured
}

fn collect_closure_free_vars_in_block(
    block: &crate::ast::Block,
    params: &[String],
    out: &mut HashSet<String>,
) {
    for stmt in &block.stmts {
        match stmt {
            crate::ast::Stmt::Expr { expr, .. } => collect_closure_free_vars(expr, params, out),
            crate::ast::Stmt::Let { value, .. } => {
                if let Some(v) = value {
                    collect_closure_free_vars(v, params, out);
                }
            }
            crate::ast::Stmt::While {
                condition, body, ..
            } => {
                collect_closure_free_vars(condition, params, out);
                collect_closure_free_vars_in_block(body, params, out);
            }
            crate::ast::Stmt::Loop { body, .. } => {
                collect_closure_free_vars_in_block(body, params, out)
            }
            _ => {}
        }
    }
}

fn collect_closure_free_vars(
    expr: &crate::ast::Expr,
    params: &[String],
    out: &mut HashSet<String>,
) {
    match expr {
        crate::ast::Expr::Closure {
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
        crate::ast::Expr::BinaryOp { left, right, .. } => {
            collect_closure_free_vars(left, params, out);
            collect_closure_free_vars(right, params, out);
        }
        crate::ast::Expr::Call { callee, args, .. } => {
            collect_closure_free_vars(callee, params, out);
            for a in args {
                collect_closure_free_vars(a, params, out);
            }
        }
        crate::ast::Expr::MethodCall { object, args, .. } => {
            collect_closure_free_vars(object, params, out);
            for a in args {
                collect_closure_free_vars(a, params, out);
            }
        }
        crate::ast::Expr::UnaryOp { expr: inner, .. } => {
            collect_closure_free_vars(inner, params, out)
        }
        crate::ast::Expr::Assign { target, value, .. } => {
            collect_closure_free_vars(target, params, out);
            collect_closure_free_vars(value, params, out);
        }
        _ => {}
    }
}

fn find_free_vars(expr: &crate::ast::Expr, params: &[String]) -> Vec<String> {
    let mut vars = Vec::new();
    collect_free_vars(expr, params, &mut vars);
    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    vars.retain(|v| seen.insert(v.clone()));
    vars
}

fn collect_free_vars(expr: &crate::ast::Expr, params: &[String], vars: &mut Vec<String>) {
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
        _ => {} // Skip other expression types for now
    }
}

fn collect_free_vars_in_stmt(stmt: &crate::ast::Stmt, params: &[String], vars: &mut Vec<String>) {
    match stmt {
        crate::ast::Stmt::Expr { expr, .. } => collect_free_vars(expr, params, vars),
        crate::ast::Stmt::Let { value, .. } => {
            if let Some(val) = value {
                collect_free_vars(val, params, vars);
            }
        }
        crate::ast::Stmt::While {
            condition, body, ..
        } => {
            collect_free_vars(condition, params, vars);
            for s in &body.stmts {
                collect_free_vars_in_stmt(s, params, vars);
            }
        }
        crate::ast::Stmt::Loop { body, .. } => {
            for s in &body.stmts {
                collect_free_vars_in_stmt(s, params, vars);
            }
        }
        _ => {}
    }
}

/// Evaluate a simple constant expression at compile time.
fn try_eval_const(expr: &crate::ast::Expr) -> Option<crate::types::Value> {
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

/// Known built-in paths that the VM can dispatch natively.
/// Validate that an integer literal value fits in the target width (for suffixed literals).
/// Note: the lexer stores u64 values > i64::MAX as negative i64 via wrapping `as i64`,
/// so we reinterpret bits as u64 for unsigned width checks.
fn validate_int_literal(
    n: i64,
    width: &IntegerWidth,
    span: crate::lexer::Span,
) -> Result<(), FerriError> {
    let fits = match width {
        IntegerWidth::I8 => (i8::MIN as i64..=i8::MAX as i64).contains(&n),
        IntegerWidth::I16 => (i16::MIN as i64..=i16::MAX as i64).contains(&n),
        IntegerWidth::I32 => (i32::MIN as i64..=i32::MAX as i64).contains(&n),
        IntegerWidth::I64 => true,
        // For unsigned widths: reinterpret the bits as u64 to handle
        // values > i64::MAX that the lexer stored via wrapping as i64.
        IntegerWidth::U8 => (n as u64) <= u8::MAX as u64,
        IntegerWidth::U16 => (n as u64) <= u16::MAX as u64,
        IntegerWidth::U32 => (n as u64) <= u32::MAX as u64,
        IntegerWidth::U64 => true,
    };
    if !fits {
        return Err(FerriError::Runtime {
            message: format!("literal out of range for `{}`", width_to_str(width)),
            line: span.line,
            column: span.column,
        });
    }
    Ok(())
}

fn width_to_str(w: &IntegerWidth) -> &str {
    match w {
        IntegerWidth::I8 => "i8",
        IntegerWidth::I16 => "i16",
        IntegerWidth::I32 => "i32",
        IntegerWidth::I64 => "i64",
        IntegerWidth::U8 => "u8",
        IntegerWidth::U16 => "u16",
        IntegerWidth::U32 => "u32",
        IntegerWidth::U64 => "u64",
    }
}

/// Check that a constant integer literal fits in the target integer type's range.
/// Returns an error if the literal value is outside the type's bounds (matches Rust).
fn check_literal_fits_type(
    expr: &Expr,
    type_name: &str,
    span: crate::lexer::Span,
) -> Result<(), FerriError> {
    let (min, max): (i128, i128) = match type_name {
        "i8" => (i8::MIN as i128, i8::MAX as i128),
        "i16" => (i16::MIN as i128, i16::MAX as i128),
        "i32" => (i32::MIN as i128, i32::MAX as i128),
        "i64" | "isize" => (i64::MIN as i128, i64::MAX as i128),
        "u8" => (0, u8::MAX as i128),
        "u16" => (0, u16::MAX as i128),
        "u32" => (0, u32::MAX as i128),
        "u64" | "usize" => (0, u64::MAX as i128),
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

/// Emit a narrowing cast if the type annotation specifies an integer or float width.
fn emit_narrowing_cast(compiler: &mut Compiler, type_name: &str) {
    let op = match type_name {
        "i8" => Some(OpCode::CastInt(IntegerWidth::I8)),
        "i16" => Some(OpCode::CastInt(IntegerWidth::I16)),
        "i32" => Some(OpCode::CastInt(IntegerWidth::I32)),
        "i64" | "isize" => Some(OpCode::CastInt(IntegerWidth::I64)),
        "u8" => Some(OpCode::CastInt(IntegerWidth::U8)),
        "u16" => Some(OpCode::CastInt(IntegerWidth::U16)),
        "u32" => Some(OpCode::CastInt(IntegerWidth::U32)),
        "u64" | "usize" => Some(OpCode::CastInt(IntegerWidth::U64)),
        "f32" => Some(OpCode::CastFloat(FloatWidth::F32)),
        "f64" => Some(OpCode::CastFloat(FloatWidth::F64)),
        _ => None,
    };
    if let Some(o) = op {
        compiler.emit(o);
    }
}

fn is_builtin_path(path: &[String]) -> bool {
    let segs: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
    let module = segs.first().copied().unwrap_or("");
    // Handle std::module::function paths (3+ segments starting with "std")
    let effective_module = if segs.len() >= 3 && module == "std" {
        segs.get(1).copied().unwrap_or("")
    } else {
        module
    };
    matches!(
        segs.as_slice(),
        // math
        ["math", "sqrt"]
            | ["math", "abs"]
            | ["math", "sin"]
            | ["math", "cos"]
            | ["math", "tan"]
            | ["math", "asin"]
            | ["math", "acos"]
            | ["math", "atan"]
            | ["math", "pow"]
            | ["math", "floor"]
            | ["math", "ceil"]
            | ["math", "round"]
            | ["math", "min"]
            | ["math", "max"]
            | ["math", "log"]
            | ["math", "log2"]
            | ["math", "log10"]
            | ["math", "gcd"]
            | ["math", "lcm"]
            // json
            | ["json", "parse"]
            | ["json", "to_string"]
            | ["json", "serialize"]
            | ["json", "deserialize"]
            | ["json", "to_string_pretty"]
            | ["json", "from_str"]
            | ["json", "from_struct"]
            // constructors
            | ["String", "from"]
            | ["HashMap", "new"]
            | ["HashSet", "new"]
            | ["BinaryHeap", "new"]
            | ["VecDeque", "new"]
            | ["ListNode", "new"]
            | ["TreeNode", "new"]
            | ["char", "from_code"]
            | ["int", "parse"]
            | ["float", "parse"]
    ) || matches!(
        effective_module,
        "fs" | "env" | "process" | "regex" | "net" | "time" | "rand" | "http"
    ) || segs.as_slice() == ["std", "env", "args"]
}
