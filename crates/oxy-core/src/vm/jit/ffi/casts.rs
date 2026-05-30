//! Numeric / char cast FFI — part of the shared oxy_* runtime. See `mod.rs`
//! for the core machinery (push/pop, call stack, ffi_symbols).

use super::*;

pub(super) extern "C" fn oxy_cast_int(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let result = super::super::runtime::cast_to_int(&val, crate::types::IntegerWidth::I64);
    unsafe {
        push(ctx, result);
    }
}

pub(super) extern "C" fn oxy_cast_byte(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let result = super::super::runtime::cast_to_int(&val, crate::types::IntegerWidth::U8);
    unsafe {
        push(ctx, result);
    }
}

pub(super) extern "C" fn oxy_cast_float(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let result = super::super::runtime::cast_to_float(&val, crate::types::FloatWidth::F64);
    unsafe {
        push(ctx, result);
    }
}

pub(super) extern "C" fn oxy_cast_to_char(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let n = super::super::runtime::value_to_i64(&val);
    let c = char::from_u32(n as u32).unwrap_or('\u{FFFD}');
    unsafe {
        push(ctx, Value::Char(c));
    }
}
