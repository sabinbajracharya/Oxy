// Phase 1: engine methods will be used when the translator is added.
#![allow(dead_code)]
//! Cranelift JIT compiler backend.
//!
//! Translates Oxy bytecode (OpCode) to native machine code via Cranelift,
//! replacing the stack-based VM interpreter loop.

mod context;
mod ffi;

pub(crate) use context::JitContext;
pub(crate) use ffi::register_ffi_symbols;

use crate::vm::Chunk;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_native;
use std::collections::HashMap;

/// Compiled native function pointer type.
/// All JIT-compiled Oxy functions take a single `*mut JitContext` argument
/// and return a `u64` discriminant: 0=Done, 1=Yielded, 2=Error.
pub(crate) type JitFn = extern "C" fn(*mut JitContext) -> u64;

/// The Cranelift JIT compilation engine.
pub(crate) struct JitEngine {
    /// The JIT module holding all compiled functions.
    module: JITModule,
    /// Bytecode instruction pointer → finalized native function pointer.
    fn_ptrs: HashMap<usize, *const u8>,
    /// Entry point IP for the main function.
    entry_point: usize,
}

impl JitEngine {
    /// Build a JIT engine from a compiled bytecode chunk.
    ///
    /// Pre-scans the chunk to find all function entry points, compiles each
    /// function body to native code, and finalizes all definitions.
    pub fn new(chunk: &Chunk) -> Result<Self, String> {
        // Detect host ISA
        let isa_builder =
            cranelift_native::builder().map_err(|e| format!("host ISA detection failed: {e}"))?;
        let mut flag_builder = settings::builder();
        // Enable best-speed optimizations for JIT code
        flag_builder
            .set("opt_level", "speed")
            .map_err(|e| format!("failed to set opt_level: {e}"))?;
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| format!("ISA build failed: {e}"))?;

        // Set up JIT builder with native ISA
        let mut jit_builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        // Register all FFI callable symbols (Rust → JIT bridge)
        register_ffi_symbols(&mut jit_builder);

        let module = JITModule::new(jit_builder);

        // Register all function entry points from the chunk
        let fn_entries = Self::collect_function_entries(chunk);

        // Placeholder: compilation of individual functions will be added in Phase 2.
        // For now, we compile an empty main stub so the engine can be constructed.
        let _fn_entries = fn_entries;

        Ok(Self {
            module,
            fn_ptrs: HashMap::new(),
            entry_point: chunk.entry_point,
        })
    }

    /// Get a native function pointer by its bytecode entry IP.
    #[allow(dead_code)]
    pub(crate) fn get_fn_ptr(&self, ip: usize) -> Option<*const u8> {
        self.fn_ptrs.get(&ip).copied()
    }

    /// Get the entry point IP.
    #[allow(dead_code)]
    pub(crate) fn entry_point(&self) -> usize {
        self.entry_point
    }

    /// Collect all function entry IPs from a chunk.
    ///
    /// Returns a Vec of (entry_ip, frame_size) pairs, sorted by IP for
    /// deterministic compilation order.
    fn collect_function_entries(chunk: &Chunk) -> Vec<(usize, usize)> {
        let mut entries: Vec<(usize, usize)> = chunk
            .fn_frame_sizes
            .iter()
            .map(|(&ip, &size)| (ip, size))
            .collect();
        // Ensure the main entry point is included
        let main_ip = chunk.entry_point;
        if !entries.iter().any(|&(ip, _)| ip == main_ip) {
            entries.push((main_ip, chunk.local_count));
        }
        entries.sort_by_key(|&(ip, _)| ip);
        entries
    }
}
