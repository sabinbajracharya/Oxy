//! Cranelift JIT compiler backend.
//!
//! Translates Oxy bytecode (OpCode) to native machine code via Cranelift,
//! replacing the stack-based VM interpreter loop.

// FIXME: remove when JIT is wired into the execution path (Phase 6)
#![allow(dead_code)]

mod context;
pub(crate) mod ffi;
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
    pub(crate) fn_ptrs: HashMap<usize, *const u8>,
    /// Function name → entry IP (for test discovery).
    pub(crate) functions: HashMap<String, usize>,
    /// Entry point IP for the main function.
    entry_point: usize,
    /// Keeps the Chunk (and all string data) alive.
    #[allow(dead_code)]
    chunk: Chunk,
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
            "oxy_enum_variant_equal",
            &[types::I64, types::I64, types::I64, types::I64, types::I64],
            None,
        ),
        (
            "oxy_path_call_builtin",
            &[types::I64, types::I64, types::I64],
            None,
        ),
        ("oxy_struct_init", &[types::I64, types::I64], None),
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
    pub fn new(chunk: Chunk) -> Result<Self, String> {
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
        let mut translator = translator::Translator::new(&chunk, &mut module, &mut fn_ctx);
        for (name, params, ret) in ffi_decls() {
            translator.declare_ffi(name, params.to_vec(), ret);
        }

        // Compile all functions
        let fn_ptrs = translator.compile_all();

        // Store function pointer table and closure metadata for FFI access
        set_fn_table(fn_ptrs.clone());
        set_closure_meta(chunk.closure_meta.clone());

        Ok(Self {
            fn_ptrs: fn_ptrs.clone(),
            functions: chunk.functions.clone(),
            entry_point: chunk.entry_point,
            chunk,
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

// ── JitVm: High-level execution wrapper ───────────────────────────────

use crate::vm::VmResult;

/// A JIT-compiled execution context that mirrors the `Vm` API.
pub(crate) struct JitVm {
    pub(crate) engine: JitEngine,
    /// Captured output buffer shared with JitContext.
    output: Option<std::rc::Rc<std::cell::RefCell<Vec<String>>>>,
}

impl JitVm {
    /// Build a JIT VM from a compiled chunk. Compiles all functions to native code.
    pub fn new(chunk: Chunk) -> Result<Self, String> {
        let engine = JitEngine::new(chunk)?;
        Ok(Self {
            engine,
            output: None,
        })
    }

    /// Build a JIT VM with captured output (for testing).
    pub fn with_captured_output(chunk: Chunk) -> Result<Self, String> {
        let engine = JitEngine::new(chunk)?;
        Ok(Self {
            engine,
            output: Some(std::rc::Rc::new(std::cell::RefCell::new(Vec::new()))),
        })
    }

    /// Return captured output lines.
    pub fn captured_output(&self) -> Vec<String> {
        self.output
            .as_ref()
            .map(|rc| rc.borrow().clone())
            .unwrap_or_default()
    }

    /// Run the main function.
    pub fn run(&mut self) -> VmResult {
        let entry_ip = self.engine.entry_point;
        self.run_function(entry_ip)
    }

    /// Run a specific function by its bytecode entry IP.
    pub fn run_function(&mut self, ip: usize) -> VmResult {
        let fn_ptr = match self.engine.get_fn_ptr(ip) {
            Some(p) => p,
            None => return VmResult::Error(format!("JIT: no function at ip={ip}")),
        };

        let mut ctx = JitContext::new(8);
        ctx.result = crate::types::Value::Unit;
        ctx.fn_table = std::ptr::null();
        ctx.fn_table_len = 0;

        // Wire up captured output
        if let Some(ref output_rc) = self.output {
            let ptr: *const std::rc::Rc<std::cell::RefCell<Vec<String>>> = output_rc;
            ctx.output = ptr;
        }

        let fn_ptr: extern "C" fn(*mut JitContext) -> u64 = unsafe { std::mem::transmute(fn_ptr) };
        let discriminant = fn_ptr(&mut ctx as *mut JitContext);

        match discriminant {
            0 => VmResult::Value(ctx.result.clone()),
            2 => {
                let msg =
                    String::from_utf8_lossy(&ctx.error_msg[..ctx.error_len.min(1024)]).into_owned();
                VmResult::Error(msg)
            }
            other => VmResult::Error(format!("JIT: unexpected discriminant {other}")),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::vm::api::{run_compiled_capturing_jit, run_compiled_jit};

    #[test]
    fn test_jit_simple_literal() {
        let result = run_compiled_jit("fn main() -> int { 42 }").unwrap();
        assert_eq!(result, crate::types::Value::I64(42));
    }

    #[test]
    fn test_jit_print_captured() {
        let (val, output) =
            run_compiled_capturing_jit("fn main() { println!(\"hello\"); println!(\"world\"); }")
                .unwrap();
        assert_eq!(val, crate::types::Value::Unit);
        assert_eq!(output, vec!["hello\n", "world\n"]);
    }

    #[test]
    fn test_jit_add_two_ints() {
        let result = run_compiled_jit("fn main() -> int { 1 + 2 }").unwrap();
        assert_eq!(result, crate::types::Value::I64(3));
    }

    #[test]
    fn test_jit_let_binding() {
        let result = run_compiled_jit("fn main() -> int { let x = 5; x }").unwrap();
        assert_eq!(result, crate::types::Value::I64(5));
    }

    #[test]
    fn test_jit_arithmetic() {
        let result = run_compiled_jit("fn main() -> int { let x = 2 + 3 * 4; x }").unwrap();
        assert_eq!(result, crate::types::Value::I64(14));
    }

    #[test]
    fn test_jit_if_true() {
        let result = run_compiled_jit("fn main() -> int { if true { 1 } else { 0 } }").unwrap();
        assert_eq!(result, crate::types::Value::I64(1));
    }

    #[test]
    fn test_jit_if_else() {
        let result =
            run_compiled_jit("fn main() -> int { let x = 5; if x > 3 { 1 } else { 0 } }").unwrap();
        assert_eq!(result, crate::types::Value::I64(1));
    }

    #[test]
    fn test_jit_path_call_builtin() {
        let result = run_compiled_jit("fn main() -> int { std::env::var(\"HOME\"); 42 }").unwrap();
        assert_eq!(result, crate::types::Value::I64(42));
    }

    #[test]
    fn test_jit_string_literal() {
        let result = run_compiled_jit("fn main() -> String { \"hello\" }").unwrap();
        assert_eq!(result, crate::types::Value::String("hello".to_string()));
    }

    #[test]
    fn test_jit_int_to_string() {
        let result = run_compiled_jit("fn main() -> String { let x = 42; x.to_string() }").unwrap();
        assert_eq!(result, crate::types::Value::String("42".to_string()));
    }

    #[test]
    fn test_jit_cast_int() {
        let result = run_compiled_jit("fn main() -> int { let x: int = 5; x }").unwrap();
        assert_eq!(result, crate::types::Value::I64(5));
    }

    #[test]
    fn test_jit_call_and_cast() {
        let result = run_compiled_jit("fn main() -> int { 100.to_string(); 42 }").unwrap();
        assert_eq!(result, crate::types::Value::I64(42));
    }

    #[test]
    fn test_jit_string_method_discard() {
        let result =
            run_compiled_jit("fn main() -> int { let s = \"hello\"; s.len(); 42 }").unwrap();
        assert_eq!(result, crate::types::Value::I64(42));
    }

    #[test]
    fn test_jit_string_len_method() {
        let result = run_compiled_jit("fn main() -> int { let s = \"hello\"; s.len() }").unwrap();
        assert_eq!(result, crate::types::Value::I64(5));
    }

    #[test]
    fn test_jit_struct_init() {
        let result = run_compiled_jit(
            "struct Point { x: int, y: int } fn main() -> int { let p = Point { x: 1, y: 2 }; p.x }",
        )
        .unwrap();
        assert_eq!(result, crate::types::Value::I64(1));
    }

    #[test]
    fn test_jit_struct_field_y() {
        let result = run_compiled_jit(
            "struct Point { x: int, y: int } fn main() -> int { let p = Point { x: 1, y: 2 }; p.y }",
        )
        .unwrap();
        assert_eq!(result, crate::types::Value::I64(2));
    }
}
