//! FFI bridge: Rust functions callable from JIT-compiled code.
//!
//! Every function registered via `register_ffi_symbols` becomes a cranelift
//! import symbol that Oxy JIT functions can `call`.
//!
//! All FFI functions use `extern "C"` ABI and operate on raw `*mut Value`
//! pointers to avoid Cranelift needing to understand the Rust Value enum layout.

// Phase 1: FFI symbol registration will be used when the translator is added.
#![allow(dead_code)]

use cranelift_jit::JITBuilder;

// ── Arithmetic FFI ──────────────────────────────────────────────────────

extern "C" fn jit_add_int(_ctx: *mut super::context::JitContext) {
    // placeholder body — populated in Phase 2
}

extern "C" fn jit_print(_ctx: *mut super::context::JitContext) {
    // placeholder body — populated in Phase 2
}

// ── FFI symbol registry ──────────────────────────────────────────────────

/// Register all FFI symbols with the JIT builder so JIT-compiled functions
/// can import them.
pub(crate) fn register_ffi_symbols(builder: &mut JITBuilder) {
    // Register Oxy FFI symbols
    builder.symbol("oxy_add_int", jit_add_int as *const u8);
    builder.symbol("oxy_print", jit_print as *const u8);
}
