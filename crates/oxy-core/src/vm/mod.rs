//! Virtual machine for executing Oxy programs.
//!
//! The bytecode VM (OpCode/Chunk/Vm) has been retired in favor of
//! the Cranelift JIT backend in `vm/jit/`. This module now holds
//! only the shared types and modules that both backends depend on.

use crate::types::Value;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod jit;
pub(crate) mod scheduler;

/// A call frame on the bytecode VM's call stack.
///
/// The JIT backend does not use this struct (it has its own `CallFrame` in
/// `ffi.rs`), but the scheduler still references it for the old VM path.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct Frame {
    pub(crate) return_ip: usize,
    pub(crate) locals: Vec<Value>,
    pub(crate) caller_op_stack_len: usize,
    pub(crate) fn_ip: usize,
    pub(crate) write_back_slot: Option<usize>,
}

/// Result of VM execution.
pub enum VmResult {
    Value(Value),
    Error(String),
}

mod api;
pub(super) mod arith;
pub mod builtins;
pub use api::*;

#[cfg(test)]
mod tests;
