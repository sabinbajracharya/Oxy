//! Cranelift JIT compiler backend.
//!
//! Translates Oxy bytecode (OpCode) to native machine code via Cranelift,
//! replacing the stack-based VM interpreter loop.

// FIXME: remove when JIT is wired into the execution path (Phase 6)
#![allow(dead_code)]

mod context;
mod ffi;
mod translator;

pub(crate) use context::JitContext;
pub(crate) use ffi::{register_ffi_symbols, set_closure_meta, set_fn_table};

use crate::vm::Chunk;
use cranelift_codegen::ir::types;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_jit::{JITBuilder, JITModule};
use std::collections::HashMap;

/// Compiled native function pointer type.
pub(crate) type JitFn = extern "C" fn(*mut JitContext) -> u64;

/// The Cranelift JIT compilation engine.
pub(crate) struct JitEngine {
    /// Bytecode instruction pointer → finalized native function pointer.
    fn_ptrs: HashMap<usize, *const u8>,
    /// Entry point IP for the main function.
    entry_point: usize,
}

/// FFI function declarations with their parameter types and return type.
/// (name, param_types, return_type)
type FfiDecl = (&'static str, &'static [types::Type], Option<types::Type>);

fn ffi_decls() -> Vec<FfiDecl> {
    vec![
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
        ("oxy_store_local", &[types::I64, types::I64], None),
        ("oxy_make_cell", &[types::I64, types::I64], None),
        ("oxy_print_val", &[types::I64], None),
        ("oxy_println_val", &[types::I64], None),
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
        ("oxy_call", &[types::I64, types::I64, types::I64], None),
        (
            "oxy_push_closure",
            &[types::I64, types::I64, types::I64, types::I64, types::I8],
            None,
        ),
        (
            "oxy_push_async_block",
            &[types::I64, types::I64, types::I64],
            None,
        ),
        ("oxy_call_closure", &[types::I64, types::I64], None),
        ("oxy_return", &[types::I64], None),
        ("oxy_panic", &[types::I64], None),
        ("oxy_make_array", &[types::I64, types::I64], None),
        ("oxy_make_fixed_array", &[types::I64, types::I64], None),
        ("oxy_make_tuple", &[types::I64, types::I64], None),
        ("oxy_make_iter", &[types::I64], None),
        ("oxy_iter_len", &[types::I64], None),
        ("oxy_vec_index", &[types::I64], None),
        ("oxy_vec_index_store", &[types::I64], None),
        ("oxy_make_range", &[types::I64], None),
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
            "oxy_path_call_builtin",
            &[types::I64, types::I64, types::I64, types::I64],
            None,
        ),
        ("oxy_display_arg", &[types::I64], None),
        ("oxy_await_ffi", &[types::I64], Some(types::I64)),
        ("oxy_spawn_ffi", &[types::I64], None),
        ("oxy_sleep_ffi", &[types::I64], Some(types::I64)),
        (
            "oxy_select_ffi",
            &[types::I64, types::I64],
            Some(types::I64),
        ),
    ]
}

impl JitEngine {
    /// Build a JIT engine from a compiled bytecode chunk.
    pub fn new(chunk: &Chunk) -> Result<Self, String> {
        // Detect host ISA
        let isa_builder =
            cranelift_native::builder().map_err(|e| format!("host ISA detection failed: {e}"))?;
        let mut flag_builder = settings::builder();
        flag_builder
            .set("opt_level", "speed")
            .map_err(|e| format!("failed to set opt_level: {e}"))?;
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| format!("ISA build failed: {e}"))?;

        let mut jit_builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        register_ffi_symbols(&mut jit_builder);

        let mut module = JITModule::new(jit_builder);
        let mut fn_ctx = FunctionBuilderContext::new();

        // Set up translator and declare all FFI imports
        let mut translator = translator::Translator::new(chunk, &mut module, &mut fn_ctx);
        for (name, params, ret) in ffi_decls() {
            translator.declare_ffi(name, params.to_vec(), ret);
        }

        // Compile all functions
        let fn_ptrs = translator.compile_all();

        // Store function pointer table and closure metadata for FFI access
        set_fn_table(fn_ptrs.clone());
        set_closure_meta(chunk.closure_meta.clone());

        Ok(Self {
            fn_ptrs,
            entry_point: chunk.entry_point,
        })
    }

    /// Get a native function pointer by its bytecode entry IP.
    pub(crate) fn get_fn_ptr(&self, ip: usize) -> Option<*const u8> {
        self.fn_ptrs.get(&ip).copied()
    }

    /// Get the entry point IP.
    pub(crate) fn entry_point(&self) -> usize {
        self.entry_point
    }
}
