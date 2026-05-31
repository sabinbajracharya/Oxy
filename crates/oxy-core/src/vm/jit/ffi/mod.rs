//! FFI bridge: Rust functions callable from JIT-compiled code.
//!
//! All functions use `extern "C"` ABI and operate on `*mut JitContext`.
//!
//! The JitContext buffer layout:
//! ```text
//! [locals: local_count × Value] [operand stack: sp × Value]
//! ```

/// Set ctx.result directly — bypasses operand-stack round-trip for plain returns.
#[no_mangle]
extern "C" fn oxy_set_result_i64(ctx: *mut super::JitContext, val: i64) {
    let ctx = unsafe { &mut *ctx };
    ctx.result = crate::types::Value::I64(val);
}

extern "C" fn oxy_set_result_bool(ctx: *mut super::JitContext, val: u8) {
    let ctx = unsafe { &mut *ctx };
    ctx.result = crate::types::Value::Bool(val != 0);
}

extern "C" fn oxy_set_result_char(ctx: *mut super::JitContext, val: u32) {
    let ctx = unsafe { &mut *ctx };
    ctx.result = crate::types::Value::Char(char::from_u32(val).unwrap_or('\0'));
}

extern "C" fn oxy_set_result_float(ctx: *mut super::JitContext, val: f64) {
    let ctx = unsafe { &mut *ctx };
    ctx.result = crate::types::Value::F64(val);
}

extern "C" fn oxy_set_result_unit(ctx: *mut super::JitContext) {
    let ctx = unsafe { &mut *ctx };
    ctx.result = crate::types::Value::Unit;
}

/// Set ctx.result from a local buffer slot — bypasses operand-stack round-trip
/// for spill-slot returns (equivalent to oxy_load_local followed by oxy_return).
extern "C" fn oxy_set_result_local(ctx: *mut super::JitContext, slot: usize) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { ctx.buffer.add(slot).read() };
    let result = match &val {
        crate::types::Value::Cell(rc) => rc.borrow().clone(),
        other => other.clone(),
    };
    // val is a shallow bitwise copy — forget it to prevent a double-free.
    std::mem::forget(val);
    ctx.result = result;
}

use super::context::{JitContext, JitTables};
use crate::types::Value;
#[cfg(not(target_arch = "wasm32"))]
use cranelift_jit::JITBuilder;
use std::collections::HashMap;

/// Raw pointer to the run's captured-output buffer (mirrors `JitContext.output`).
/// A null pointer means "print to stdout"; otherwise printed lines are pushed
/// into the shared `Vec<String>`. Threaded into child contexts so output capture
/// follows execution into closures and spawned tasks.
type OutputPtr = *const std::rc::Rc<std::cell::RefCell<Vec<String>>>;

// ── Stack helpers (used by JIT and FFI internally) ──────────────────

pub(crate) unsafe fn push(ctx: &mut JitContext, val: Value) {
    let slot = ctx.push_slot();
    unsafe {
        slot.write(val);
    }
}

/// Write an error message to the context, replacing any existing error.
pub(crate) fn set_error(ctx: &mut JitContext, msg: String) {
    let len = msg.len().min(1023);
    ctx.error_msg[..len].copy_from_slice(&msg.as_bytes()[..len]);
    // Ensure error_len is non-zero even for empty messages (e.g. ? propagation).
    ctx.error_len = if len == 0 { 1 } else { len };
}

pub(crate) unsafe fn pop(ctx: &mut JitContext) -> Value {
    if ctx.sp == 0 {
        return Value::Unit;
    }
    ctx.sp -= 1;
    let src = unsafe { ctx.buffer.add(ctx.local_count + ctx.sp) };
    let val = unsafe { src.read() };
    // Clear the source slot so the caller's buffer doesn't double-free.
    unsafe { src.write(Value::Unit) };
    val
}

// ── Constants ────────────────────────────────────────────────────────

extern "C" fn oxy_push_unit(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    unsafe {
        push(ctx, Value::Unit);
    }
}

extern "C" fn oxy_push_bool(ctx: *mut JitContext, val: u8) {
    let ctx = unsafe { &mut *ctx };
    unsafe {
        push(ctx, Value::Bool(val != 0));
    }
}

extern "C" fn oxy_push_int(ctx: *mut JitContext, val: i64) {
    let ctx = unsafe { &mut *ctx };
    unsafe {
        push(ctx, Value::I64(val));
    }
}

extern "C" fn oxy_push_float(ctx: *mut JitContext, val: f64) {
    let ctx = unsafe { &mut *ctx };
    unsafe {
        push(ctx, Value::F64(val));
    }
}

extern "C" fn oxy_push_char(ctx: *mut JitContext, val: u32) {
    let ctx = unsafe { &mut *ctx };
    unsafe {
        let c = char::from_u32(val).unwrap_or('\u{FFFD}');
        push(ctx, Value::Char(c));
    }
}

extern "C" fn oxy_push_string(ctx: *mut JitContext, ptr: *const u8, len: usize) {
    let ctx = unsafe { &mut *ctx };
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let s = String::from_utf8_lossy(bytes).into_owned();
    unsafe {
        push(ctx, Value::String(s));
    }
}

// ── Stack manipulation ───────────────────────────────────────────────

extern "C" fn oxy_pop(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let _ = unsafe { pop(ctx) };
}

extern "C" fn oxy_dup(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    if ctx.sp == 0 {
        set_error(ctx, "JIT stack underflow on dup".to_string());
        return;
    }
    let val = unsafe { ctx.buffer.add(ctx.local_count + ctx.sp - 1).read() };
    let val_clone = val.clone();
    // Prevent double-free: val is a shallow copy sharing heap pointers with the original
    std::mem::forget(val);
    unsafe {
        push(ctx, val_clone);
    }
}

// ── Variables ────────────────────────────────────────────────────────

extern "C" fn oxy_load_local(ctx: *mut JitContext, index: usize) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { ctx.buffer.add(index).read() };
    // If it's a Cell, load through it; otherwise clone
    let to_push = match &val {
        Value::Cell(rc) => rc.borrow().clone(),
        other => other.clone(),
    };
    // CRITICAL: val is a shallow bitwise copy (ptr::read). Its Drop would
    // free heap memory still owned by the original in the locals buffer.
    // Forget it to prevent a double-free.
    std::mem::forget(val);
    unsafe {
        push(ctx, to_push);
    }
}

/// Load a local WITHOUT Cell unwrapping — preserves Cell for mutable receivers.
extern "C" fn oxy_load_local_raw(ctx: *mut JitContext, index: usize) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { ctx.buffer.add(index).read() };
    let to_push = match &val {
        Value::Cell(rc) => Value::Cell(std::rc::Rc::clone(rc)),
        other => other.clone(),
    };
    std::mem::forget(val);
    unsafe {
        push(ctx, to_push);
    }
}

extern "C" fn oxy_store_local(ctx: *mut JitContext, index: usize) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    // If the target is a Cell, write through it
    let target = unsafe { ctx.buffer.add(index) };
    let is_cell = unsafe { matches!(&*target, Value::Cell(_)) };
    if is_cell {
        if let Value::Cell(rc) = unsafe { &*target } {
            *rc.borrow_mut() = val;
        }
    } else {
        unsafe {
            target.write(val);
        }
    }
}

/// Store into a slot transparently: always overwrite, never write through a
/// `Cell`. Used for spill slots, which are transient register storage and must
/// round-trip values faithfully (the dual of `oxy_load_local_raw`).
///
/// `oxy_store_local` writes *through* a cell so `self.field = v` and captured
/// mutable variables propagate — correct for real locals. But a spill slot that
/// happens to hold a `Cell` (from a previous iteration spilling a `LoadLocalRaw`
/// result) would, under that rule, have the next iteration's value written into
/// the cell's interior, producing `Cell(Cell(..))` and corrupting dispatch. Here
/// the previous occupant is dropped and the new value written in its place, so a
/// spilled cell is replaced rather than nested. The slot is either zeroed
/// (`I64(0)`, a no-op drop) or a prior valid value, so `drop_in_place` is sound.
extern "C" fn oxy_store_local_raw(ctx: *mut JitContext, index: usize) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let target = unsafe { ctx.buffer.add(index) };
    unsafe {
        std::ptr::drop_in_place(target);
        target.write(val);
    }
}

/// Read a local slot and return its raw i64 representation.
/// Returns 0 for non-integer types (they always flow through the FFI stack).
extern "C" fn oxy_read_local_i64(ctx: *mut JitContext, index: usize) -> i64 {
    let ctx = unsafe { &mut *ctx };
    let slot = unsafe { &*ctx.buffer.add(index) };
    match slot {
        Value::I64(n) => *n,
        Value::Bool(b) => *b as i64,
        Value::U8(b) => *b as i64,
        Value::Unit => 0,
        _ => 0,
    }
}

extern "C" fn oxy_make_cell(ctx: *mut JitContext, index: usize) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { ctx.buffer.add(index).read() };
    // Idempotent: a slot that already holds a `Cell` is left untouched. The
    // compile-time `celled_slots` guard only stops the op being *emitted* twice;
    // it cannot stop the single emitted op from *executing* repeatedly when it
    // sits in a loop (e.g. a `mut` receiver in a `while let` condition). Without
    // this guard a second execution would wrap `Cell(v)` into `Cell(Cell(v))`,
    // and dispatch would then see the inner cell instead of the real value.
    if matches!(&val, Value::Cell(_)) {
        // `val` is a shallow bitwise copy of the slot; forget it so its Drop
        // doesn't decrement the Rc still owned by the slot.
        std::mem::forget(val);
        return;
    }
    let cell = Value::Cell(std::rc::Rc::new(std::cell::RefCell::new(val)));
    // val was a shallow copy; it has been moved into the Rc so it won't Drop.
    // The original in the buffer is overwritten by write() below (write does
    // not drop the old value, so no double-free).
    unsafe {
        ctx.buffer.add(index).write(cell);
    }
}

// ── Output ───────────────────────────────────────────────────────────

// `format_template` (the `format!`/`print!`/`println!` placeholder engine)
// lives in `crate::types` so it is reachable wasm-side and from the stdlib
// registry without depending on the Cranelift-gated `jit` module. The JIT print
// builtins use the `_with` variant to layer on `Display::fmt` dispatch.
use crate::types::format_template_with;

extern "C" fn oxy_print_val(ctx: *mut JitContext, count: usize) {
    let ctx = unsafe { &mut *ctx };
    let mut vals = Vec::with_capacity(count);
    for _ in 0..count {
        vals.push(unsafe { pop(ctx) });
    }
    vals.reverse();
    if vals.is_empty() {
        return;
    }
    let template = vals[0].to_string();
    if count == 1 {
        print!("{template}");
        return;
    }
    let result = format_template_with(&template, &vals[1..], |v| unsafe {
        display_via_user_fmt(ctx, v)
    });
    print!("{result}");
}

extern "C" fn oxy_println_val(ctx: *mut JitContext, count: usize) {
    let ctx = unsafe { &mut *ctx };
    let mut vals = Vec::with_capacity(count);
    for _ in 0..count {
        vals.push(unsafe { pop(ctx) });
    }
    vals.reverse();
    let line = if vals.is_empty() {
        String::new()
    } else if count == 1 {
        vals[0].to_string()
    } else {
        let template = vals[0].to_string();
        format_template_with(&template, &vals[1..], |v| unsafe {
            display_via_user_fmt(ctx, v)
        })
    };
    if !ctx.output.is_null() {
        let output = unsafe { &*ctx.output };
        output.borrow_mut().push(format!("{line}\n"));
    } else {
        println!("{line}");
    }
}

// ── Binary arithmetic ────────────────────────────────────────────────

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

macro_rules! binary_op {
    ($name:ident, $func:path, $method:expr) => {
        extern "C" fn $name(ctx: *mut JitContext) {
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
        extern "C" fn $name(ctx: *mut JitContext) {
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

binary_op!(oxy_add, super::runtime::vm_add, "add");
binary_op!(oxy_sub, super::runtime::vm_sub, "sub");
binary_op!(oxy_mul, super::runtime::vm_mul, "mul");
binary_op!(oxy_div, super::runtime::vm_div, "div");
binary_op!(oxy_mod, super::runtime::vm_rem, "rem");

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

// ── Bitwise ──────────────────────────────────────────────────────────

binary_op!(oxy_bitand, super::runtime::vm_bitand, "bitand");
binary_op!(oxy_bitor, super::runtime::vm_bitor, "bitor");
binary_op!(oxy_bitxor, super::runtime::vm_bitxor, "bitxor");
binary_op!(oxy_shl, super::runtime::vm_shl, "shl");
binary_op!(oxy_shr, super::runtime::vm_shr, "shr");

// ── Unary ────────────────────────────────────────────────────────────

extern "C" fn oxy_neg(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    if matches!(&val, Value::Struct { .. } | Value::EnumVariant { .. }) {
        if let Some((fn_index, fp)) = lookup_op_method(ctx, &val, "neg") {
            let result = invoke_unary_op_method(ctx, val, fn_index, fp);
            unsafe { push(ctx, result) };
            return;
        }
    }
    let result = super::runtime::vm_neg(val);
    unsafe {
        push(ctx, result);
    }
}

extern "C" fn oxy_not(ctx: *mut JitContext) {
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

extern "C" fn oxy_bitnot(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    if matches!(&val, Value::Struct { .. } | Value::EnumVariant { .. }) {
        if let Some((fn_index, fp)) = lookup_op_method(ctx, &val, "bitnot") {
            let result = invoke_unary_op_method(ctx, val, fn_index, fp);
            unsafe { push(ctx, result) };
            return;
        }
    }
    let result = super::runtime::vm_bitnot(val);
    unsafe {
        push(ctx, result);
    }
}

// ── Control flow helpers ─────────────────────────────────────────────

extern "C" fn oxy_is_falsy(ctx: *mut JitContext) -> u8 {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    if val.is_truthy() {
        0
    } else {
        1
    }
}

extern "C" fn oxy_is_truthy(ctx: *mut JitContext) -> u8 {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    if val.is_truthy() {
        1
    } else {
        0
    }
}

// ── Function calls ───────────────────────────────────────────────────

// ── CalleeFrame: buffer lifecycle for JIT function calls ────────────────
//
// Every JIT function call needs a fresh buffer where the callee's locals
// and operand stack live. CalleeFrame encapsulates the alloc / swap / call /
// drop / dealloc / restore pattern so it isn't duplicated at every call site.

const STACK_CAP: usize = 2048;

struct CalleeFrame {
    buf: *mut Value,
    layout: std::alloc::Layout,
    capacity: usize,
    local_count: usize,
}

impl CalleeFrame {
    fn new(min_locals: usize) -> Self {
        let capacity = min_locals + STACK_CAP;
        let layout = std::alloc::Layout::array::<Value>(capacity).unwrap();
        let buf = unsafe { std::alloc::alloc_zeroed(layout) as *mut Value };
        Self {
            buf,
            layout,
            capacity,
            local_count: min_locals,
        }
    }

    fn buf_mut(&mut self) -> *mut Value {
        self.buf
    }

    /// Swap this frame into ctx, call fn_ptr, drop callee state, dealloc,
    /// restore the caller's buffer, and push the result onto the caller's stack.
    ///
    /// `saved_local_count` is the caller's local_count before the call.
    /// `result_sp` is the caller's sp value after consuming args (where the
    ///   callee's result should be pushed).
    unsafe fn execute(
        self,
        ctx: &mut JitContext,
        fn_ptr: *const u8,
        saved_local_count: usize,
        result_sp: usize,
    ) {
        let saved_buf = ctx.buffer;
        let saved_cap = ctx.capacity;

        ctx.buffer = self.buf;
        ctx.capacity = self.capacity;
        ctx.local_count = self.local_count;
        ctx.sp = 0;

        let jit_fn: extern "C" fn(*mut JitContext) -> u64 =
            std::mem::transmute(fn_ptr as *const ());
        let _discriminant = jit_fn(ctx);

        // Drop callee's locals and any remaining stack
        for i in 0..ctx.local_count {
            std::ptr::drop_in_place(ctx.buffer.add(i));
        }
        for i in 0..ctx.sp {
            std::ptr::drop_in_place(ctx.buffer.add(ctx.local_count + i));
        }
        std::alloc::dealloc(ctx.buffer as *mut u8, self.layout);

        // Restore caller's buffer
        ctx.buffer = saved_buf;
        ctx.capacity = saved_cap;

        // Push result onto caller's operand stack
        let result = std::mem::replace(&mut ctx.result, Value::Unit);
        // The error flag is intra-function plumbing for `?`: set by oxy_try_pop,
        // consumed by the very next CheckError in the SAME function. A `?`
        // short-circuit signals via set_error with an EMPTY message; once we've
        // materialized the Err/None onto the caller's stack as an ordinary
        // Result/Option value, that signal has done its job and must be cleared
        // — otherwise the caller's next CheckError fires spuriously (a leaked
        // flag from one call would short-circuit the following call). A real
        // runtime error carries a non-empty message and is left set so it
        // bubbles up to the top-level call_fn.
        if ctx.error_len == 1 && ctx.error_msg[0] == 0 {
            ctx.error_len = 0;
        }
        ctx.local_count = saved_local_count;
        ctx.sp = result_sp;
        push(ctx, result);
    }
}

/// Call a JIT-compiled function identified by fn_ptr with args already on
/// the caller's operand stack. This is the shared call path used by both
/// oxy_call (simple calls) and oxy_path_call_builtin (module-qualified calls).
/// Invoke a JIT-compiled function with `args` already owned (drained off the
/// caller's operand stack by the caller). `args[i]` becomes callee local `i`,
/// so order is preserved by construction — there is no operand-stack round-trip
/// to get backwards. Mirrors how `oxy_call_closure` builds its callee frame
/// directly from a Vec.
fn invoke_jit_fn(ctx: &mut JitContext, fn_ptr: *const u8, local_count: usize, args: Vec<Value>) {
    // The caller already popped the args, so the operand stack top is where the
    // call's result will land.
    let result_sp = ctx.sp;
    let mut frame = CalleeFrame::new(local_count);
    for (i, arg) in args.into_iter().enumerate() {
        unsafe { frame.buf_mut().add(i).write(arg) };
    }

    let saved_local_count = ctx.local_count;
    unsafe {
        frame.execute(ctx, fn_ptr, saved_local_count, result_sp);
    }
}

// ── Closures ─────────────────────────────────────────────────────────

extern "C" fn oxy_push_closure(
    ctx: *mut JitContext,
    name_ptr: *const u8,
    name_len: usize,
    meta_idx: usize,
) {
    let ctx = unsafe { &mut *ctx };

    let name = unsafe {
        let bytes = std::slice::from_raw_parts(name_ptr, name_len);
        String::from_utf8_lossy(bytes).into_owned()
    };

    // Look up captures metadata.
    let tables = unsafe { &*ctx.tables };
    let meta = tables.closure_meta(meta_idx).cloned();
    let (param_names, captured, is_async) = meta
        .map(|m| (m.param_names, m.captured, m.is_async))
        .unwrap_or_default();

    // Build captured values from current locals at the outer slots.
    // For Cell (mutable) variables, share the Rc<RefCell> so mutations
    // are visible in both the closure and the outer function.
    let closure_env = crate::env::Environment::new();
    for (captured_name, outer_slot, is_mut) in &captured {
        let shallow = unsafe { ctx.buffer.add(*outer_slot).read() };
        let val = match &shallow {
            Value::Cell(rc) => Value::Cell(std::rc::Rc::clone(rc)),
            other => other.clone(),
        };
        std::mem::forget(shallow);
        closure_env
            .borrow_mut()
            .define(captured_name.clone(), val, *is_mut);
    }

    let fn_index = tables.name_to_index(&name).unwrap_or(usize::MAX);

    let captured_names: Vec<String> = captured.iter().map(|(n, _, _)| n.clone()).collect();
    let placeholder_span = crate::lexer::Span {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    };
    let fn_data = crate::types::FunctionData {
        name,
        params: param_names
            .iter()
            .map(|n| crate::ast::Param {
                name: n.clone(),
                type_ann: crate::ast::TypeAnnotation::Named {
                    name: "int".into(),
                    generic_args: vec![],
                    span: placeholder_span,
                },
                is_mut: false,
                span: placeholder_span,
            })
            .collect(),
        return_type: None,
        body: crate::ast::Block {
            stmts: vec![],
            span: crate::lexer::Span {
                start: 0,
                end: 0,
                line: 0,
                column: 0,
            },
        },
        closure_env,
        target_ip: Some(fn_index),
        captured_names,
        is_async,
    };
    unsafe {
        push(ctx, Value::Function(Box::new(fn_data)));
    }
}

/// Create a `Value::Function` for a named (non-closure) function so it can
/// be called through the same `oxy_call_closure` path as everything else.
extern "C" fn oxy_push_named_fn(ctx: *mut JitContext, name_ptr: *const u8, name_len: usize) {
    let ctx = unsafe { &mut *ctx };
    let name = unsafe {
        let bytes = std::slice::from_raw_parts(name_ptr, name_len);
        String::from_utf8_lossy(bytes).into_owned()
    };
    let tables = unsafe { &*ctx.tables };
    let fn_index = match tables.name_to_index(&name) {
        Some(idx) => idx,
        None => {
            set_error(ctx, format!("JIT: function not found: {name}"));
            unsafe { push(ctx, Value::Unit) };
            return;
        }
    };
    let placeholder_span = crate::lexer::Span {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    };
    let fn_data = crate::types::FunctionData {
        name,
        params: vec![],
        return_type: None,
        body: crate::ast::Block {
            stmts: vec![],
            span: placeholder_span,
        },
        closure_env: crate::env::Environment::new(),
        target_ip: Some(fn_index),
        captured_names: vec![],
        is_async: false,
    };
    unsafe {
        push(ctx, Value::Function(Box::new(fn_data)));
    }
}

extern "C" fn oxy_push_async_block(
    ctx: *mut JitContext,
    name_ptr: *const u8,
    name_len: usize,
    meta_idx: usize,
) {
    let ctx = unsafe { &mut *ctx };

    let name = unsafe {
        let bytes = std::slice::from_raw_parts(name_ptr, name_len);
        String::from_utf8_lossy(bytes).into_owned()
    };

    let tables = unsafe { &*ctx.tables };
    let fn_index = tables.name_to_index(&name).unwrap_or(usize::MAX);

    let meta = tables.closure_meta(meta_idx).cloned();
    let captured = meta.map(|m| m.captured.clone()).unwrap_or_default();

    let closure_env = crate::env::Environment::new();
    for (captured_name, outer_slot, is_mut) in &captured {
        let shallow = unsafe { ctx.buffer.add(*outer_slot).read() };
        let val = match &shallow {
            Value::Cell(rc) => Value::Cell(std::rc::Rc::clone(rc)),
            other => other.clone(),
        };
        std::mem::forget(shallow);
        closure_env
            .borrow_mut()
            .define(captured_name.clone(), val, *is_mut);
    }

    let captured_names: Vec<String> = captured.iter().map(|(n, _, _)| n.clone()).collect();
    let future_data = crate::types::FutureData {
        name,
        params: vec![],
        return_type: None,
        body: crate::ast::Block {
            stmts: vec![],
            span: crate::lexer::Span {
                start: 0,
                end: 0,
                line: 0,
                column: 0,
            },
        },
        closure_env,
        args: vec![],
        target_ip: fn_index,
        captured_names,
    };
    unsafe {
        push(ctx, Value::Future(Box::new(future_data)));
    }
}

extern "C" fn oxy_call_closure(ctx: *mut JitContext, arg_count: usize) {
    let ctx = unsafe { &mut *ctx };

    // Pop closure value (receiver) from below the args
    let closure_idx = ctx.sp - arg_count - 1;
    let closure_val = unsafe { ctx.buffer.add(ctx.local_count + closure_idx).read() };

    let (target_ip, is_async, captured_names, closure_env) = match &closure_val {
        Value::Function(f) => (
            f.target_ip,
            f.is_async,
            f.captured_names.clone(),
            f.closure_env.clone(),
        ),
        _ => {
            set_error(
                ctx,
                "CallClosure: value is not a callable closure".to_string(),
            );
            unsafe {
                push(ctx, Value::Unit);
            }
            return;
        }
    };

    let target_ip = match target_ip {
        Some(ip) if ip != usize::MAX => ip,
        _ => {
            set_error(ctx, "CallClosure: invalid target_ip".to_string());
            unsafe {
                push(ctx, Value::Unit);
            }
            return;
        }
    };

    if is_async {
        // Create Future instead of executing.
        // Move closure + args off the stack, clearing each source slot.
        let drain_start = ctx.sp - arg_count - 1;
        let mut args = Vec::new();
        for i in 0..arg_count {
            let src = unsafe { ctx.buffer.add(ctx.local_count + drain_start + 1 + i) };
            let val = unsafe { src.read() };
            unsafe { src.write(Value::Unit) };
            args.push(val);
        }
        // Clear the closure slot too
        unsafe {
            ctx.buffer
                .add(ctx.local_count + drain_start)
                .write(Value::Unit)
        };
        ctx.sp = drain_start;

        let fn_data = match &closure_val {
            Value::Function(f) => f.clone(),
            _ => unreachable!(),
        };
        let future = crate::types::FutureData {
            name: fn_data.name,
            params: fn_data.params,
            return_type: fn_data.return_type,
            body: fn_data.body,
            closure_env,
            args,
            target_ip,
            captured_names,
        };
        unsafe {
            push(ctx, Value::Future(Box::new(future)));
        }
        return;
    }

    // Sync closure: look up JIT fn, call it
    let tables = unsafe { &*ctx.tables };
    let fn_ptr = match tables.fn_ptr(target_ip) {
        Some(p) => p,
        None => {
            set_error(
                ctx,
                format!("JIT: no function for closure at ip={target_ip}"),
            );
            unsafe {
                push(ctx, Value::Unit);
            }
            return;
        }
    };

    let saved_local_count = ctx.local_count;
    let captures_end = captured_names.len();
    let drain_start = ctx.sp - arg_count - 1;

    // Move args off the caller's stack, clearing each source slot.
    let mut args_vals = Vec::with_capacity(arg_count);
    for i in 0..arg_count {
        let src = unsafe { ctx.buffer.add(ctx.local_count + drain_start + 1 + i) };
        args_vals.push(unsafe { src.read() });
        unsafe { src.write(Value::Unit) };
    }
    // Clear the closure slot
    unsafe {
        ctx.buffer
            .add(ctx.local_count + drain_start)
            .write(Value::Unit)
    };
    ctx.sp = drain_start;

    let fn_local_count = tables.local_count(target_ip);
    let total_frame = fn_local_count.max(captures_end + arg_count);
    let mut frame = CalleeFrame::new(total_frame);
    for (i, name) in captured_names.iter().enumerate() {
        let val = closure_env.borrow().get(name).ok().unwrap_or(Value::Unit);
        unsafe {
            frame.buf_mut().add(i).write(val);
        }
    }
    for (i, arg) in args_vals.into_iter().enumerate() {
        unsafe {
            frame.buf_mut().add(captures_end + i).write(arg);
        }
    }

    unsafe {
        frame.execute(ctx, fn_ptr, saved_local_count, ctx.sp);
    }
}

// ── Return / panic ──────────────────────────────────────────────────

extern "C" fn oxy_error_discriminant(ctx: *const JitContext) -> u64 {
    let ctx = unsafe { &*ctx };
    if ctx.error_len > 0 {
        2
    } else {
        0
    }
}

extern "C" fn oxy_return(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let result = if ctx.sp == 0 {
        Value::Unit
    } else {
        unsafe { pop(ctx) }
    };
    ctx.result = result;
}

extern "C" fn oxy_panic(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let msg_val = unsafe { pop(ctx) };
    let msg = format!("{msg_val:?}");
    let len = msg.len().min(1023);
    ctx.error_msg[..len].copy_from_slice(&msg.as_bytes()[..len]);
    ctx.error_len = len;
}

// ── Interpreter call-back hook ────────────────────────────────────────
//
// On the IR interpreter (the wasm/browser backend) the `fn_table` holds no
// native pointers, so any runtime site that would invoke a compiled function
// through `fn_table.fn_ptr(..)` has nothing to call: higher-order built-ins
// (`map`/`filter`/`fold`/`sort_by`/`for_each`/Option·Result combinators, plus
// `std::process::spawn`'s per-line callback) via `jit_closure_invoker`, the
// async eager-runs (`spawn`/`await`) via `oxy_spawn_ffi`/`oxy_await_ffi`, and
// user `Display::fmt` rendering via `display_via_user_fmt`.
//
// The interpreter installs this thread-local hook for the duration of a run.
// Each such site, on an `fn_table` miss, drives the function by *interpreting*
// it through the hook instead of calling native code. The JIT never installs
// the hook and always resolves through `fn_table`, so both backends share one
// code path and cannot silently diverge — the only difference is who runs the
// callee. See CLAUDE.md "Two execution backends".

/// Interpret the function at `target_ip` with `frame` as its initial locals
/// (captures/receiver first, then args), returning its result value or an error
/// message. The opaque `*const ()` is the installing `Interpreter`. Implemented
/// in `vm::interp`; the JIT leaves the hook unset.
pub(crate) type InterpInvokeFn = fn(*const (), usize, Vec<Value>) -> Result<Value, String>;

thread_local! {
    static INTERP_INVOKE: std::cell::Cell<Option<(InterpInvokeFn, *const ())>> =
        const { std::cell::Cell::new(None) };
}

/// Install (or clear, with `None`) the interpreter call-back hook, returning the
/// previous value so a guard can restore it — supporting reentrant/nested runs.
pub(crate) fn set_interp_invoke(
    hook: Option<(InterpInvokeFn, *const ())>,
) -> Option<(InterpInvokeFn, *const ())> {
    INTERP_INVOKE.with(|c| c.replace(hook))
}

/// The currently-installed interpreter call-back hook, if any. `None` on the JIT.
fn interp_invoke() -> Option<(InterpInvokeFn, *const ())> {
    INTERP_INVOKE.with(|c| c.get())
}

// ── Method dispatch ───────────────────────────────────────────────────

/// JIT-compatible closure invoker callback. Matches the signature
/// `Fn(&Value, &[Value]) -> Result<Value, String>` used by builtins.
fn jit_closure_invoker(
    tables: &JitTables,
    output: OutputPtr,
    func: &Value,
    args: &[Value],
) -> Result<Value, String> {
    let ft = match func {
        Value::Function(f) => f.clone(),
        _ => return Err("not a callable function".into()),
    };
    let target_ip = ft.target_ip.ok_or("function has no target_ip")?;
    // A populated `fn_table` always resolves on the JIT. A miss means we're on
    // the IR interpreter, which has no native pointer to call the closure
    // through — every higher-order built-in (map/filter/fold/sort_by/for_each/
    // Option·Result combinators, `std::process::spawn`'s callback) funnels
    // through this one invoker, so this single site routes them all to the
    // installed interpreter hook, which runs the closure by interpreting it. If
    // no hook is installed (interpreter misuse), surface the canonical marker
    // rather than silently returning a wrong/Unit result.
    let fn_ptr = match tables.fn_ptr(target_ip) {
        Some(p) => p,
        None => {
            let (invoke, data) = interp_invoke().ok_or_else(|| {
                format!("higher-order built-in (closure argument): {INTERP_UNSUPPORTED_MARKER}")
            })?;
            // Frame layout matches the native path below: captures, then args.
            let mut frame: Vec<Value> = ft
                .captured_names
                .iter()
                .map(|name| {
                    ft.closure_env
                        .borrow()
                        .get(name)
                        .ok()
                        .unwrap_or(Value::Unit)
                })
                .collect();
            frame.extend_from_slice(args);
            return invoke(data, target_ip, frame);
        }
    };

    let captures_end = ft.captured_names.len();
    let actual_local_count = tables.local_count(target_ip);
    let min_locals = captures_end + args.len();
    let local_count = actual_local_count.max(min_locals);
    let mut call_ctx = JitContext::new(local_count);
    call_ctx.tables = tables as *const JitTables;
    // Inherit the parent's capture buffer so `println!` (and other output) from
    // inside a closure driven by a Rust-side consumer loop (`for_each`, `sort_by`,
    // Option/Result combinators, …) lands in the captured output rather than
    // escaping to real stdout. A null `output` means "print to stdout", which is
    // exactly the parent's behaviour when not capturing — so this is correct
    // whether or not capture is active.
    call_ctx.output = output;
    for (i, name) in ft.captured_names.iter().enumerate() {
        let val = ft
            .closure_env
            .borrow()
            .get(name)
            .ok()
            .unwrap_or(Value::Unit);
        unsafe {
            call_ctx.buffer.add(i).write(val);
        }
    }
    for (i, arg) in args.iter().enumerate() {
        unsafe {
            call_ctx.buffer.add(captures_end + i).write(arg.clone());
        }
    }
    call_ctx.local_count = local_count;

    let fn_ptr: extern "C" fn(*mut JitContext) -> u64 =
        unsafe { std::mem::transmute(fn_ptr as *const ()) };
    let disc = fn_ptr(&mut call_ctx as *mut JitContext);
    if disc == 0 {
        Ok(std::mem::replace(&mut call_ctx.result, Value::Unit))
    } else {
        Err(String::from_utf8_lossy(&call_ctx.error_msg[..call_ctx.error_len]).into_owned())
    }
}

/// Invoke a JIT-compiled method (`fn_index`/`fp` resolved by the caller) with an
/// explicit receiver and argument list, returning its result `Value`.
///
/// This is the shared frame-setup/teardown for every "call a compiled method from
/// FFI" site: regular method dispatch (`oxy_method_call`) and `Display` rendering
/// in the format builtins both route through here so the buffer-swap and cleanup
/// invariants live in exactly one place. A fresh callee buffer is allocated, the
/// receiver and args are written into the new frame, `ctx`'s stack window is
/// swapped to it for the duration of the call, then all callee slots are dropped
/// and the original window restored. Safe to call reentrantly (e.g. formatting a
/// struct while another method is mid-flight) because it never touches the
/// caller's live stack region.
///
/// # Safety
/// `ctx` must be valid and `fp` must be the JIT entry point for `fn_index`.
unsafe fn invoke_compiled_method(
    ctx: &mut JitContext,
    fn_index: usize,
    fp: usize,
    receiver: Value,
    args: Vec<Value>,
) -> Value {
    let tables = unsafe { &*ctx.tables };
    let arg_count = args.len();
    let fn_local_count = tables.local_count(fn_index);
    let total_frame = fn_local_count.max(1 + arg_count);
    const STACK_CAP2: usize = 2048;
    let callee_cap = total_frame + STACK_CAP2;
    let callee_layout = std::alloc::Layout::array::<Value>(callee_cap).unwrap();
    let callee_buf = unsafe { std::alloc::alloc_zeroed(callee_layout) as *mut Value };

    unsafe {
        callee_buf.add(0).write(receiver);
    }
    for (i, arg) in args.into_iter().enumerate() {
        unsafe {
            callee_buf.add(1 + i).write(arg);
        }
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

/// Render a value via its user-defined `Display::fmt` method, if one exists.
///
/// `{}` placeholders in `format!`/`println!` should use a struct/enum's `fmt`
/// method when the type implements `Display` (`fn fmt(self) -> String`). Returns
/// `Some(rendered)` when such a method is found and invoked, `None` otherwise so
/// the caller falls back to the value's default `to_string`. Only structs and
/// enum variants can carry user methods, so every other value returns `None`.
///
/// # Safety
/// `ctx` must be valid (used to resolve and invoke the compiled `fmt`).
unsafe fn display_via_user_fmt(ctx: &mut JitContext, value: &Value) -> Option<String> {
    let lookup_name = match value {
        Value::Struct { name, .. } => name.clone(),
        Value::EnumVariant { enum_name, .. } => enum_name.clone(),
        _ => return None,
    };
    let (fn_index, fp) = {
        let tables = unsafe { &*ctx.tables };
        let qualified = format!("{lookup_name}::fmt");
        let fn_index = tables.name_to_index(&qualified)?;
        match tables.fn_table.get(&fn_index).copied() {
            Some(fp) => (fn_index, fp),
            None => {
                // IR interpreter: no native code. Render through the interpreter
                // hook (frame = [receiver]); fall back to the default `to_string`
                // if no hook is installed.
                let (invoke, data) = interp_invoke()?;
                let result = invoke(data, fn_index, vec![value.clone()]).ok()?;
                return Some(match result {
                    Value::String(s) => s,
                    other => other.to_string(),
                });
            }
        }
    };
    let result = unsafe { invoke_compiled_method(ctx, fn_index, fp, value.clone(), Vec::new()) };
    Some(match result {
        Value::String(s) => s,
        other => other.to_string(),
    })
}

extern "C" fn oxy_method_call(
    ctx: *mut JitContext,
    name_ptr: *const u8,
    name_len: usize,
    arg_count: usize,
) {
    let ctx = unsafe { &mut *ctx };
    let name_bytes = unsafe { std::slice::from_raw_parts(name_ptr, name_len) };
    let method_name = String::from_utf8_lossy(name_bytes);

    // Drain args and receiver
    let mut args = Vec::with_capacity(arg_count);
    for _ in 0..arg_count {
        args.push(unsafe { pop(ctx) });
    }
    args.reverse();
    let receiver = unsafe { pop(ctx) };

    // A mutable local receiver arrives wrapped in a `Value::Cell` so a `mut self`
    // method can write the updated struct back through the shared storage (see the
    // MethodCall lowering in ir_gen). Dispatch keys off the *inner* value's type,
    // but the cell itself is what we hand to the method as `self` — unwrapping here
    // would sever the write-back path. Peek through the cell for the lookup name
    // while leaving `receiver` (the cell) intact for the call.
    let dispatch_value = match &receiver {
        Value::Cell(rc) => rc.borrow().clone(),
        other => other.clone(),
    };

    // Determine lookup name from receiver type (like old VM does)
    let type_name = dispatch_value.type_name().to_string();
    let lookup_name = match &dispatch_value {
        Value::Struct { name, .. } => name.clone(),
        Value::EnumVariant { enum_name, .. } => enum_name.clone(),
        _ => type_name,
    };

    // JIT name-based lookup for user-defined struct/enum methods.
    let tables = unsafe { &*ctx.tables };
    // The old method_ips table is bytecode-only; JIT-compiled methods are
    // registered by qualified name (e.g. "Counter::inc") in the fn table.
    let qualified = format!("{lookup_name}::{method_name}");
    if let Some(fn_index) = tables.name_to_index(&qualified) {
        if let Some(fp) = tables.fn_table.get(&fn_index).copied() {
            let result = unsafe { invoke_compiled_method(ctx, fn_index, fp, receiver, args) };
            unsafe {
                push(ctx, result);
            }
            return;
        }
    }

    // Fall back to built-in dispatch. Built-in receivers (Vec, String, …) are
    // either value types or already `Rc<RefCell>`-shared, so they're never the
    // cell-wrapped mutable receivers the user-method path relies on — dispatch on
    // the unwrapped value so a celled mutable local (e.g. `let mut v = vec![]`)
    // still resolves to the underlying collection's methods.
    let result = dispatch_builtin_method(
        tables,
        ctx.output,
        dispatch_value.clone(),
        &method_name,
        args.clone(),
    );

    match result {
        Ok(val) => unsafe {
            push(ctx, val);
        },
        Err(e) => {
            set_error(ctx, format!("method call '{method_name}' failed: {e}"));
            unsafe {
                push(ctx, Value::Unit);
            }
        }
    }
}

/// Reimplementation of Vm::builtin_method, minus the Vm dependency.
/// Dispatch an OOP-style `Regex` method (`re.is_match(text)`, `re.find(text)`,
/// …) by delegating to the canonical `std::regex` module implementation. The
/// pattern is pulled from the Regex struct's `pattern` field and prepended to
/// the arguments, matching the module functions' `(pattern, …)` shape — so the
/// regex logic lives in exactly one place.
///
/// Two OOP methods intentionally take the "do the obvious thing" semantics and
/// differ from their module counterparts:
/// - `find_all` yields the matched substrings (`Vec<String>`), whereas
///   `std::regex::find_all` yields rich `Match` structs — we project each
///   `Match` to its `text` field.
/// - `replace` replaces *every* match (mapped to module `replace_all`); the
///   module `replace` is first-match-only.
fn regex_method(receiver: &Value, method_name: &str, args: &[Value]) -> Result<Value, String> {
    let pattern = match receiver {
        Value::Struct { fields, .. } => fields
            .get("pattern")
            .cloned()
            .unwrap_or_else(|| Value::String(String::new())),
        _ => Value::String(String::new()),
    };
    let mut full_args = Vec::with_capacity(args.len() + 1);
    full_args.push(pattern);
    full_args.extend(args.iter().cloned());

    // OOP `replace` means replace-all; the module's `replace` is first-only.
    let module_fn = match method_name {
        "replace" => "replace_all",
        other => other,
    };

    let span = crate::lexer::Span {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    };
    let mut cb = |_: &Value, _: &[Value]| Err("regex methods do not take closures".to_string());
    let result = crate::stdlib::regex::call(module_fn, &full_args, &span, &mut cb)
        .map_err(|e| e.to_string())?;

    if method_name == "find_all" {
        if let Value::Vec(items) = &result {
            let strings: Vec<Value> = items
                .borrow()
                .iter()
                .map(|m| match m {
                    Value::Struct { fields, .. } => fields
                        .get("text")
                        .cloned()
                        .unwrap_or_else(|| Value::String(String::new())),
                    other => other.clone(),
                })
                .collect();
            return Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(
                strings,
            ))));
        }
    }
    Ok(result)
}

fn dispatch_builtin_method(
    tables: &JitTables,
    output: OutputPtr,
    receiver: Value,
    method_name: &str,
    args: Vec<Value>,
) -> Result<Value, String> {
    // Unwrap Cell for method dispatch. The inner value (e.g. Vec, HashMap)
    // has its own interior mutability via Rc<RefCell<>>, so mutations
    // through the clone are visible to the original Cell owner.
    let receiver = match receiver {
        Value::Cell(rc) => rc.borrow().clone(),
        other => other,
    };
    if method_name == "to_json" {
        return match crate::json::serialize(&receiver) {
            Ok(s) => Ok(Value::ok(Value::String(s))),
            Err(e) => Ok(Value::err(Value::String(e))),
        };
    }
    match &receiver {
        Value::Vec(rc) => {
            let result = crate::vm::builtins::vec::dispatch(
                Value::Vec(rc.clone()),
                method_name,
                &args,
                |f, fa| jit_closure_invoker(tables, output, &f, fa),
            );
            if result.is_ok() {
                return result;
            }
            if let Err(ref e) = result {
                if !e.starts_with("no method") {
                    return result;
                }
            }
            // Fall through to iterator dispatch
            let data = rc.borrow().clone();
            let iter = Value::Iterator(std::rc::Rc::new(std::cell::RefCell::new(
                crate::types::IteratorState::VecSource { data, index: 0 },
            )));
            crate::vm::builtins::iterator::dispatch(iter, method_name, &args, |f, fa| {
                jit_closure_invoker(tables, output, f, fa)
            })
        }
        Value::String(_) => crate::vm::builtins::string::dispatch(receiver, method_name, &args),
        Value::HashMap(_) => crate::vm::builtins::hashmap::dispatch(receiver, method_name, &args),
        Value::HashSet(_) => crate::vm::builtins::hashset::dispatch(receiver, method_name, &args),
        Value::BTreeMap(_) => crate::vm::builtins::btreemap::dispatch(receiver, method_name, &args),
        Value::BTreeSet(_) => crate::vm::builtins::btreeset::dispatch(receiver, method_name, &args),
        Value::VecDeque(_) => {
            crate::vm::builtins::vec_deque::dispatch(receiver, method_name, &args)
        }
        Value::BinaryHeap(_) => {
            crate::vm::builtins::binary_heap::dispatch(receiver, method_name, &args)
        }
        Value::Char(c) => match method_name {
            "is_digit" => Ok(Value::Bool(c.is_ascii_digit())),
            "is_alphabetic" => Ok(Value::Bool(c.is_alphabetic())),
            "is_alphanumeric" => Ok(Value::Bool(c.is_alphanumeric())),
            "is_whitespace" => Ok(Value::Bool(c.is_whitespace())),
            "is_lowercase" => Ok(Value::Bool(c.is_lowercase())),
            "is_uppercase" => Ok(Value::Bool(c.is_uppercase())),
            "is_ascii" => Ok(Value::Bool(c.is_ascii())),
            "to_lowercase" => Ok(Value::Char(c.to_lowercase().next().unwrap_or(*c))),
            "to_uppercase" => Ok(Value::Char(c.to_uppercase().next().unwrap_or(*c))),
            "clone" => Ok(Value::Char(*c)),
            "code" => Ok(Value::I64(*c as i64)),
            "to_string" => Ok(Value::String(c.to_string())),
            _ => Err(format!("no method '{method_name}' on type char")),
        },
        Value::I64(_) | Value::U8(_) | Value::F64(_) => {
            crate::vm::builtins::numeric::dispatch(receiver, method_name, &args)
        }
        Value::EnumVariant { enum_name, .. } if enum_name == "Option" => {
            crate::vm::builtins::option::dispatch(receiver, method_name, &args, |f, fa| {
                jit_closure_invoker(tables, output, &f, fa)
            })
        }
        Value::EnumVariant { enum_name, .. } if enum_name == "Result" => {
            crate::vm::builtins::result::dispatch(receiver, method_name, &args, |f, fa| {
                jit_closure_invoker(tables, output, &f, fa)
            })
        }
        Value::EnumVariant { enum_name, .. } => match method_name {
            "clone" => Ok(receiver.clone()),
            "to_string" => Ok(Value::String(receiver.to_string())),
            _ => Err(format!("no method '{method_name}' on type {enum_name}")),
        },
        Value::Struct { name, .. } if name == "Regex" => match method_name {
            "clone" => Ok(receiver.clone()),
            "to_string" => Ok(Value::String(receiver.to_string())),
            "pattern" => {
                if let Value::Struct { fields, .. } = &receiver {
                    Ok(fields
                        .get("pattern")
                        .cloned()
                        .unwrap_or(Value::String(String::new())))
                } else {
                    Ok(Value::Unit)
                }
            }
            "is_match" | "find" | "find_all" | "captures" | "replace" | "replace_all" | "split" => {
                regex_method(&receiver, method_name, &args)
            }
            _ => Err(format!("no method '{method_name}' on type Regex")),
        },
        Value::Struct { .. } => match method_name {
            "clone" => Ok(receiver.clone()),
            "to_string" => Ok(Value::String(receiver.to_string())),
            _ => Err(format!("no method '{method_name}' on struct")),
        },
        Value::Iterator(_) => {
            crate::vm::builtins::iterator::dispatch(receiver, method_name, &args, |f, fa| {
                jit_closure_invoker(tables, output, f, fa)
            })
        }
        Value::Tuple(ref _t) => match method_name {
            "clone" => Ok(receiver.clone()),
            "to_string" => Ok(Value::String(receiver.to_string())),
            _ => Err(format!("no method '{method_name}' on type tuple")),
        },
        Value::Array(ref _a) => match method_name {
            "len" => {
                if let Value::Array(a) = &receiver {
                    Ok(Value::I64(a.len() as i64))
                } else {
                    Ok(Value::I64(0))
                }
            }
            "is_empty" => {
                if let Value::Array(a) = &receiver {
                    Ok(Value::Bool(a.is_empty()))
                } else {
                    Ok(Value::Bool(true))
                }
            }
            "clone" => Ok(receiver.clone()),
            "to_string" => Ok(Value::String(receiver.to_string())),
            _ => Err(format!("no method '{method_name}' on type array")),
        },
        Value::Bool(ref _b) => match method_name {
            "clone" => Ok(receiver.clone()),
            "to_string" => Ok(Value::String(receiver.to_string())),
            _ => Err(format!("no method '{method_name}' on type bool")),
        },
        Value::Range(start, end) => {
            let iter = Value::Iterator(std::rc::Rc::new(std::cell::RefCell::new(
                crate::types::IteratorState::RangeSource {
                    current: *start,
                    end: *end,
                },
            )));
            crate::vm::builtins::iterator::dispatch(iter, method_name, &args, |f, fa| {
                jit_closure_invoker(tables, output, f, fa)
            })
        }
        _ => Err(format!(
            "no method '{method_name}' on type {}",
            receiver.type_name()
        )),
    }
}

// ── Try ─────────────────────────────────────────────────────────────

extern "C" fn oxy_try_pop(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    match &val {
        Value::EnumVariant {
            enum_name, variant, ..
        } if enum_name == "Result" && variant == "Err" => {
            ctx.result = val;
            set_error(ctx, String::new());
            unsafe {
                push(ctx, Value::Unit);
            }
        }
        Value::EnumVariant {
            enum_name, variant, ..
        } if enum_name == "Option" && variant == "None" => {
            ctx.result = val;
            set_error(ctx, String::new());
            unsafe {
                push(ctx, Value::Unit);
            }
        }
        _ => {
            // Unwrap: for Some/Ok, push inner data. For other types, pass through.
            match &val {
                Value::EnumVariant { data, .. } if !data.is_empty() => unsafe {
                    push(ctx, data[0].clone());
                },
                _ => unsafe {
                    push(ctx, val);
                },
            }
        }
    }
}

extern "C" fn oxy_bind_ident(ctx: *mut JitContext, index: usize) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    unsafe {
        ctx.buffer.add(index).write(val);
    }
}

extern "C" fn oxy_path_call_builtin(
    ctx: *mut JitContext,
    path_ptr: *const u8,
    path_len: usize,
    arg_count: usize,
) {
    let ctx = unsafe { &mut *ctx };
    let path_bytes = unsafe { std::slice::from_raw_parts(path_ptr, path_len) };
    let segments: Vec<String> = path_bytes
        .split(|b| *b == 0)
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect();
    let seg_refs: Vec<&str> = segments.iter().map(|s| s.as_str()).collect();

    let mut args = Vec::with_capacity(arg_count);
    for _ in 0..arg_count {
        args.push(unsafe { pop(ctx) });
    }
    args.reverse();

    use crate::stdlib::registry;

    // Try exact-path items first
    if let Some(handler) = registry::lookup_item(&seg_refs) {
        match handler(&args) {
            Ok(val) => unsafe {
                push(ctx, val);
            },
            Err(e) => {
                set_error(
                    ctx,
                    format!("builtin call '{}' failed: {e}", seg_refs.join("::")),
                );
                unsafe {
                    push(ctx, Value::Unit);
                }
                return;
            }
        }
        return;
    }

    // Try function call lookup FIRST: join path segments with "::" and look
    // up in the JIT function table. Must come before module dispatch so
    // user-defined modules (e.g. `mod math { fn double }`) take priority
    // over stdlib modules with the same name (e.g. math::sqrt).
    let fn_name = seg_refs.join("::");
    let tables = unsafe { &*ctx.tables };
    if let Some(fn_idx) = tables.name_to_index(&fn_name) {
        if let Some(fn_ptr) = tables.fn_ptr(fn_idx) {
            let local_count = tables.local_count(fn_idx);
            // `args` is already in original order ([arg0, arg1, …]); hand it
            // straight to invoke_jit_fn, which writes arg i to callee local i.
            invoke_jit_fn(ctx, fn_ptr, local_count, args);
            return;
        }
    }

    // Try module dispatch: [module, fn] or [std, module, fn]
    let module_route = match seg_refs.as_slice() {
        [module, func] => Some((module.to_string(), func.to_string())),
        ["std", module, func] => Some((module.to_string(), func.to_string())),
        _ => None,
    };
    if let Some((module, func)) = module_route {
        if let Some(call) = registry::lookup_module(&module) {
            match call_stdlib_jit(tables, ctx.output, call, &func, &args) {
                Ok(val) => unsafe {
                    push(ctx, val);
                },
                Err(e) => {
                    set_error(ctx, format!("module call '{module}::{func}' failed: {e}"));
                    unsafe {
                        push(ctx, Value::Unit);
                    }
                    return;
                }
            }
            return;
        }
    }

    // Try enum variant construction: 2-segment paths like EnumName::VariantName
    // that don't match any builtin, module, or function are enum constructors.
    if let [enum_name, variant] = seg_refs.as_slice() {
        unsafe {
            push(
                ctx,
                Value::EnumVariant {
                    enum_name: enum_name.to_string(),
                    variant: variant.to_string(),
                    data: args,
                },
            );
        }
        return;
    }

    set_error(
        ctx,
        format!("unknown built-in path: {}", seg_refs.join("::")),
    );
    unsafe {
        push(ctx, Value::Unit);
    }
}

/// Call a stdlib module function with JIT closure support.
fn call_stdlib_jit(
    tables: &JitTables,
    output: OutputPtr,
    module_call: crate::stdlib::registry::ModuleCall,
    func: &str,
    args: &[Value],
) -> Result<Value, String> {
    let mut cb = |f: &Value, fargs: &[Value]| jit_closure_invoker(tables, output, f, fargs);
    let span = crate::lexer::Span {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    };
    module_call(func, args, &span, &mut cb).map_err(|e| e.to_string())
}

// ── Display trait ─────────────────────────────────────────────────────

extern "C" fn oxy_display_arg(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    // Push the display string via the to_string convention
    unsafe {
        push(ctx, Value::String(val.to_string()));
    }
}
// ── Async runtime ────────────────────────────────────────────────────

// The scheduler is per-execution, not a global singleton.  A global singleton
// caused test flakiness: `JitEngine::compile()` called `reset_runtime_state()`
// which wiped tasks from a concurrently-executing test (Rust's test harness
// runs tests in parallel on separate threads).
//
// Fix: each `JitVm::call_fn` / `Interpreter::run_function` installs a fresh
// `Box<Scheduler>` into a thread-local via `SchedulerGuard`.  Parallel tests
// are on separate threads, so their schedulers never interfere.  The guard
// saves and restores the previous pointer (like `InvokerGuard`) so nested
// runs compose correctly.

thread_local! {
    static CURRENT_SCHEDULER: std::cell::Cell<*mut crate::vm::scheduler::Scheduler> =
        const { std::cell::Cell::new(std::ptr::null_mut()) };
}

/// RAII guard: allocates a `Scheduler` for the current thread's execution and
/// restores whatever was installed before (supports nested/reentrant runs).
pub(crate) struct SchedulerGuard {
    // Owns the Box so the scheduler is freed when the guard is dropped.
    // Never read directly — only held for its destructor.
    #[allow(dead_code)]
    scheduler: Box<crate::vm::scheduler::Scheduler>,
    prev: *mut crate::vm::scheduler::Scheduler,
}

impl SchedulerGuard {
    pub(crate) fn new() -> Self {
        let scheduler = Box::new(crate::vm::scheduler::Scheduler::new());
        let ptr: *mut _ = &*scheduler as *const _ as *mut _;
        let prev = CURRENT_SCHEDULER.with(|c| c.replace(ptr));
        Self { scheduler, prev }
    }
}

impl Drop for SchedulerGuard {
    fn drop(&mut self) {
        CURRENT_SCHEDULER.with(|c| c.set(self.prev));
        // `self.scheduler` is dropped here, freeing the heap allocation.
    }
}

/// Obtain a reference to the current thread's scheduler.
/// Panics if no `SchedulerGuard` is installed — callers are only reached
/// during execution where the guard is always active.
fn scheduler_ref() -> &'static mut crate::vm::scheduler::Scheduler {
    let ptr = CURRENT_SCHEDULER.with(|c| c.get());
    debug_assert!(!ptr.is_null(), "scheduler not installed for this thread");
    unsafe { &mut *ptr }
}

thread_local! {
    /// Accumulates `sleep` durations (ms) for the task body currently running
    /// eagerly. Saved/restored around each eager spawn so nested spawns don't
    /// pollute one another's count; the total becomes the task's virtual
    /// completion time for `select` ordering.
    static SLEEP_ACCUM: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

extern "C" fn oxy_await_ffi(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };

    match val {
        Value::Future(fut) => {
            let target_ip = fut.target_ip;
            let tables = unsafe { &*ctx.tables };
            let fn_ptr = match tables.fn_ptr(target_ip) {
                Some(p) => p,
                None => {
                    // IR interpreter: no native code. Interpret the future body
                    // (captures, then args) through the hook and push its result.
                    if let Some((invoke, data)) = interp_invoke() {
                        let mut frame: Vec<Value> = fut
                            .captured_names
                            .iter()
                            .map(|name| {
                                fut.closure_env
                                    .borrow()
                                    .get(name)
                                    .ok()
                                    .unwrap_or(Value::Unit)
                            })
                            .collect();
                        frame.extend(fut.args.iter().cloned());
                        let result = invoke(data, target_ip, frame).unwrap_or(Value::Unit);
                        unsafe { push(ctx, result) };
                    } else {
                        set_error(
                            ctx,
                            format!("JIT: no function for future at ip={target_ip}"),
                        );
                        unsafe { push(ctx, Value::Unit) };
                    }
                    return;
                }
            };

            let fn_local_count = tables.local_count(target_ip);
            let captures_end = fut.captured_names.len();
            let total_frame = fn_local_count.max(captures_end + fut.args.len());
            let mut frame = CalleeFrame::new(total_frame);
            for (i, name) in fut.captured_names.iter().enumerate() {
                let v = fut
                    .closure_env
                    .borrow()
                    .get(name)
                    .ok()
                    .unwrap_or(Value::Unit);
                unsafe {
                    frame.buf_mut().add(i).write(v);
                }
            }
            for (i, arg) in fut.args.iter().enumerate() {
                unsafe {
                    frame.buf_mut().add(captures_end + i).write(arg.clone());
                }
            }

            let saved_sp = ctx.sp;
            unsafe {
                frame.execute(ctx, fn_ptr, ctx.local_count, saved_sp);
            }
        }
        Value::JoinHandle { task_id } => {
            // Tasks run eagerly in oxy_spawn_ffi, so the result is always ready.
            let result = scheduler_ref().task_result(task_id);
            unsafe {
                push(ctx, result.unwrap_or(Value::Unit));
            }
        }
        other => unsafe {
            push(ctx, other);
        },
    }
}

extern "C" fn oxy_spawn_ffi(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let closure = unsafe { pop(ctx) };

    match closure {
        Value::Function(f) => {
            let target_ip = f.target_ip.unwrap_or(0);
            let tables = unsafe { &*ctx.tables };
            let capture_count = f.captured_names.len();
            let local_count = tables.local_count(target_ip).max(capture_count);

            // Eagerly run the task function synchronously. JIT functions
            // are native code that runs start-to-finish — they can't be
            // paused mid-execution and resumed. By running the task now,
            // the result is immediately available when await is called.
            //
            // Save/restore the sleep accumulator around the run so this task's
            // `sleep`s are counted independently of an enclosing task's (spawn
            // can be nested inside a spawned closure).
            let saved_accum = SLEEP_ACCUM.with(|a| a.replace(0));
            let task_result = if let Some(fn_ptr) = tables.fn_ptr(target_ip) {
                let mut task_ctx = JitContext::new(local_count);
                task_ctx.tables = tables as *const JitTables;
                // Spawned tasks inherit the run's capture buffer so output from a
                // spawned closure is captured like everything else (see OutputPtr).
                task_ctx.output = ctx.output;
                task_ctx.local_count = local_count;
                for (i, name) in f.captured_names.iter().enumerate() {
                    let val = f.closure_env.borrow().get(name).ok().unwrap_or(Value::Unit);
                    unsafe {
                        task_ctx.buffer.add(i).write(val);
                    }
                }
                let task_fn: extern "C" fn(*mut JitContext) -> u64 =
                    unsafe { std::mem::transmute(fn_ptr as *const ()) };
                let disc = task_fn(&mut task_ctx as *mut JitContext);
                if disc == 0 {
                    std::mem::replace(&mut task_ctx.result, Value::Unit)
                } else {
                    Value::Unit
                }
            } else if let Some((invoke, data)) = interp_invoke() {
                // IR interpreter: interpret the task body to completion eagerly,
                // mirroring the native eager-run. The captures form the initial
                // frame (a spawned closure takes no positional args); an error in
                // the body yields Unit, matching the native `disc != 0` branch.
                let frame: Vec<Value> = f
                    .captured_names
                    .iter()
                    .map(|name| f.closure_env.borrow().get(name).ok().unwrap_or(Value::Unit))
                    .collect();
                invoke(data, target_ip, frame).unwrap_or(Value::Unit)
            } else {
                Value::Unit
            };
            let task_sleep = SLEEP_ACCUM.with(|a| a.replace(saved_accum));

            let sched = scheduler_ref();
            let task_id = sched.create_task();
            // Record the virtual completion time and the eagerly-computed result.
            sched.set_virtual_time(task_id, task_sleep);
            sched.complete(task_id, task_result);
            unsafe {
                push(ctx, Value::JoinHandle { task_id });
            }
        }
        _ => {
            set_error(ctx, "spawn requires a closure".to_string());
            unsafe {
                push(ctx, Value::Unit);
            }
        }
    }
}

extern "C" fn oxy_sleep_ffi(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let ms_val = unsafe { pop(ctx) };
    let ms = match ms_val {
        Value::I64(n) => n as u64,
        Value::U8(n) => n as u64,
        _ => 0,
    };
    // JIT tasks run eagerly start-to-finish, so we can't actually suspend on a
    // timer. Instead we accumulate the requested delay into the running task's
    // virtual clock; `select` uses that to decide which task would finish
    // first. The accumulator is per-eager-run (see oxy_spawn_ffi).
    SLEEP_ACCUM.with(|a| a.set(a.get().saturating_add(ms)));
    unsafe {
        push(ctx, Value::Unit);
    }
}

extern "C" fn oxy_select_ffi(ctx: *mut JitContext, count: usize) {
    let ctx = unsafe { &mut *ctx };
    let mut task_ids = Vec::new();
    for _ in 0..count {
        let val = unsafe { pop(ctx) };
        if let Value::JoinHandle { task_id } = val {
            task_ids.push(task_id);
        }
    }
    // Operands are popped in reverse, so restore the original argument order.
    // `select` returns whichever task would complete first. Tasks ran eagerly,
    // so we use each task's recorded virtual time (sum of its `sleep`s) as its
    // completion time and pick the minimum; ties resolve to the earliest
    // argument for deterministic results.
    task_ids.reverse();
    let sched = scheduler_ref();
    let winner = task_ids
        .iter()
        .filter(|&&tid| sched.task_result(tid).is_some())
        .min_by_key(|&&tid| sched.task_virtual_time(tid))
        .copied();
    let result = winner.and_then(|tid| sched.task_result(tid));
    unsafe {
        push(ctx, result.unwrap_or(Value::Unit));
    }
}

// ── Symbol registry ──────────────────────────────────────────────────

/// What an `oxy_*` FFI function returns through the C ABI (independent of any
/// value it may push onto the operand stack). The IR interpreter needs this to
/// decide whether to capture a scalar return; the Cranelift backend derives the
/// same shape from `ffi_decls`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FfiRet {
    /// Returns nothing through the ABI (result, if any, is pushed on the stack).
    Void,
    /// Returns an `i64` (e.g. loop discriminants from `oxy_iter_next`).
    I64,
    /// Returns an `i8` boolean (e.g. `oxy_is_truthy`).
    I8,
}

/// Canonical phrase identifying a feature the portable IR interpreter (the
/// wasm/browser backend) cannot run because it needs native code. Both the
/// `unsupported_on_wasm!` marker in `vm::interp` and the closure-invoker miss
/// below embed it, so the `jit_interp_parity` test can recognize a deliberately
/// unsupported divergence by this stable substring. Keep the leading sentence
/// fragment "not supported by the Oxy IR interpreter" intact — it is the match
/// key.
pub(crate) const INTERP_UNSUPPORTED_MARKER: &str =
    "not supported by the Oxy IR interpreter (the wasm/browser execution backend); \
     it runs only under the native Cranelift JIT. See CLAUDE.md \u{201c}Two execution backends\u{201d}.";

/// Single source of truth for every `oxy_*` FFI function: its name, raw
/// function pointer, and ABI return kind. Both the Cranelift symbol registry
/// (native) and the IR interpreter (all targets) consume this table, so a
/// function added here is visible to both backends. The `symbol_consistency`
/// test cross-checks this against `ffi_decls` so the two never drift.
pub(crate) fn ffi_symbols() -> Vec<(&'static str, *const u8, FfiRet)> {
    use FfiRet::*;
    vec![
        ("oxy_set_result_i64", oxy_set_result_i64 as _, Void),
        ("oxy_set_result_bool", oxy_set_result_bool as _, Void),
        ("oxy_set_result_char", oxy_set_result_char as _, Void),
        ("oxy_set_result_float", oxy_set_result_float as _, Void),
        ("oxy_set_result_unit", oxy_set_result_unit as _, Void),
        ("oxy_set_result_local", oxy_set_result_local as _, Void),
        ("oxy_push_unit", oxy_push_unit as _, Void),
        ("oxy_push_bool", oxy_push_bool as _, Void),
        ("oxy_push_int", oxy_push_int as _, Void),
        ("oxy_push_float", oxy_push_float as _, Void),
        ("oxy_push_char", oxy_push_char as _, Void),
        ("oxy_push_string", oxy_push_string as _, Void),
        ("oxy_pop", oxy_pop as _, Void),
        ("oxy_dup", oxy_dup as _, Void),
        ("oxy_load_local", oxy_load_local as _, Void),
        ("oxy_load_local_raw", oxy_load_local_raw as _, Void),
        ("oxy_read_local_i64", oxy_read_local_i64 as _, I64),
        ("oxy_store_local", oxy_store_local as _, Void),
        ("oxy_store_local_raw", oxy_store_local_raw as _, Void),
        ("oxy_make_cell", oxy_make_cell as _, Void),
        ("oxy_print_val", oxy_print_val as _, Void),
        ("oxy_println_val", oxy_println_val as _, Void),
        ("oxy_add", oxy_add as _, Void),
        ("oxy_sub", oxy_sub as _, Void),
        ("oxy_mul", oxy_mul as _, Void),
        ("oxy_div", oxy_div as _, Void),
        ("oxy_mod", oxy_mod as _, Void),
        ("oxy_eq", oxy_eq as _, Void),
        ("oxy_neq", oxy_neq as _, Void),
        ("oxy_lt", oxy_lt as _, Void),
        ("oxy_gt", oxy_gt as _, Void),
        ("oxy_le", oxy_le as _, Void),
        ("oxy_ge", oxy_ge as _, Void),
        ("oxy_and", oxy_and as _, Void),
        ("oxy_or", oxy_or as _, Void),
        ("oxy_bitand", oxy_bitand as _, Void),
        ("oxy_bitor", oxy_bitor as _, Void),
        ("oxy_bitxor", oxy_bitxor as _, Void),
        ("oxy_shl", oxy_shl as _, Void),
        ("oxy_shr", oxy_shr as _, Void),
        ("oxy_neg", oxy_neg as _, Void),
        ("oxy_not", oxy_not as _, Void),
        ("oxy_bitnot", oxy_bitnot as _, Void),
        ("oxy_is_falsy", oxy_is_falsy as _, I8),
        ("oxy_is_truthy", oxy_is_truthy as _, I8),
        ("oxy_push_named_fn", oxy_push_named_fn as _, Void),
        ("oxy_push_closure", oxy_push_closure as _, Void),
        ("oxy_push_async_block", oxy_push_async_block as _, Void),
        ("oxy_call_closure", oxy_call_closure as _, Void),
        ("oxy_return", oxy_return as _, Void),
        ("oxy_error_discriminant", oxy_error_discriminant as _, I64),
        ("oxy_panic", oxy_panic as _, Void),
        ("oxy_make_array", collections::oxy_make_array as _, Void),
        (
            "oxy_make_fixed_array",
            collections::oxy_make_fixed_array as _,
            Void,
        ),
        ("oxy_make_tuple", collections::oxy_make_tuple as _, Void),
        ("oxy_make_iter", collections::oxy_make_iter as _, Void),
        ("oxy_make_repeat", collections::oxy_make_repeat as _, Void),
        ("oxy_iter_len", collections::oxy_iter_len as _, Void),
        ("oxy_iter_next", collections::oxy_iter_next as _, I64),
        (
            "oxy_iter_next_destructure",
            collections::oxy_iter_next_destructure as _,
            I64,
        ),
        ("oxy_vec_index", collections::oxy_vec_index as _, Void),
        (
            "oxy_vec_index_store",
            collections::oxy_vec_index_store as _,
            Void,
        ),
        ("oxy_make_range", collections::oxy_make_range as _, Void),
        ("oxy_to_string", strings_fmt::oxy_to_string as _, Void),
        (
            "oxy_fstring_concat",
            strings_fmt::oxy_fstring_concat as _,
            Void,
        ),
        ("oxy_format", strings_fmt::oxy_format as _, Void),
        ("oxy_dbg", strings_fmt::oxy_dbg as _, Void),
        ("oxy_struct_init", structs::oxy_struct_init as _, Void),
        ("oxy_struct_update", structs::oxy_struct_update as _, Void),
        ("oxy_field_access", structs::oxy_field_access as _, Void),
        ("oxy_field_store", structs::oxy_field_store as _, Void),
        ("oxy_method_call", oxy_method_call as _, Void),
        ("oxy_try_pop", oxy_try_pop as _, Void),
        ("oxy_cast_int", casts::oxy_cast_int as _, Void),
        ("oxy_cast_byte", casts::oxy_cast_byte as _, Void),
        ("oxy_cast_float", casts::oxy_cast_float as _, Void),
        ("oxy_cast_to_char", casts::oxy_cast_to_char as _, Void),
        ("oxy_bind_ident", oxy_bind_ident as _, Void),
        ("oxy_enum_data_get", enums::oxy_enum_data_get as _, Void),
        (
            "oxy_enum_variant_equal",
            enums::oxy_enum_variant_equal as _,
            Void,
        ),
        (
            "oxy_make_enum_variant",
            enums::oxy_make_enum_variant as _,
            Void,
        ),
        (
            "oxy_const_enum_variant",
            enums::oxy_const_enum_variant as _,
            Void,
        ),
        ("oxy_module_const", enums::oxy_module_const as _, Void),
        ("oxy_path_call_builtin", oxy_path_call_builtin as _, Void),
        ("oxy_display_arg", oxy_display_arg as _, Void),
        ("oxy_await_ffi", oxy_await_ffi as _, Void),
        ("oxy_spawn_ffi", oxy_spawn_ffi as _, Void),
        ("oxy_sleep_ffi", oxy_sleep_ffi as _, Void),
        ("oxy_select_ffi", oxy_select_ffi as _, Void),
    ]
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn register_ffi_symbols(builder: &mut JITBuilder) {
    for (name, ptr, _ret) in ffi_symbols() {
        builder.symbol(name, ptr);
    }
}

mod casts;
mod collections;
mod enums;
mod strings_fmt;
mod structs;
