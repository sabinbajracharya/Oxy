//! Cranelift JIT compiler backend.
//!
//! Pipeline: AST → TypeChecker → ir_gen (AST → Register IR + CFG) → codegen (IR → CLIF) → native.
//! Bytecode (OpCode/Chunk) has been retired. No operand stack.

#![allow(dead_code)]

// ── Cranelift backend (native only) ────────────────────────────────────
#[cfg(not(target_arch = "wasm32"))]
mod codegen;
// ── Shared register-IR + runtime (compiled on all targets) ─────────────
mod context;
pub(crate) mod ffi;
pub(crate) mod ir;
pub(crate) mod ir_gen;
pub(crate) mod ir_snapshot;
pub(crate) mod runtime;

pub(crate) use context::{ClosureRuntimeMeta, JitContext, JitTables};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use ffi::register_ffi_symbols;

#[cfg(not(target_arch = "wasm32"))]
use crate::vm::VmResult;
#[cfg(not(target_arch = "wasm32"))]
use cranelift_codegen::ir::types;
#[cfg(not(target_arch = "wasm32"))]
use cranelift_codegen::settings::{self, Configurable};
#[cfg(not(target_arch = "wasm32"))]
use cranelift_frontend::FunctionBuilderContext;
#[cfg(not(target_arch = "wasm32"))]
use cranelift_jit::{JITBuilder, JITModule};
use std::collections::HashMap;
use std::path::PathBuf;

/// Compiled native function pointer type.
pub(crate) type JitFn = extern "C" fn(*mut JitContext) -> u64;

/// FFI function declarations with their parameter types and return type.
#[cfg(not(target_arch = "wasm32"))]
type FfiDecl = (&'static str, &'static [types::Type], Option<types::Type>);

#[cfg(not(target_arch = "wasm32"))]
fn ffi_decls() -> Vec<FfiDecl> {
    vec![
        ("oxy_set_result_i64", &[types::I64, types::I64], None),
        ("oxy_push_unit", &[types::I64], None),
        ("oxy_push_bool", &[types::I64, types::I8], None),
        ("oxy_push_int", &[types::I64, types::I64], None),
        ("oxy_push_float", &[types::I64, types::F64], None),
        ("oxy_push_char", &[types::I64, types::I32], None),
        (
            "oxy_push_string",
            &[types::I64, types::I64, types::I64],
            None,
        ),
        ("oxy_pop", &[types::I64], None),
        ("oxy_dup", &[types::I64], None),
        ("oxy_load_local", &[types::I64, types::I64], None),
        ("oxy_load_local_raw", &[types::I64, types::I64], None),
        (
            "oxy_read_local_i64",
            &[types::I64, types::I64],
            Some(types::I64),
        ),
        ("oxy_store_local", &[types::I64, types::I64], None),
        ("oxy_store_local_raw", &[types::I64, types::I64], None),
        ("oxy_make_cell", &[types::I64, types::I64], None),
        ("oxy_print_val", &[types::I64, types::I64], None),
        ("oxy_println_val", &[types::I64, types::I64], None),
        ("oxy_add", &[types::I64], None),
        ("oxy_sub", &[types::I64], None),
        ("oxy_mul", &[types::I64], None),
        ("oxy_div", &[types::I64], None),
        ("oxy_mod", &[types::I64], None),
        ("oxy_eq", &[types::I64], None),
        ("oxy_neq", &[types::I64], None),
        ("oxy_lt", &[types::I64], None),
        ("oxy_gt", &[types::I64], None),
        ("oxy_le", &[types::I64], None),
        ("oxy_ge", &[types::I64], None),
        ("oxy_and", &[types::I64], None),
        ("oxy_or", &[types::I64], None),
        ("oxy_bitand", &[types::I64], None),
        ("oxy_bitor", &[types::I64], None),
        ("oxy_bitxor", &[types::I64], None),
        ("oxy_shl", &[types::I64], None),
        ("oxy_shr", &[types::I64], None),
        ("oxy_neg", &[types::I64], None),
        ("oxy_not", &[types::I64], None),
        ("oxy_bitnot", &[types::I64], None),
        ("oxy_is_falsy", &[types::I64], Some(types::I8)),
        ("oxy_is_truthy", &[types::I64], Some(types::I8)),
        (
            "oxy_push_named_fn",
            &[types::I64, types::I64, types::I64],
            None,
        ),
        (
            "oxy_push_closure",
            &[types::I64, types::I64, types::I64, types::I64],
            None,
        ),
        (
            "oxy_push_async_block",
            &[types::I64, types::I64, types::I64, types::I64],
            None,
        ),
        ("oxy_call_closure", &[types::I64, types::I64], None),
        ("oxy_return", &[types::I64], None),
        ("oxy_error_discriminant", &[types::I64], Some(types::I64)),
        ("oxy_panic", &[types::I64], None),
        ("oxy_make_array", &[types::I64, types::I64], None),
        ("oxy_make_fixed_array", &[types::I64, types::I64], None),
        ("oxy_make_tuple", &[types::I64, types::I64], None),
        ("oxy_make_iter", &[types::I64], None),
        ("oxy_make_repeat", &[types::I64], None),
        ("oxy_iter_len", &[types::I64], None),
        (
            "oxy_iter_next",
            &[types::I64, types::I64, types::I64],
            Some(types::I64),
        ),
        (
            "oxy_iter_next_destructure",
            &[types::I64, types::I64],
            Some(types::I64),
        ),
        ("oxy_vec_index", &[types::I64], None),
        ("oxy_vec_index_store", &[types::I64], None),
        ("oxy_make_range", &[types::I64, types::I64], None),
        ("oxy_to_string", &[types::I64], None),
        ("oxy_fstring_concat", &[types::I64, types::I64], None),
        ("oxy_format", &[types::I64, types::I64], None),
        ("oxy_dbg", &[types::I64, types::I64], None),
        (
            "oxy_field_access",
            &[types::I64, types::I64, types::I64],
            None,
        ),
        (
            "oxy_field_store",
            &[types::I64, types::I64, types::I64],
            None,
        ),
        (
            "oxy_method_call",
            &[types::I64, types::I64, types::I64, types::I64],
            None,
        ),
        ("oxy_try_pop", &[types::I64], None),
        ("oxy_cast_int", &[types::I64], None),
        ("oxy_cast_byte", &[types::I64], None),
        ("oxy_cast_float", &[types::I64], None),
        ("oxy_cast_to_char", &[types::I64], None),
        ("oxy_bind_ident", &[types::I64, types::I64], None),
        ("oxy_enum_data_get", &[types::I64, types::I64], None),
        (
            "oxy_enum_variant_equal",
            &[types::I64, types::I64, types::I64, types::I64, types::I64],
            None,
        ),
        (
            "oxy_make_enum_variant",
            &[
                types::I64,
                types::I64,
                types::I64,
                types::I64,
                types::I64,
                types::I64,
            ],
            None,
        ),
        (
            "oxy_path_call_builtin",
            &[types::I64, types::I64, types::I64, types::I64],
            None,
        ),
        (
            "oxy_struct_init",
            &[
                types::I64,
                types::I64,
                types::I64,
                types::I64,
                types::I64,
                types::I64,
            ],
            None,
        ),
        (
            "oxy_const_enum_variant",
            &[types::I64, types::I64, types::I64, types::I64, types::I64],
            None,
        ),
        (
            "oxy_module_const",
            &[types::I64, types::I64, types::I64],
            None,
        ),
        (
            "oxy_struct_update",
            &[types::I64, types::I64, types::I64, types::I64],
            None,
        ),
        ("oxy_display_arg", &[types::I64], None),
        ("oxy_await_ffi", &[types::I64], None),
        ("oxy_spawn_ffi", &[types::I64], None),
        ("oxy_sleep_ffi", &[types::I64], None),
        ("oxy_select_ffi", &[types::I64, types::I64], None),
    ]
}

// ── Module resolution ────────────────────────────────────────────────────

/// Resolve file-based `mod <name>;` declarations by loading and parsing the
/// referenced source files. Mutates `program` in place, filling in `body` for
/// each unresolved module.
///
/// Resolution order (mirrors the old `Compiler::load_module_file`):
/// 1. Externs map (caller-supplied, like rustc `--extern`)
/// 2. `<source_dir>/<name>.ox`
/// 3. `<source_dir>/<name>/mod.ox`
///
/// Recursively resolves modules inside loaded files.
pub(crate) fn resolve_modules(
    items: &mut [crate::ast::Item],
    source_dir: Option<&str>,
    externs: &HashMap<String, PathBuf>,
) -> Result<(), String> {
    for item in items.iter_mut() {
        if let crate::ast::Item::Module(m) = item {
            if m.body.is_some() {
                // Already resolved (inline module or already loaded).
                // Recurse into the body.
                if let Some(ref mut body) = m.body {
                    resolve_modules(body, source_dir, externs)?;
                }
            } else {
                // File-based module: load and parse.
                let source = load_module_file(&m.name, m.span, source_dir, externs)?;
                let program = crate::parser::parse(&source)
                    .map_err(|e| format!("parse module `{}`: {e}", m.name))?;
                let mut body = program.items;
                resolve_modules(&mut body, source_dir, externs)?;
                m.body = Some(body);
            }
        }
    }
    Ok(())
}

/// Try to find and read a module file.
fn load_module_file(
    name: &str,
    _span: crate::lexer::Span,
    source_dir: Option<&str>,
    externs: &HashMap<String, PathBuf>,
) -> Result<String, String> {
    // 1. Externs
    if let Some(extern_path) = externs.get(name) {
        return std::fs::read_to_string(extern_path).map_err(|e| {
            format!(
                "could not read extern module `{name}` from '{}': {e}",
                extern_path.display()
            )
        });
    }

    // 2. Sibling file, 3. Sibling mod directory
    let base = source_dir.unwrap_or(".");
    let path1 = format!("{base}/{name}.ox");
    let path2 = format!("{base}/{name}/mod.ox");

    if let Ok(source) = std::fs::read_to_string(&path1) {
        return Ok(source);
    }
    if let Ok(source) = std::fs::read_to_string(&path2) {
        return Ok(source);
    }

    Err(format!(
        "could not find module `{name}`: tried '{path1}' and '{path2}' \
         (pass --extern {name}=<path> if it's an external dependency)"
    ))
}

// ── Derive macro expansion ──────────────────────────────────────────────

/// Expand `#[derive(Default)]` into synthetic `impl` blocks with a
/// `default()` constructor. Call this **before** type checking so both
/// the type checker and ir_gen see the generated functions.
///
/// Skips structs that already have a manual `impl Default for T` block
/// (explicit override takes precedence).
pub(crate) fn expand_derives(program: &mut crate::ast::Program) {
    // Collect names of types that have a manual Default impl.
    let manual_defaults: std::collections::HashSet<String> = program
        .items
        .iter()
        .filter_map(|item| match item {
            crate::ast::Item::ImplTrait(imp) if imp.trait_name == "Default" => {
                Some(imp.type_name.clone())
            }
            _ => None,
        })
        .collect();

    let mut new_impls: Vec<crate::ast::Item> = Vec::new();
    for item in &program.items {
        if let crate::ast::Item::Struct(s) = item {
            if has_derive(&s.attributes, "Default") && !manual_defaults.contains(&s.name) {
                if let Some(imp) = make_default_impl(s) {
                    new_impls.push(crate::ast::Item::Impl(imp));
                }
            }
        }
    }
    program.items.extend(new_impls);
}

/// Lower a (resolved, type-checked) program to register IR and render the
/// canonical IR disassembly — the same format the IR snapshot tests use.
/// Backs `oxy --dump-bytecode` / `tug build`. Lowering only; native codegen is
/// validated separately by the caller.
pub(crate) fn dump_ir(program: &crate::ast::Program) -> String {
    let mut ir = ir_gen::IrGen::new();
    ir.gen_program(program);
    ir_snapshot::serialize_program(&ir.functions)
}

fn has_derive(attrs: &[crate::ast::Attribute], name: &str) -> bool {
    attrs
        .iter()
        .any(|a| a.name == "derive" && a.args.iter().any(|arg| arg == name))
}

/// Build a synthetic `impl S { fn default() -> Self { … } }` block for a struct.
fn make_default_impl(s: &crate::ast::StructDef) -> Option<crate::ast::ImplBlock> {
    let fields = match &s.kind {
        crate::ast::StructKind::Named(fields) => fields,
        _ => return None, // tuple / unit structs: skip
    };
    let span = s.span;
    let mut field_inits: Vec<(String, crate::ast::Expr)> = Vec::new();
    for field in fields {
        let default_val = default_value_for_type(field.type_ann.name(), span);
        field_inits.push((field.name.clone(), default_val));
    }
    let body = crate::ast::Block {
        stmts: vec![crate::ast::Stmt::Expr {
            expr: crate::ast::Expr::StructInit {
                name: s.name.clone(),
                fields: field_inits,
                base: None,
                span,
            },
            has_semicolon: false,
        }],
        span,
    };
    let fn_def = crate::ast::FnDef {
        name: "default".to_string(),
        is_async: false,
        generic_params: vec![],
        params: vec![],
        return_type: Some(crate::ast::TypeAnnotation::Named {
            name: "Self".to_string(),
            generic_args: vec![],
            span,
        }),
        body,
        attributes: vec![],
        visibility: crate::ast::Visibility::Private,
        span,
    };
    Some(crate::ast::ImplBlock {
        type_name: s.name.clone(),
        methods: vec![fn_def],
        span,
    })
}

/// Pick a zero-like default expression for a type name.
fn default_value_for_type(type_name: &str, span: crate::lexer::Span) -> crate::ast::Expr {
    match type_name {
        "int" => crate::ast::Expr::IntLiteral(0, crate::lexer::IntegerSuffix::None, span),
        "float" => crate::ast::Expr::FloatLiteral(0.0, crate::lexer::FloatSuffix::None, span),
        "byte" => crate::ast::Expr::IntLiteral(0, crate::lexer::IntegerSuffix::None, span),
        "bool" => crate::ast::Expr::BoolLiteral(false, span),
        "char" => crate::ast::Expr::CharLiteral('\0', span),
        "String" => crate::ast::Expr::StringLiteral(String::new(), span),
        _ => crate::ast::Expr::IntLiteral(0, crate::lexer::IntegerSuffix::None, span),
    }
}

// ── JitEngine ────────────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct JitEngine {
    /// Function name → JIT fn pointer.
    pub(crate) functions: HashMap<String, *const u8>,
    /// Entry point name.
    entry_name: String,
    /// Local slot count for the entry function (must match codegen's spill slot base).
    pub(crate) local_count: usize,
    /// Per-function local slot counts (name → local_count).
    fn_local_counts: HashMap<String, usize>,
    /// Compilation output tables (fn pointers, local counts, name→index, closure meta).
    /// Owned here, borrowed via `*const JitTables` on each JitContext.
    pub(crate) tables: context::JitTables,
}

#[cfg(not(target_arch = "wasm32"))]
impl JitEngine {
    /// Build a JIT engine from a typed AST program.
    pub fn compile(program: &crate::ast::Program) -> Result<Self, String> {
        // Reset the async scheduler so tasks from previous compilations
        // don't leak into this run (the scheduler is a OnceLock singleton).
        ffi::reset_runtime_state();

        // 1. Generate register IR + CFG
        let mut ir = ir_gen::IrGen::new();
        ir.gen_program(program);
        let functions: Vec<ir::IrFunction> = std::mem::take(&mut ir.functions);

        // 1a. Collect closure metadata (will be stored on JitTables).
        let closure_meta: Vec<context::ClosureRuntimeMeta> = ir
            .closure_meta
            .drain(..)
            .map(
                |(param_names, captured, is_async)| context::ClosureRuntimeMeta {
                    param_names,
                    captured,
                    is_async,
                },
            )
            .collect();

        // 2. Detect ISA, build JIT module
        let isa_builder = cranelift_native::builder().map_err(|e| format!("host ISA: {e}"))?;
        let mut flag_builder = settings::builder();
        flag_builder
            .set("opt_level", "speed")
            .map_err(|e| format!("opt_level: {e}"))?;
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| format!("ISA: {e}"))?;

        let mut jit_builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        register_ffi_symbols(&mut jit_builder);
        let mut module = JITModule::new(jit_builder);
        let mut fn_ctx = FunctionBuilderContext::new();

        // 3. Set up codegen with FFI declarations
        let mut cg = codegen::Codegen::new(&mut module, &mut fn_ctx);
        for (name, params, ret) in ffi_decls() {
            cg.declare_ffi(name, params.to_vec(), ret);
        }

        // 4. Dump IR if OXY_VM_TRACE is set.
        if std::env::var("OXY_VM_TRACE").is_ok() {
            for f in &functions {
                eprint!("{}", f.dump());
            }
        }

        // 5. Extract main's local_count before functions is moved.
        let main_local_count = functions
            .iter()
            .find(|f| f.name == "main")
            .map(|f| f.local_count)
            .unwrap_or(8);

        // 6. Compile IR → native
        cg.compile(functions)?;

        // 4a. Build JitTables from codegen output (replaces global OnceLock tables).
        let tables = context::JitTables {
            fn_table: cg.fn_ptrs.iter().map(|(k, v)| (*k, *v as usize)).collect(),
            fn_local_counts: cg.fn_local_counts.clone(),
            name_to_index: cg.fn_names.clone(),
            closure_meta,
        };

        // 5. Build engine
        let entry_name = "main".to_string();

        // Build per-function local count map (name → local_count)
        let fn_local_counts: HashMap<String, usize> = cg
            .fn_names
            .iter()
            .map(|(name, idx)| {
                (
                    name.clone(),
                    cg.fn_local_counts.get(idx).copied().unwrap_or(8),
                )
            })
            .collect();

        Ok(Self {
            functions: std::mem::take(&mut cg.fn_names)
                .into_iter()
                .map(|(name, idx)| (name, cg.fn_ptrs[&idx]))
                .collect(),
            entry_name,
            local_count: main_local_count,
            fn_local_counts,
            tables,
        })
    }

    pub(crate) fn get_fn_ptr(&self, name: &str) -> Option<*const u8> {
        self.functions.get(name).copied()
    }

    pub(crate) fn entry_fn_ptr(&self) -> Option<*const u8> {
        self.functions.get(&self.entry_name).copied()
    }
}

// ── JitVm ──────────────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct JitVm {
    pub(crate) engine: JitEngine,
    pub(crate) output: Option<std::rc::Rc<std::cell::RefCell<Vec<String>>>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl JitVm {
    /// Compile an Oxy program and prepare to run it.
    pub fn compile(source: &str) -> Result<Self, String> {
        Self::compile_with_options(source, None, HashMap::new())
    }

    /// Compile with module resolution (source path for sibling files, externs map).
    pub fn compile_with_options(
        source: &str,
        source_path: Option<&str>,
        externs: HashMap<String, PathBuf>,
    ) -> Result<Self, String> {
        let source_dir = source_path.and_then(|p| {
            std::path::Path::new(p)
                .parent()
                .and_then(|parent| parent.to_str())
        });
        let mut program = crate::parser::parse(source).map_err(|e| format!("parse: {e}"))?;
        resolve_modules(&mut program.items, source_dir, &externs)?;
        expand_derives(&mut program);
        crate::type_checker::TypeChecker::new()
            .check_program(&program)
            .map_err(|e| format!("type check: {e}"))?;
        let engine = JitEngine::compile(&program)?;
        Ok(Self {
            engine,
            output: None,
        })
    }

    pub fn with_captured_output(&mut self) {
        self.output = Some(std::rc::Rc::new(std::cell::RefCell::new(Vec::new())));
    }

    pub fn captured_output(&self) -> Vec<String> {
        self.output
            .as_ref()
            .map(|rc| rc.borrow().clone())
            .unwrap_or_default()
    }

    pub fn run(&mut self) -> VmResult {
        let local_count = self.engine.local_count;
        match self.engine.entry_fn_ptr() {
            Some(ptr) => self.call_fn(ptr, local_count),
            None => VmResult::Error("no `main` function defined".to_string()),
        }
    }

    pub fn run_function(&mut self, name: &str) -> VmResult {
        let local_count = self
            .engine
            .fn_local_counts
            .get(name)
            .copied()
            .unwrap_or(self.engine.local_count);
        match self.engine.get_fn_ptr(name) {
            Some(ptr) => self.call_fn(ptr, local_count),
            None => VmResult::Error(format!("function not found: {name}")),
        }
    }

    fn call_fn(&self, ptr: *const u8, local_count: usize) -> VmResult {
        let mut ctx = context::JitContext::new(local_count);
        ctx.result = crate::types::Value::Unit;
        ctx.tables = &self.engine.tables as *const context::JitTables;

        if let Some(ref output_rc) = self.output {
            ctx.output = output_rc as *const _;
        }

        let fn_ptr: extern "C" fn(*mut context::JitContext) -> u64 =
            unsafe { std::mem::transmute(ptr) };
        let disc = fn_ptr(&mut ctx as *mut context::JitContext);

        match disc {
            0 => VmResult::Value(ctx.result.clone()),
            2 => {
                // set_error with empty message signals ? propagation.
                // Return ctx.result which holds the Err/None value.
                if ctx.error_len == 1 && ctx.error_msg[0] == 0 {
                    VmResult::Value(ctx.result.clone())
                } else {
                    let msg = String::from_utf8_lossy(&ctx.error_msg[..ctx.error_len.min(1024)])
                        .into_owned();
                    VmResult::Error(msg)
                }
            }
            other => VmResult::Error(format!("unexpected discriminant {other}")),
        }
    }
}
