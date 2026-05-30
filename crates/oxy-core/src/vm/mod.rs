//! Shared types and modules for Oxy program execution.
//!
//! The bytecode VM (OpCode/Chunk/Vm) has been retired in favor of
//! the Cranelift JIT backend in `vm/jit/`. This module retains only
//! the public API entry points, async scheduler, built-in methods,
//! and the shared `VmResult` type.

use crate::types::Value;

// The `jit` module hosts the shared register-IR + runtime layer (ir, ir_gen,
// ir_snapshot, context, runtime, and the oxy_* FFI bodies) which compile on
// ALL targets, plus the Cranelift-specific backend (codegen, JitEngine, JitVm)
// which is gated to non-wasm inside the module. On wasm there is no Cranelift,
// so execution runs through the portable IR interpreter in `vm::interp`.
pub(crate) mod interp;
pub(crate) mod jit;
pub(crate) mod scheduler;

/// Result of VM execution.
pub enum VmResult {
    Value(Value),
    Error(String),
}

mod api;
pub mod builtins;
pub use api::*;

#[cfg(test)]
mod tests;
