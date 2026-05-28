//! Cranelift JIT compiler backend.
//!
//! Pipeline: AST → TypeChecker → ir_gen (AST → Register IR + CFG) → codegen (IR → CLIF) → native.
//! Bytecode (OpCode/Chunk) has been retired. No operand stack.

#![allow(dead_code)]

mod codegen;
mod context;
pub(crate) mod ffi;
pub(crate) mod ir;
pub(crate) mod ir_gen;

pub(crate) use context::JitContext;
pub(crate) use ffi::register_ffi_symbols;

use crate::vm::VmResult;
use cranelift_codegen::ir::types;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_jit::{JITBuilder, JITModule};
use std::collections::HashMap;

/// Compiled native function pointer type.
pub(crate) type JitFn = extern "C" fn(*mut JitContext) -> u64;

/// FFI function declarations with their parameter types and return type.
type FfiDecl = (&'static str, &'static [types::Type], Option<types::Type>);

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
            "oxy_call",
            &[types::I64, types::I64, types::I64, types::I64],
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
            "oxy_struct_update",
            &[types::I64, types::I64, types::I64, types::I64],
            None,
        ),
        ("oxy_display_arg", &[types::I64], None),
        ("oxy_await_ffi", &[types::I64], None),
        ("oxy_spawn_ffi", &[types::I64], None),
        ("oxy_sleep_ffi", &[types::I64], None),
        ("oxy_select_ffi", &[types::I64, types::I64], None),
        (
            "oxy_make_future",
            &[types::I64, types::I64, types::I64],
            None,
        ),
    ]
}

// ── JitEngine (stub — will be wired to ir_gen+codegen) ──────────────────

pub(crate) struct JitEngine {
    /// Function name → JIT fn pointer.
    pub(crate) functions: HashMap<String, *const u8>,
    /// Entry point name.
    entry_name: String,
    /// Local slot count for the entry function (must match codegen's spill slot base).
    pub(crate) local_count: usize,
    /// Per-function local slot counts (name → local_count).
    fn_local_counts: HashMap<String, usize>,
}

/// Serialize compilation so parallel tests don't race on global fn tables.
static COMPILE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

impl JitEngine {
    /// Build a JIT engine from a typed AST program.
    pub fn compile(program: &crate::ast::Program) -> Result<Self, String> {
        let _guard = COMPILE_LOCK.lock().unwrap();

        // Reset the global scheduler so tasks from previous compilations
        // don't leak into this run (the scheduler is a OnceLock singleton).
        ffi::scheduler_lock().reset();

        // 1. Generate register IR + CFG
        let mut ir = ir_gen::IrGen::new();
        ir.gen_program(program);
        let functions: Vec<ir::IrFunction> = std::mem::take(&mut ir.functions);

        // 1a. Register closure metadata so oxy_push_closure can look up captures at runtime.
        if !ir.closure_meta.is_empty() {
            ffi::set_closure_meta(std::mem::take(&mut ir.closure_meta));
        }

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

        // 4. Extract main's local_count before functions is moved.
        let main_local_count = functions
            .iter()
            .find(|f| f.name == "main")
            .map(|f| f.local_count)
            .unwrap_or(8);

        // 4b. Compile IR → native
        cg.compile(functions)?;

        // 4a. Populate fn_table for closure/async-block dispatch.
        //     fn_index → native fn pointer (stored as usize).
        if !cg.fn_ptrs.is_empty() {
            let fn_table: HashMap<usize, *const u8> = cg.fn_ptrs.clone();
            ffi::set_fn_table(fn_table);
        }
        if !cg.fn_local_counts.is_empty() {
            ffi::set_fn_local_counts(cg.fn_local_counts.clone());
        }
        // 4b. Build closure name → fn_index mapping for runtime lookup.
        {
            let closure_indices: HashMap<String, usize> = cg.fn_names.clone();
            ffi::set_closure_fn_indices(closure_indices);
        }

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

pub(crate) struct JitVm {
    pub(crate) engine: JitEngine,
    pub(crate) output: Option<std::rc::Rc<std::cell::RefCell<Vec<String>>>>,
}

impl JitVm {
    /// Compile an Oxy program and prepare to run it.
    pub fn compile(source: &str) -> Result<Self, String> {
        let program = crate::parser::parse(source).map_err(|e| format!("parse: {e}"))?;
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
            None => VmResult::Error("no entry point".to_string()),
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

        if let Some(ref output_rc) = self.output {
            ctx.output = output_rc as *const _;
        }

        let fn_ptr: extern "C" fn(*mut context::JitContext) -> u64 =
            unsafe { std::mem::transmute(ptr) };
        let disc = fn_ptr(&mut ctx as *mut context::JitContext);

        match disc {
            0 => VmResult::Value(ctx.result.clone()),
            2 => {
                let msg =
                    String::from_utf8_lossy(&ctx.error_msg[..ctx.error_len.min(1024)]).into_owned();
                VmResult::Error(msg)
            }
            other => VmResult::Error(format!("unexpected discriminant {other}")),
        }
    }
}
