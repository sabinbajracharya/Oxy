//! Compiler: walks the Oxy AST and emits stack-based bytecode for the VM.
//!
//! The compiler is single-pass. It resolves local variable names to stack
//! slot indices and emits [`OpCode`]s into a [`Chunk`]. Forward jumps
//! (for `if`, `while`, `loop`) are backpatched after the target is known.
//!
//! # Module structure
//! ```text
//! mod.rs  ── struct Compiler, compile() pipeline, item/module compilation
//!   ├── sym_table.rs     SymTable struct (pub(crate) use'd here)
//!   ├── loop_context.rs  LoopContext struct (pub(crate) use'd here)
//!   ├── helpers.rs       free functions (pub(crate) use'd here)
//!   ├── visibility.rs    impl Compiler { is_visible, check_path_visible_with_leaf, ... }
//!   └── expr.rs          impl Compiler { compile_expr, compile_stmt, ... }
//! ```
//!
//! All Compiler fields are `pub(crate)` so submodule impl blocks can access them.

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::errors::FerriError;
use crate::lexer::{FloatSuffix, IntegerSuffix};
use crate::types::IntegerWidth;
use crate::vm::{Chunk, OpCode};

pub(crate) use loop_context::LoopContext;
pub(crate) use sym_table::SymTable;

/// The Oxy bytecode compiler.
pub struct Compiler {
    /// The output code buffer.
    pub(crate) code: Vec<OpCode>,
    /// Current scope's symbol table.
    pub(crate) sym: SymTable,
    /// Function entry points: name → instruction index.
    pub(crate) functions: HashMap<String, usize>,
    /// Stack of enclosing loop contexts (for break/continue).
    pub(crate) loop_stack: Vec<LoopContext>,
    /// Closure metadata: (param_names, body_expr, captured_vars_with_slots_and_mutability).
    pub(crate) closure_meta: Vec<(Vec<String>, crate::ast::Expr, Vec<(String, usize, bool)>)>,
    /// Snapshot of main's local variable names (for Eval env reconstruction).
    pub(crate) main_local_names: Vec<String>,
    /// Registered struct definitions.
    pub(crate) struct_defs: HashMap<String, StructDef>,
    /// Registered enum definitions.
    pub(crate) enum_defs: HashMap<String, EnumDef>,
    /// Impl methods: type_name → method definitions.
    pub(crate) impl_methods: HashMap<String, Vec<FnDef>>,
    /// Compiled method entry points: (type_name, method_name) → instruction index.
    pub(crate) method_ips: HashMap<(String, String), usize>,
    /// Directory of the source file (for resolving file-based modules).
    pub(crate) source_dir: Option<std::path::PathBuf>,
    /// Use aliases: alias_name → qualified_name (e.g., "add" → "math::add").
    pub(crate) use_aliases: HashMap<String, String>,
    /// Const/static values: name → value (inlined at reference sites).
    pub(crate) const_values: HashMap<String, crate::types::Value>,
    /// Function metadata for named function references: name → (params, body, return_type).
    pub(crate) fn_meta: HashMap<
        String,
        (
            Vec<crate::ast::Param>,
            Box<crate::ast::Expr>,
            Option<crate::ast::TypeAnnotation>,
        ),
    >,
    /// Generic param names for functions: function_name → generic_param_names.
    pub(crate) fn_generic_names: HashMap<String, Vec<String>>,
    /// Per-function local variable names: function entry IP → slot_names.
    pub(crate) fn_local_names: HashMap<usize, Vec<String>>,
    /// Per-function frame size (number of local slots): function entry IP → size.
    /// Used by the VM at Call time to pre-allocate the frame's locals vec.
    pub(crate) fn_frame_sizes: HashMap<usize, usize>,
    /// Mutable variables captured by closures (for targeted Cell wrapping).
    pub(crate) captured_mutable: HashSet<String>,
    /// If true, compilation fails when no `main` function exists.
    /// Set to false for test runners and library code.
    pub(crate) require_main: bool,
    /// Current impl type name (for resolving `Self` in method bodies).
    pub(crate) current_impl_type: Option<String>,
    /// Generic params of the current function being compiled (for resolving `T::method()`).
    pub(crate) current_generic_params: Vec<crate::ast::GenericParam>,
    /// Trait definitions: trait_name → methods (for default method inheritance).
    pub(crate) trait_defs: HashMap<String, Vec<FnDef>>,
    /// Trait method signatures: trait_name → method names (for resolving T::method()).
    pub(crate) trait_method_names: HashMap<String, Vec<String>>,
    /// Monomorphized function instances: mangled_name → instruction index.
    pub(crate) monomorphized_fns: HashMap<String, usize>,
    /// Type aliases: alias_name → actual_type_name (e.g., P → Point).
    pub(crate) type_aliases: HashMap<String, String>,
    /// Forward calls that need target patching: (bytecode_index, function_name).
    pub(crate) forward_calls: Vec<(usize, String)>,
    /// Module name stack for resolving `self`, `super`, `crate` in paths.
    pub(crate) module_stack: Vec<String>,
    /// Deferred use resolutions processed in post-pass: (qualified_path, reexport_name_or_empty).
    /// Empty reexport_name means glob import; non-empty means pub use re-export.
    pub(crate) deferred_globs: Vec<(String, String)>,
    /// Visibility of public items (name → visibility level).
    pub(crate) pub_vis: HashMap<String, Visibility>,
    /// Module qualified names (for checking path visibility through modules).
    pub(crate) module_names: HashSet<String>,
    /// Declared integer return width of the function currently being compiled.
    /// Used to truncate the return value at every `Return` so the declared
    /// width (e.g. `u8`, `u64`) actually matters at runtime, instead of every
    /// integer silently widening to `i64`.
    pub(crate) current_fn_return_width: Option<IntegerWidth>,
}

/// Extract the integer width from a bare type annotation, if it names one.
/// Returns `None` for non-integer or parameterized/array types.
pub(crate) fn integer_width_of(ann: &TypeAnnotation) -> Option<IntegerWidth> {
    match ann {
        TypeAnnotation::Named {
            name, generic_args, ..
        } if generic_args.is_empty() => match name.as_str() {
            "int" => Some(IntegerWidth::I64),
            "byte" => Some(IntegerWidth::U8),
            _ => None,
        },
        _ => None,
    }
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
            fn_generic_names: HashMap::new(),
            fn_local_names: HashMap::new(),
            fn_frame_sizes: HashMap::new(),
            captured_mutable: HashSet::new(),
            require_main: true,
            current_impl_type: None,
            current_generic_params: Vec::new(),
            trait_defs: HashMap::new(),
            trait_method_names: HashMap::new(),
            monomorphized_fns: HashMap::new(),
            type_aliases: HashMap::new(),
            forward_calls: Vec::new(),
            module_stack: Vec::new(),
            deferred_globs: Vec::new(),
            pub_vis: HashMap::new(),
            module_names: HashSet::new(),
            current_fn_return_width: None,
        }
    }
}

impl Compiler {
    /// Emit a `Return`, prefixed by an integer-truncation cast if the
    /// surrounding function declared an integer return type. This makes
    /// `fn f() -> u8 { ... }` actually wrap the return value to `u8`
    /// instead of silently leaking the inner `i64`.
    pub(crate) fn emit_return(&mut self) {
        if let Some(w) = self.current_fn_return_width {
            self.emit(OpCode::CastInt(w));
        }
        self.emit(OpCode::Return);
    }

    /// At a function's entry, coerce each integer-typed parameter to its
    /// declared width. Args are otherwise stored verbatim in the locals
    /// frame, so without this step `fn f(n: u32)` called with `i64(5)`
    /// would carry an `i64` value throughout the body.
    pub(crate) fn emit_param_coercions(&mut self, params: &[Param]) {
        for (i, param) in params.iter().enumerate() {
            if let Some(w) = integer_width_of(&param.type_ann) {
                self.emit(OpCode::LoadLocal(i));
                self.emit(OpCode::CastInt(w));
                self.emit(OpCode::StoreLocal(i));
            }
        }
    }

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
            pub_vis: &mut HashMap<String, Visibility>,
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
                        if f.visibility.is_pub() {
                            pub_vis.insert(name.clone(), f.visibility.clone());
                        }
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
                        if s.visibility.is_pub() {
                            pub_vis.insert(name.clone(), s.visibility.clone());
                        }
                        struct_defs.insert(name, s.clone());
                    }
                    Item::Enum(e) => {
                        let name = if prefix.is_empty() {
                            e.name.clone()
                        } else {
                            format!("{}::{}", prefix, e.name)
                        };
                        if e.visibility.is_pub() {
                            pub_vis.insert(name.clone(), e.visibility.clone());
                        }
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
                                pub_vis,
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
            &mut self.pub_vis,
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
            local_count: self.main_local_names.len(),
            entry_point,
            functions: self.functions,
            closure_meta: self.closure_meta,
            local_names: self.main_local_names,
            fn_local_names: self.fn_local_names,
            fn_frame_sizes: self.fn_frame_sizes,
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
                self.pub_vis.insert(f.name.clone(), f.visibility.clone());
                self.compile_fn_item(f, None)?;
                Ok(())
            }
            Item::Struct(s) => {
                self.struct_defs.insert(s.name.clone(), s.clone());
                self.pub_vis.insert(s.name.clone(), s.visibility.clone());
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
                self.pub_vis.insert(e.name.clone(), e.visibility.clone());
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
                // Store all method names (signatures + default methods) for generic resolution
                let mut method_names: Vec<String> = t
                    .methods
                    .iter()
                    .map(|s| s.name.clone())
                    .chain(t.default_methods.iter().map(|d| d.name.clone()))
                    .collect();
                method_names.sort();
                method_names.dedup();
                self.trait_method_names.insert(t.name.clone(), method_names);
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
                self.type_aliases
                    .insert(name.clone(), target.name().to_string());
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
            let default_val = match field.type_ann.name() {
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
            return_type: Some(TypeAnnotation::Named {
                name: "Self".to_string(),
                generic_args: Vec::new(),
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
                self.pub_vis.insert(qualified.clone(), f.visibility.clone());
            }
            self.method_ips.insert((tn.to_string(), f.name.clone()), ip);
            // If type has generic args (e.g. "Pair<i64>" or "Cell<T>"), also
            // register under the base type name so PathCall lookup with
            // `Cell::make` (no turbofish on the type) resolves to the same
            // function. Mirrors the method_ips registration below.
            if let Some(lt_pos) = tn.find('<') {
                let base_name = tn[..lt_pos].to_string();
                self.method_ips
                    .insert((base_name.clone(), f.name.clone()), ip);
                let base_qualified = format!("{}::{}", base_name, f.name);
                self.functions.insert(base_qualified.clone(), ip);
                if f.visibility.is_pub() {
                    self.pub_vis.insert(base_qualified, f.visibility.clone());
                }
            }
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
            self.fn_meta
                .insert(format!("{}::{}", tn, f.name), meta.clone());
            // Same base-name registration as above so generic-type methods
            // resolve via either `Cell::make` or `Cell<T>::make`.
            if let Some(lt_pos) = tn.find('<') {
                let base_name = &tn[..lt_pos];
                self.fn_meta
                    .insert(format!("{}::{}", base_name, f.name), meta);
            }
        }
        // Store generic param names for monomorphization
        if !f.generic_params.is_empty() {
            let generic_names: Vec<String> =
                f.generic_params.iter().map(|p| p.name.clone()).collect();
            self.fn_generic_names.insert(f.name.clone(), generic_names);
        }

        let saved_sym = self.sym.clone();
        for param in &f.params {
            if param.is_mut {
                self.sym.define_mut(&param.name);
            } else {
                self.sym.define(&param.name);
            }
        }

        // Pre-scan: find mutable variables captured by closures
        let param_names: Vec<String> = f.params.iter().map(|p| p.name.clone()).collect();
        self.captured_mutable = find_captured_mutable(&f.body, &param_names);

        // Store generic params for resolving T::method() calls in the function body
        let saved_generic_params =
            std::mem::replace(&mut self.current_generic_params, f.generic_params.clone());

        // Coerce integer params to their declared widths; track the declared
        // return width so every `Return` truncates accordingly.
        self.emit_param_coercions(&f.params);
        let saved_return_width = self.current_fn_return_width;
        self.current_fn_return_width = f.return_type.as_ref().and_then(integer_width_of);

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
        self.emit_return();
        self.current_fn_return_width = saved_return_width;

        if f.name == "main" {
            self.main_local_names = self.sym.build_slot_names();
        }
        self.fn_local_names.insert(ip, self.sym.build_slot_names());
        self.fn_frame_sizes.insert(ip, self.sym.next_slot);

        self.sym = saved_sym;
        self.current_impl_type = saved_impl_type;
        self.current_generic_params = saved_generic_params;
        Ok(())
    }

    /// Monomorphize a generic function call: compile a copy with concrete types substituted
    /// for the generic params. Returns the new function's entry IP.
    fn monomorphize_call(
        &mut self,
        fn_name: &str,
        type_args: &[String],
        err_line: usize,
        err_col: usize,
    ) -> Result<usize, FerriError> {
        // Generate mangled name for dedup and registration
        let mangled = format!("{}@{}", fn_name, type_args.join("_"));
        if let Some(&ip) = self.monomorphized_fns.get(&mangled) {
            return Ok(ip);
        }

        // Get the generic function's metadata
        let meta = self
            .fn_meta
            .get(fn_name)
            .cloned()
            .or_else(|| self.fn_meta.get(&mangled).cloned());
        let (params, body_expr, return_type) = match meta {
            Some(m) => m,
            None => {
                return Err(FerriError::Runtime {
                    message: format!("cannot monomorphize; function '{}' has no stored AST body (fn_meta not found)", fn_name),
                    line: err_line,
                    column: err_col,
                });
            }
        };

        // Build substitution map: generic_param_name → concrete_type_name
        let generic_param_names = self
            .fn_generic_names
            .get(fn_name)
            .cloned()
            .unwrap_or_default();
        if generic_param_names.len() != type_args.len() {
            return Err(FerriError::Runtime {
                message: format!(
                    "function '{}' expects {} type argument(s), but {} provided",
                    fn_name,
                    generic_param_names.len(),
                    type_args.len()
                ),
                line: err_line,
                column: err_col,
            });
        }
        let subst: Vec<(String, String)> = generic_param_names
            .iter()
            .zip(type_args.iter())
            .map(|(p, c)| (p.clone(), c.clone()))
            .collect();

        // Substitute type params in the body expression
        let mut subbed_body = (*body_expr).clone();
        substitute_type_params(&mut subbed_body, &subst);

        // Build a synthetic FnDef for compilation
        let blank_span = crate::lexer::Span {
            start: 0,
            end: 0,
            line: 0,
            column: 0,
        };
        let synth_fn = crate::ast::FnDef {
            name: mangled.clone(),
            is_async: false,
            generic_params: vec![], // no longer generic
            params: params.clone(),
            return_type: return_type.map(|rt| {
                let mut ann = rt.clone();
                for (param, concrete) in &subst {
                    match &mut ann {
                        TypeAnnotation::Named { ref mut name, .. } => {
                            *name = name.replace(param.as_str(), concrete.as_str());
                        }
                        TypeAnnotation::Array { ref mut inner, .. } => {
                            if let TypeAnnotation::Named { ref mut name, .. } = **inner {
                                *name = name.replace(param.as_str(), concrete.as_str());
                            }
                        }
                    }
                }
                ann
            }),
            body: crate::ast::Block {
                stmts: vec![crate::ast::Stmt::Expr {
                    expr: subbed_body,
                    has_semicolon: false,
                }],
                span: blank_span,
            },
            attributes: vec![],
            visibility: crate::ast::Visibility::Private,
            span: blank_span,
        };

        // Compile the synthetic function
        let ip = self.code.len();
        self.functions.insert(mangled.clone(), ip);
        self.compile_fn_item(&synth_fn, None)?;

        // Store for dedup
        self.monomorphized_fns.insert(mangled, ip);
        Ok(ip)
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
        self.module_names.insert(prefix.clone());
        if module.visibility.is_pub() {
            self.pub_vis
                .insert(prefix.clone(), module.visibility.clone());
        }
        self.module_stack.push(module.name.clone());
        self.compile_module_items(&items, &prefix)?;
        self.module_stack.pop();
        Ok(())
    }

    /// Process a `use` declaration.
    fn compile_use(&mut self, use_def: &UseDef) -> Result<(), FerriError> {
        let resolved_path = resolve_use_path(&use_def.path, &self.module_stack);
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
                        if !stripped.contains("::") && self.is_visible(qualified_name) {
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
                        if !stripped.contains("::") && self.is_visible(qualified_name) {
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
                        if !stripped.contains("::") && self.is_visible(qualified_name) {
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
    /// Pre-resolve all use statements against pre-scanned data, before function bodies
    /// are compiled. This ensures use aliases (especially globs) are available.
    fn preresolve_uses(&mut self, items: &[Item]) -> Result<(), FerriError> {
        let module_prefix = self.module_stack.join("::");
        for item in items {
            match item {
                Item::Use(u) => {
                    let resolved_path = resolve_use_path(&u.path, &self.module_stack);
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
                                    if !stripped.contains("::") && self.is_visible(qualified_name) {
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
                                    if !stripped.contains("::") && self.is_visible(qualified_name) {
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
                                    if !stripped.contains("::") && self.is_visible(qualified_name) {
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
                        if !stripped.contains("::") && self.is_visible(qualified_name) {
                            self.use_aliases
                                .insert(stripped.to_string(), qualified_name.clone());
                        }
                    }
                }
                for qualified_name in self.struct_defs.keys() {
                    if let Some(stripped) = qualified_name.strip_prefix(&format!("{}::", prefix)) {
                        if !stripped.contains("::") && self.is_visible(qualified_name) {
                            self.use_aliases
                                .insert(stripped.to_string(), qualified_name.clone());
                        }
                    }
                }
                for qualified_name in self.enum_defs.keys() {
                    if let Some(stripped) = qualified_name.strip_prefix(&format!("{}::", prefix)) {
                        if !stripped.contains("::") && self.is_visible(qualified_name) {
                            self.use_aliases
                                .insert(stripped.to_string(), qualified_name.clone());
                        }
                    }
                }
            } else {
                // pub use re-export: register qualified item under alias name
                if let Some(&ip) = self.functions.get(base_path) {
                    self.functions.insert(reexport_name.clone(), ip);
                    self.pub_vis.insert(reexport_name.clone(), Visibility::Pub);
                }
                if let Some(def) = self.struct_defs.get(base_path).cloned() {
                    self.struct_defs.insert(reexport_name.clone(), def);
                    self.pub_vis.insert(reexport_name.clone(), Visibility::Pub);
                }
                if let Some(def) = self.enum_defs.get(base_path).cloned() {
                    self.enum_defs.insert(reexport_name.clone(), def);
                    self.pub_vis.insert(reexport_name.clone(), Visibility::Pub);
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
                        self.pub_vis.insert(qualified.clone(), f.visibility.clone());
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
                        if param.is_mut {
                            self.sym.define_mut(&param.name);
                        } else {
                            self.sym.define(&param.name);
                        }
                    }
                    self.emit_param_coercions(&f.params);
                    let saved_return_width = self.current_fn_return_width;
                    self.current_fn_return_width =
                        f.return_type.as_ref().and_then(integer_width_of);
                    self.compile_block(&f.body)?;
                    self.emit_return();
                    self.current_fn_return_width = saved_return_width;
                    self.fn_local_names.insert(ip, self.sym.build_slot_names());
                    self.fn_frame_sizes.insert(ip, self.sym.next_slot);
                    self.sym = saved_sym;
                }
                Item::Struct(s) => {
                    let qualified = format!("{}::{}", prefix, s.name);
                    self.struct_defs.insert(qualified.clone(), s.clone());
                    if s.visibility.is_pub() {
                        self.pub_vis.insert(qualified.clone(), s.visibility.clone());
                    }
                }
                Item::Enum(e) => {
                    let qualified = format!("{}::{}", prefix, e.name);
                    self.enum_defs.insert(qualified.clone(), e.clone());
                    if e.visibility.is_pub() {
                        self.pub_vis.insert(qualified.clone(), e.visibility.clone());
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
                            self.pub_vis
                                .insert(mname.clone(), method.visibility.clone());
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
                        // Also register under base type name (strip type args: "Pair<i64>" → "Pair")
                        if let Some(lt_pos) = i.type_name.find('<') {
                            let base_name = i.type_name[..lt_pos].to_string();
                            self.method_ips
                                .insert((base_name.clone(), method.name.clone()), ip);
                            let base_qualified = if qualified_type.contains("::") {
                                let lt_in_qualified = qualified_type.find('<').unwrap();
                                qualified_type[..lt_in_qualified].to_string()
                            } else {
                                base_name
                            };
                            self.method_ips
                                .insert((base_qualified, method.name.clone()), ip);
                        }
                        let saved_sym = self.sym.clone();
                        for param in &method.params {
                            if param.is_mut {
                                self.sym.define_mut(&param.name);
                            } else {
                                self.sym.define(&param.name);
                            }
                        }
                        self.emit_param_coercions(&method.params);
                        let saved_return_width = self.current_fn_return_width;
                        self.current_fn_return_width =
                            method.return_type.as_ref().and_then(integer_width_of);
                        self.compile_block(&method.body)?;
                        self.emit_return();
                        self.current_fn_return_width = saved_return_width;
                        self.fn_local_names.insert(ip, self.sym.build_slot_names());
                        self.fn_frame_sizes.insert(ip, self.sym.next_slot);
                        self.sym = saved_sym;
                    }
                }
                Item::Module(m) => {
                    let nested_prefix = format!("{}::{}", prefix, m.name);
                    self.module_names.insert(nested_prefix.clone());
                    if m.visibility.is_pub() {
                        self.pub_vis
                            .insert(nested_prefix.clone(), m.visibility.clone());
                    }
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
                            self.pub_vis
                                .insert(mname.clone(), method.visibility.clone());
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
                            if param.is_mut {
                                self.sym.define_mut(&param.name);
                            } else {
                                self.sym.define(&param.name);
                            }
                        }
                        self.emit_param_coercions(&method.params);
                        let saved_return_width = self.current_fn_return_width;
                        self.current_fn_return_width =
                            method.return_type.as_ref().and_then(integer_width_of);
                        self.compile_block(&method.body)?;
                        self.emit_return();
                        self.current_fn_return_width = saved_return_width;
                        self.fn_local_names.insert(ip, self.sym.build_slot_names());
                        self.fn_frame_sizes.insert(ip, self.sym.next_slot);
                        self.sym = saved_sym;
                    }
                }
                Item::TypeAlias { name, target, .. } => {
                    let qualified = format!("{}::{}", prefix, name);
                    self.type_aliases
                        .insert(qualified, target.name().to_string());
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
}

mod expr;

mod helpers;
mod loop_context;
mod path_resolution;
mod sym_table;
mod visibility;

pub(crate) use helpers::{
    find_captured_mutable, resolve_use_path, substitute_type_params, try_eval_const,
};
