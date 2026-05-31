//! Numeric / char cast FFI — part of the shared oxy_* runtime. See `mod.rs`
//! for the core machinery (push/pop, call stack, ffi_symbols).
//!
//! # Safety
//!
//! All functions in this module are `extern "C"` entry points called exclusively
//! from Cranelift-generated JIT code. The `*mut JitContext` pointer is guaranteed
//! valid and non-aliased for the call's duration. `pop`/`push` are unsafe because
//! they manipulate the raw operand-stack buffer; the stack has sufficient capacity
//! (pre-allocated by `JitContext::new`) and the caller ensures the right operand
//! depth for each op.

use super::*;

pub(super) extern "C" fn oxy_cast_int(ctx: *mut JitContext) {
    // Safety: ctx is a valid, non-aliased pointer from the JIT's calling convention.
    let ctx = unsafe { &mut *ctx };
    // Safety: pop/push operate on a valid, capacity-guaranteed operand stack.
    let val = unsafe { pop(ctx) };
    let result = super::super::runtime::cast_to_int(&val, crate::types::IntegerWidth::I64);
    unsafe {
        push(ctx, result);
    }
}

pub(super) extern "C" fn oxy_cast_byte(ctx: *mut JitContext) {
    // Safety: ctx is valid, owned by the calling JIT frame.
    let ctx = unsafe { &mut *ctx };
    // Safety: pop from a valid operand stack whose depth is guaranteed by the JIT.
    let val = unsafe { pop(ctx) };
    let result = super::super::runtime::cast_to_int(&val, crate::types::IntegerWidth::U8);
    unsafe {
        push(ctx, result);
    }
}

pub(super) extern "C" fn oxy_cast_float(ctx: *mut JitContext) {
    // Safety: ctx is valid, owned by the calling JIT frame.
    let ctx = unsafe { &mut *ctx };
    // Safety: pop from a valid operand stack with correct depth.
    let val = unsafe { pop(ctx) };
    let result = super::super::runtime::cast_to_float(&val, crate::types::FloatWidth::F64);
    unsafe {
        push(ctx, result);
    }
}

pub(super) extern "C" fn oxy_cast_to_char(ctx: *mut JitContext) {
    // Safety: ctx is valid, owned by the calling JIT frame.
    let ctx = unsafe { &mut *ctx };
    // Safety: pop from a valid operand stack with correct depth.
    let val = unsafe { pop(ctx) };
    let n = super::super::runtime::value_to_i64(&val);
    let c = char::from_u32(n as u32).unwrap_or('\u{FFFD}');
    unsafe {
        push(ctx, Value::Char(c));
    }
}
