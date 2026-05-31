//! Arithmetic, bitwise, comparison, and unary operator FFI functions.
//!
//! Extracted from [`super`] to keep mod.rs under ~900 lines.

use super::*;
use crate::types::Value;
use crate::vm::jit::context::JitContext;

// ── Operator trait dispatch helpers ──────────────────────────────────────

/// Try to dispatch a binary operator (a + b, a - b, etc.) to a trait method
/// when the left operand is a user-defined struct or enum variant.
/// Only looks up the method name — does not consume values.
fn lookup_op_method(ctx: &JitContext, lhs: &Value, method: &str) -> Option<(usize, usize)> {
    let lookup_name = match lhs {
        Value::Struct { name, .. } => name.clone(),
        Value::EnumVariant { enum_name, .. } => enum_name.clone(),
        _ => return None,
    };
    let tables = unsafe { &*ctx.tables };
    let qualified = format!("{lookup_name}::{method}");
    let fn_index = tables.name_to_index(&qualified)?;
    let fp = tables.fn_table.get(&fn_index).copied()?;
    Some((fn_index, fp))
}

/// Invoke a trait method for a binary operator. Consumes lhs and rhs.
fn invoke_binary_op_method(
    ctx: &mut JitContext,
    lhs: Value,
    rhs: Value,
    fn_index: usize,
    fp: usize,
) -> Value {
    let tables = unsafe { &*ctx.tables };
    let fn_local_count = tables.local_count(fn_index);
    let total_frame = fn_local_count.max(2);
    const STACK_CAP: usize = 2048;
    let callee_cap = total_frame + STACK_CAP;
    let callee_layout = std::alloc::Layout::array::<Value>(callee_cap).unwrap();
    let callee_buf = unsafe { std::alloc::alloc_zeroed(callee_layout) as *mut Value };
    unsafe {
        callee_buf.add(0).write(lhs);
    }
    unsafe {
        callee_buf.add(1).write(rhs);
    }
    let saved_buffer = ctx.buffer;
    let saved_capacity = ctx.capacity;
    let saved_local_count = ctx.local_count;
    let saved_sp = ctx.sp;
    ctx.buffer = callee_buf;
    ctx.capacity = callee_cap;
    ctx.local_count = total_frame;
    ctx.sp = 0;
    let fn_ptr: extern "C" fn(*mut JitContext) -> u64 =
        unsafe { std::mem::transmute(fp as *const ()) };
    let _disc = fn_ptr(ctx);
    for i in 0..ctx.local_count {
        unsafe {
            std::ptr::drop_in_place(ctx.buffer.add(i));
        }
    }
    for i in 0..ctx.sp {
        unsafe {
            std::ptr::drop_in_place(ctx.buffer.add(ctx.local_count + i));
        }
    }
    unsafe {
        std::alloc::dealloc(ctx.buffer as *mut u8, callee_layout);
    }
    let result = std::mem::replace(&mut ctx.result, Value::Unit);
    ctx.buffer = saved_buffer;
    ctx.capacity = saved_capacity;
    ctx.local_count = saved_local_count;
    ctx.sp = saved_sp;
    result
}

/// Invoke a trait method for a unary operator (neg, not). Consumes val.
fn invoke_unary_op_method(ctx: &mut JitContext, val: Value, fn_index: usize, fp: usize) -> Value {
    let tables = unsafe { &*ctx.tables };
    let fn_local_count = tables.local_count(fn_index);
    let total_frame = fn_local_count.max(1);
    const STACK_CAP: usize = 2048;
    let callee_cap = total_frame + STACK_CAP;
    let callee_layout = std::alloc::Layout::array::<Value>(callee_cap).unwrap();
    let callee_buf = unsafe { std::alloc::alloc_zeroed(callee_layout) as *mut Value };
    unsafe {
        callee_buf.add(0).write(val);
    }
    let saved_buffer = ctx.buffer;
    let saved_capacity = ctx.capacity;
    let saved_local_count = ctx.local_count;
    let saved_sp = ctx.sp;
    ctx.buffer = callee_buf;
    ctx.capacity = callee_cap;
    ctx.local_count = total_frame;
    ctx.sp = 0;
    let fn_ptr: extern "C" fn(*mut JitContext) -> u64 =
        unsafe { std::mem::transmute(fp as *const ()) };
    let _disc = fn_ptr(ctx);
    for i in 0..ctx.local_count {
        unsafe {
            std::ptr::drop_in_place(ctx.buffer.add(i));
        }
    }
    for i in 0..ctx.sp {
        unsafe {
            std::ptr::drop_in_place(ctx.buffer.add(ctx.local_count + i));
        }
    }
    unsafe {
        std::alloc::dealloc(ctx.buffer as *mut u8, callee_layout);
    }
    let result = std::mem::replace(&mut ctx.result, Value::Unit);
    ctx.buffer = saved_buffer;
    ctx.capacity = saved_capacity;
    ctx.local_count = saved_local_count;
    ctx.sp = saved_sp;
    result
}

// ── Macros ───────────────────────────────────────────────────────────────

macro_rules! binary_op {
    ($name:ident, $func:path, $method:expr) => {
        pub(super) extern "C" fn $name(ctx: *mut JitContext) {
            let ctx = unsafe { &mut *ctx };
            let rhs = unsafe { pop(ctx) };
            let lhs = unsafe { pop(ctx) };
            // Try trait method dispatch for struct/enum operands (non-consuming check)
            if matches!(&lhs, Value::Struct { .. } | Value::EnumVariant { .. }) {
                if let Some((fn_index, fp)) = lookup_op_method(ctx, &lhs, $method) {
                    let result = invoke_binary_op_method(ctx, lhs, rhs, fn_index, fp);
                    unsafe { push(ctx, result) };
                    return;
                }
            }
            match $func(lhs, rhs) {
                Ok(result) => unsafe { push(ctx, result) },
                Err(e) => {
                    // Write error to context instead of panicking — panics
                    // across the FFI boundary are UB (no unwind tables in JIT code).
                    let len = e.len().min(1023);
                    ctx.error_msg[..len].copy_from_slice(&e.as_bytes()[..len]);
                    ctx.error_len = len;
                    unsafe { push(ctx, Value::I64(0)) };
                }
            }
        }
    };
}

macro_rules! binary_op_val {
    ($name:ident, $func:expr) => {
        pub(super) extern "C" fn $name(ctx: *mut JitContext) {
            let ctx = unsafe { &mut *ctx };
            let rhs = unsafe { pop(ctx) };
            let lhs = unsafe { pop(ctx) };
            let result = $func(lhs, rhs);
            unsafe {
                push(ctx, result);
            }
        }
    };
}

// ── Binary arithmetic ────────────────────────────────────────────────────

binary_op!(oxy_add, crate::vm::jit::runtime::vm_add, "add");
binary_op!(oxy_sub, crate::vm::jit::runtime::vm_sub, "sub");
binary_op!(oxy_mul, crate::vm::jit::runtime::vm_mul, "mul");
binary_op!(oxy_div, crate::vm::jit::runtime::vm_div, "div");
binary_op!(oxy_mod, crate::vm::jit::runtime::vm_rem, "rem");

/// Total/partial ordering of two operands for comparison operators.
///
/// Mirrors the cross-width promotion that the arithmetic ops apply via
/// `promote_ints`: a `byte` and an `int` are both integers, so they compare on
/// their `i64` value rather than on their `Value` discriminant. Without this,
/// `U8(2) <= I64(1)` would fall through to a "not comparable" result — which is
/// exactly how a `byte` parameter (now correctly wrapped to `U8` at entry) broke
/// `n <= 1` loop/recursion guards. Mixed integer/float compares as `f64`.
/// Returns `None` only for genuinely incomparable operands (e.g. struct vs int).
fn value_ordering(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    let numeric = |v: &Value| matches!(v, Value::I64(_) | Value::U8(_) | Value::F64(_));
    if numeric(a) && numeric(b) {
        if a.is_float() || b.is_float() {
            return a.to_f64().partial_cmp(&b.to_f64());
        }
        return Some(a.as_i64().cmp(&b.as_i64()));
    }
    match (a, b) {
        (Value::String(x), Value::String(y)) => Some(x.cmp(y)),
        (Value::Char(x), Value::Char(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

fn vm_eq(lhs: Value, rhs: Value) -> Value {
    // Cross-width integers (and int/float) must compare by value, not by
    // discriminant: `U8(2) == I64(2)` is true. Non-numeric values fall back to
    // structural `PartialEq` (structs, enums, bool, unit, collections).
    if let Some(ord) = value_ordering(&lhs, &rhs) {
        return Value::Bool(ord == std::cmp::Ordering::Equal);
    }
    Value::Bool(lhs == rhs)
}
fn vm_neq(lhs: Value, rhs: Value) -> Value {
    if let Some(ord) = value_ordering(&lhs, &rhs) {
        return Value::Bool(ord != std::cmp::Ordering::Equal);
    }
    Value::Bool(lhs != rhs)
}
fn vm_lt(lhs: Value, rhs: Value) -> Value {
    Value::Bool(value_ordering(&lhs, &rhs) == Some(std::cmp::Ordering::Less))
}
fn vm_gt(lhs: Value, rhs: Value) -> Value {
    Value::Bool(value_ordering(&lhs, &rhs) == Some(std::cmp::Ordering::Greater))
}
fn vm_le(lhs: Value, rhs: Value) -> Value {
    Value::Bool(matches!(
        value_ordering(&lhs, &rhs),
        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
    ))
}
fn vm_ge(lhs: Value, rhs: Value) -> Value {
    Value::Bool(matches!(
        value_ordering(&lhs, &rhs),
        Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
    ))
}
fn vm_and(lhs: Value, rhs: Value) -> Value {
    Value::Bool(lhs.is_truthy() && rhs.is_truthy())
}
fn vm_or(lhs: Value, rhs: Value) -> Value {
    Value::Bool(lhs.is_truthy() || rhs.is_truthy())
}

binary_op_val!(oxy_eq, vm_eq);
binary_op_val!(oxy_neq, vm_neq);
binary_op_val!(oxy_lt, vm_lt);
binary_op_val!(oxy_gt, vm_gt);
binary_op_val!(oxy_le, vm_le);
binary_op_val!(oxy_ge, vm_ge);
binary_op_val!(oxy_and, vm_and);
binary_op_val!(oxy_or, vm_or);

// ── Bitwise ──────────────────────────────────────────────────────────────

binary_op!(oxy_bitand, crate::vm::jit::runtime::vm_bitand, "bitand");
binary_op!(oxy_bitor, crate::vm::jit::runtime::vm_bitor, "bitor");
binary_op!(oxy_bitxor, crate::vm::jit::runtime::vm_bitxor, "bitxor");
binary_op!(oxy_shl, crate::vm::jit::runtime::vm_shl, "shl");
binary_op!(oxy_shr, crate::vm::jit::runtime::vm_shr, "shr");

// ── Unary ────────────────────────────────────────────────────────────────

pub(super) extern "C" fn oxy_neg(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    if matches!(&val, Value::Struct { .. } | Value::EnumVariant { .. }) {
        if let Some((fn_index, fp)) = lookup_op_method(ctx, &val, "neg") {
            let result = invoke_unary_op_method(ctx, val, fn_index, fp);
            unsafe { push(ctx, result) };
            return;
        }
    }
    let result = crate::vm::jit::runtime::vm_neg(val);
    unsafe {
        push(ctx, result);
    }
}

pub(super) extern "C" fn oxy_not(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    if matches!(&val, Value::Struct { .. } | Value::EnumVariant { .. }) {
        if let Some((fn_index, fp)) = lookup_op_method(ctx, &val, "not") {
            let result = invoke_unary_op_method(ctx, val, fn_index, fp);
            unsafe { push(ctx, result) };
            return;
        }
    }
    unsafe {
        push(ctx, Value::Bool(!val.is_truthy()));
    }
}

pub(super) extern "C" fn oxy_bitnot(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    if matches!(&val, Value::Struct { .. } | Value::EnumVariant { .. }) {
        if let Some((fn_index, fp)) = lookup_op_method(ctx, &val, "bitnot") {
            let result = invoke_unary_op_method(ctx, val, fn_index, fp);
            unsafe { push(ctx, result) };
            return;
        }
    }
    let result = crate::vm::jit::runtime::vm_bitnot(val);
    unsafe {
        push(ctx, result);
    }
}
