//! Cranelift JIT compiler backend.
//!
//! Pipeline: AST → TypeChecker → ir_gen (AST → Register IR + CFG) → codegen (IR → CLIF) → native.
//! Bytecode (OpCode/Chunk) has been retired. No operand stack.

#![allow(dead_code)]

mod context;
pub(crate) mod ffi;
pub(crate) mod ir;
pub(crate) mod ir_gen;

pub(crate) use context::JitContext;
pub(crate) use ffi::register_ffi_symbols;

use crate::vm::Chunk;
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
        ("oxy_push_unit", &[types::I64], None),
        ("oxy_push_bool", &[types::I64, types::I8], None),
        ("oxy_push_int", &[types::I64, types::I64], None),
        ("oxy_push_float", &[types::I64, types::F64], None),
        ("oxy_push_char", &[types::I64, types::I32], None),
        ("oxy_push_string", &[types::I64, types::I64, types::I64], None),
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
        ("oxy_push_closure", &[types::I64, types::I64, types::I64, types::I64, types::I8], None),
        ("oxy_push_async_block", &[types::I64, types::I64, types::I64], None),
        ("oxy_call_closure", &[types::I64, types::I64], None),
        ("oxy_return", &[types::I64], None),
        ("oxy_error_discriminant", &[types::I64], Some(types::I64)),
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
        ("oxy_field_access", &[types::I64, types::I64, types::I64], None),
        ("oxy_field_store", &[types::I64, types::I64, types::I64], None),
        ("oxy_method_call", &[types::I64, types::I64, types::I64, types::I64], None),
        ("oxy_try_pop", &[types::I64], None),
        ("oxy_cast_int", &[types::I64], None),
        ("oxy_cast_float", &[types::I64], None),
        ("oxy_cast_to_char", &[types::I64], None),
        ("oxy_bind_ident", &[types::I64, types::I64], None),
        ("oxy_enum_data_get", &[types::I64, types::I64], None),
        ("oxy_enum_variant_equal", &[types::I64, types::I64, types::I64, types::I64, types::I64], None),
        ("oxy_make_enum_variant", &[types::I64, types::I64, types::I64, types::I64, types::I64, types::I64], None),
        ("oxy_path_call_builtin", &[types::I64, types::I64, types::I64], None),
        ("oxy_struct_init", &[types::I64, types::I64], None),
        ("oxy_const_enum_variant", &[types::I64, types::I64], None),
        ("oxy_struct_update", &[types::I64, types::I64], None),
        ("oxy_display_arg", &[types::I64], None),
        ("oxy_await_ffi", &[types::I64], Some(types::I64)),
        ("oxy_spawn_ffi", &[types::I64], None),
        ("oxy_sleep_ffi", &[types::I64], Some(types::I64)),
        ("oxy_select_ffi", &[types::I64, types::I64], Some(types::I64)),
        ("oxy_make_future", &[types::I64, types::I64, types::I64], None),
    ]
}

// ── JitEngine (stub — will be wired to ir_gen+codegen) ──────────────────

pub(crate) struct JitEngine {
    /// Function name → JIT fn pointer.
    pub(crate) functions: HashMap<String, *const u8>,
}

impl JitEngine {
    pub fn new(_chunk: Chunk) -> Result<Self, String> {
        // Stub: bytecode path retired. Will be replaced with ir_gen→codegen.
        Err("JIT engine: bytecode path retired. Awaiting AST→IR→CLIF wiring.".to_string())
    }

    pub(crate) fn get_fn_ptr(&self, name: &str) -> Option<*const u8> {
        self.functions.get(name).copied()
    }
}

// ── JitVm (stub) ────────────────────────────────────────────────────────

pub(crate) struct JitVm {
    pub(crate) engine: JitEngine,
    output: Option<std::rc::Rc<std::cell::RefCell<Vec<String>>>>,
}

impl JitVm {
    pub fn new(chunk: Chunk) -> Result<Self, String> {
        let engine = JitEngine::new(chunk)?;
        Ok(Self { engine, output: None })
    }

    pub fn with_captured_output(chunk: Chunk) -> Result<Self, String> {
        let engine = JitEngine::new(chunk)?;
        Ok(Self { engine, output: Some(std::rc::Rc::new(std::cell::RefCell::new(Vec::new()))) })
    }

    pub fn captured_output(&self) -> Vec<String> {
        self.output.as_ref().map(|rc| rc.borrow().clone()).unwrap_or_default()
    }

    pub fn run(&mut self) -> VmResult {
        VmResult::Error("JIT path not yet wired".to_string())
    }

    pub fn run_function(&mut self, _name: &str) -> VmResult {
        VmResult::Error("JIT path not yet wired".to_string())
    }
}
