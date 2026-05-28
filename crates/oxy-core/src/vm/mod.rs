//! Shared types and modules for Oxy program execution.
//!
//! The bytecode VM (OpCode/Chunk/Vm) has been retired in favor of
//! the Cranelift JIT backend in `vm/jit/`. This module retains only
//! the public API entry points, async scheduler, built-in methods,
//! and the shared `VmResult` type.

use crate::types::Value;

#[cfg(not(target_arch = "wasm32"))]
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
