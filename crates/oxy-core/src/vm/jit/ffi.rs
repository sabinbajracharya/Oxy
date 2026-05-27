//! FFI bridge: Rust functions callable from JIT-compiled code.
//!
//! All functions use `extern "C"` ABI and operate on `*mut JitContext`,
//! reading/writing Value slots in the context's buffer.
//!
//! The JitContext buffer layout:
//! ```text
//! [locals: local_count × Value] [operand stack: sp × Value]
//! ```

use super::context::JitContext;
use crate::types::Value;
use cranelift_jit::JITBuilder;
use std::collections::HashMap;

// ── Stack helpers (used by JIT and FFI internally) ──────────────────

unsafe fn push(ctx: &mut JitContext, val: Value) {
    let slot = ctx.push_slot();
    unsafe {
        slot.write(val);
    }
}

unsafe fn pop(ctx: &mut JitContext) -> Value {
    if ctx.sp == 0 {
        panic!("JIT stack underflow");
    }
    ctx.sp -= 1;
    unsafe { ctx.buffer.add(ctx.local_count + ctx.sp).read() }
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
        panic!("JIT stack underflow on dup");
    }
    let val = unsafe { ctx.buffer.add(ctx.local_count + ctx.sp - 1).read() };
    let val_clone = val.clone();
    unsafe {
        push(ctx, val_clone);
    }
}

// ── Variables ────────────────────────────────────────────────────────

extern "C" fn oxy_load_local(ctx: *mut JitContext, index: usize) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { ctx.buffer.add(index).read() };
    // If it's a Cell, load through it; otherwise clone
    match &val {
        Value::Cell(rc) => {
            let inner = rc.borrow().clone();
            unsafe {
                push(ctx, inner);
            }
        }
        other => unsafe {
            push(ctx, other.clone());
        },
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

extern "C" fn oxy_make_cell(ctx: *mut JitContext, index: usize) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { ctx.buffer.add(index).read() };
    let cell = Value::Cell(std::rc::Rc::new(std::cell::RefCell::new(val)));
    unsafe {
        ctx.buffer.add(index).write(cell);
    }
}

// ── Output ───────────────────────────────────────────────────────────

extern "C" fn oxy_print_val(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    print!("{val}");
}

extern "C" fn oxy_println_val(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    println!("{val}");
}

// ── Binary arithmetic ────────────────────────────────────────────────

macro_rules! binary_op {
    ($name:ident, $func:path) => {
        extern "C" fn $name(ctx: *mut JitContext) {
            let ctx = unsafe { &mut *ctx };
            let rhs = unsafe { pop(ctx) };
            let lhs = unsafe { pop(ctx) };
            let result = $func(lhs, rhs).unwrap_or_else(|e| panic!("{e}"));
            unsafe {
                push(ctx, result);
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

binary_op!(oxy_add, crate::vm::arith::vm_add);
binary_op!(oxy_sub, crate::vm::arith::vm_sub);
binary_op!(oxy_mul, crate::vm::arith::vm_mul);
binary_op!(oxy_div, crate::vm::arith::vm_div);
binary_op!(oxy_mod, crate::vm::arith::vm_rem);

fn vm_eq(lhs: Value, rhs: Value) -> Value {
    Value::Bool(lhs == rhs)
}
fn vm_neq(lhs: Value, rhs: Value) -> Value {
    Value::Bool(lhs != rhs)
}
fn vm_lt(lhs: Value, rhs: Value) -> Value {
    match (&lhs, &rhs) {
        (Value::I64(a), Value::I64(b)) => Value::Bool(a < b),
        (Value::F64(a), Value::F64(b)) => Value::Bool(a < b),
        (Value::U8(a), Value::U8(b)) => Value::Bool(a < b),
        _ => panic!("cannot compare {lhs:?} < {rhs:?}"),
    }
}
fn vm_gt(lhs: Value, rhs: Value) -> Value {
    match (&lhs, &rhs) {
        (Value::I64(a), Value::I64(b)) => Value::Bool(a > b),
        (Value::F64(a), Value::F64(b)) => Value::Bool(a > b),
        (Value::U8(a), Value::U8(b)) => Value::Bool(a > b),
        _ => panic!("cannot compare {lhs:?} > {rhs:?}"),
    }
}
fn vm_le(lhs: Value, rhs: Value) -> Value {
    match (&lhs, &rhs) {
        (Value::I64(a), Value::I64(b)) => Value::Bool(a <= b),
        (Value::F64(a), Value::F64(b)) => Value::Bool(a <= b),
        (Value::U8(a), Value::U8(b)) => Value::Bool(a <= b),
        _ => panic!("cannot compare {lhs:?} <= {rhs:?}"),
    }
}
fn vm_ge(lhs: Value, rhs: Value) -> Value {
    match (&lhs, &rhs) {
        (Value::I64(a), Value::I64(b)) => Value::Bool(a >= b),
        (Value::F64(a), Value::F64(b)) => Value::Bool(a >= b),
        (Value::U8(a), Value::U8(b)) => Value::Bool(a >= b),
        _ => panic!("cannot compare {lhs:?} >= {rhs:?}"),
    }
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

binary_op!(oxy_bitand, crate::vm::arith::vm_bitand);
binary_op!(oxy_bitor, crate::vm::arith::vm_bitor);
binary_op!(oxy_bitxor, crate::vm::arith::vm_bitxor);
binary_op!(oxy_shl, crate::vm::arith::vm_shl);
binary_op!(oxy_shr, crate::vm::arith::vm_shr);

// ── Unary ────────────────────────────────────────────────────────────

extern "C" fn oxy_neg(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let result = crate::vm::arith::vm_neg(val);
    unsafe {
        push(ctx, result);
    }
}

extern "C" fn oxy_not(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    unsafe {
        push(ctx, Value::Bool(!val.is_truthy()));
    }
}

extern "C" fn oxy_bitnot(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let result = crate::vm::arith::vm_bitnot(val);
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

/// Call stack for nested Oxy function invocations.
static CALL_STACK: std::sync::OnceLock<std::sync::Mutex<Vec<CallFrame>>> = std::sync::OnceLock::new();

struct CallFrame {
    return_ip: usize,
    caller_local_count: usize,
    caller_sp: usize,
}

/// Function pointer table: bytecode IP → native fn pointer (stored as usize).
static FN_TABLE: std::sync::OnceLock<std::sync::Mutex<HashMap<usize, usize>>> =
    std::sync::OnceLock::new();

fn call_stack_lock() -> std::sync::MutexGuard<'static, Vec<CallFrame>> {
    CALL_STACK
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .unwrap()
}

fn fn_table_lock() -> std::sync::MutexGuard<'static, HashMap<usize, usize>> {
    FN_TABLE
        .get_or_init(|| std::sync::Mutex::new(HashMap::new()))
        .lock()
        .unwrap()
}

pub(crate) fn set_fn_table(table: HashMap<usize, *const u8>) {
    let mut m = fn_table_lock();
    m.clear();
    for (ip, ptr) in table {
        m.insert(ip, ptr as usize);
    }
}

extern "C" fn oxy_call(ctx: *mut JitContext, target_ip: usize, _arg_count: usize) {
    let ctx = unsafe { &mut *ctx };
    let fn_ptr = {
        let table = fn_table_lock();
        table
            .get(&target_ip)
            .copied()
            .unwrap_or_else(|| panic!("JIT: no function at ip={target_ip}"))
    };

    // Save caller state
    {
        let mut call_stack = call_stack_lock();
        call_stack.push(CallFrame {
            return_ip: 0,
            caller_local_count: ctx.local_count,
            caller_sp: ctx.sp,
        });
    }

    // Call the JIT function
    let fn_ptr: extern "C" fn(*mut JitContext) -> u64 =
        unsafe { std::mem::transmute(fn_ptr as *const ()) };
    let _discriminant = fn_ptr(ctx);
}

// ── Return / panic ──────────────────────────────────────────────────

extern "C" fn oxy_return(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let result = unsafe { pop(ctx) };
    ctx.result = result;
    // Restore caller's sp? For now, the Cranelift return_ will handle exit.
}

extern "C" fn oxy_panic(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let msg_val = unsafe { pop(ctx) };
    let msg = format!("{msg_val:?}");
    let len = msg.len().min(1023);
    ctx.error_msg[..len].copy_from_slice(&msg.as_bytes()[..len]);
    ctx.error_len = len;
}

// ── Collections placeholders ────────────────────────────────────────

extern "C" fn oxy_make_array(_ctx: *mut JitContext, _count: usize) {}
extern "C" fn oxy_make_fixed_array(_ctx: *mut JitContext, _count: usize) {}
extern "C" fn oxy_make_tuple(_ctx: *mut JitContext, _count: usize) {}
extern "C" fn oxy_make_iter(_ctx: *mut JitContext) {}
extern "C" fn oxy_iter_len(_ctx: *mut JitContext) {}
extern "C" fn oxy_vec_index(_ctx: *mut JitContext) {}
extern "C" fn oxy_vec_index_store(_ctx: *mut JitContext) {}
extern "C" fn oxy_make_range(_ctx: *mut JitContext) {}
extern "C" fn oxy_to_string(_ctx: *mut JitContext) {}
extern "C" fn oxy_fstring_concat(_ctx: *mut JitContext, _count: usize) {}
extern "C" fn oxy_format(_ctx: *mut JitContext, _count: usize) {}
extern "C" fn oxy_struct_init(
    _ctx: *mut JitContext,
    _name_ptr: *const u8,
    _name_len: usize,
    _field_count: usize,
    _fnames_ptr: *const u8,
    _fnames_len: usize,
) {
}
extern "C" fn oxy_struct_update(_ctx: *mut JitContext, _field_count: usize) {}
extern "C" fn oxy_field_access(_ctx: *mut JitContext, _name_ptr: *const u8, _name_len: usize) {}
extern "C" fn oxy_field_store(_ctx: *mut JitContext, _name_ptr: *const u8, _name_len: usize) {}
extern "C" fn oxy_method_call(
    _ctx: *mut JitContext,
    _name_ptr: *const u8,
    _name_len: usize,
    _arg_count: usize,
) {
}
extern "C" fn oxy_try_pop(_ctx: *mut JitContext) {}
extern "C" fn oxy_cast_int(_ctx: *mut JitContext) {}
extern "C" fn oxy_cast_float(_ctx: *mut JitContext) {}
extern "C" fn oxy_cast_to_char(_ctx: *mut JitContext) {}
extern "C" fn oxy_bind_ident(_ctx: *mut JitContext, _index: usize) {}
extern "C" fn oxy_enum_data_get(_ctx: *mut JitContext, _index: usize) {}
extern "C" fn oxy_path_call_builtin(
    _ctx: *mut JitContext,
    _sptr: *const u8,
    _slen: usize,
    _arg_count: usize,
) {
}
extern "C" fn oxy_display_arg(_ctx: *mut JitContext) {}
extern "C" fn oxy_await_ffi(_ctx: *mut JitContext) -> u64 {
    0
}
extern "C" fn oxy_spawn_ffi(_ctx: *mut JitContext) {}
extern "C" fn oxy_sleep_ffi(_ctx: *mut JitContext) -> u64 {
    0
}
extern "C" fn oxy_select_ffi(_ctx: *mut JitContext, _count: usize) -> u64 {
    0
}

// ── Symbol registry ──────────────────────────────────────────────────

pub(crate) fn register_ffi_symbols(builder: &mut JITBuilder) {
    let syms: &[(&str, *const u8)] = &[
        ("oxy_push_unit", oxy_push_unit as _),
        ("oxy_push_bool", oxy_push_bool as _),
        ("oxy_push_int", oxy_push_int as _),
        ("oxy_push_float", oxy_push_float as _),
        ("oxy_push_char", oxy_push_char as _),
        ("oxy_push_string", oxy_push_string as _),
        ("oxy_pop", oxy_pop as _),
        ("oxy_dup", oxy_dup as _),
        ("oxy_load_local", oxy_load_local as _),
        ("oxy_store_local", oxy_store_local as _),
        ("oxy_make_cell", oxy_make_cell as _),
        ("oxy_print_val", oxy_print_val as _),
        ("oxy_println_val", oxy_println_val as _),
        ("oxy_add", oxy_add as _),
        ("oxy_sub", oxy_sub as _),
        ("oxy_mul", oxy_mul as _),
        ("oxy_div", oxy_div as _),
        ("oxy_mod", oxy_mod as _),
        ("oxy_eq", oxy_eq as _),
        ("oxy_neq", oxy_neq as _),
        ("oxy_lt", oxy_lt as _),
        ("oxy_gt", oxy_gt as _),
        ("oxy_le", oxy_le as _),
        ("oxy_ge", oxy_ge as _),
        ("oxy_and", oxy_and as _),
        ("oxy_or", oxy_or as _),
        ("oxy_bitand", oxy_bitand as _),
        ("oxy_bitor", oxy_bitor as _),
        ("oxy_bitxor", oxy_bitxor as _),
        ("oxy_shl", oxy_shl as _),
        ("oxy_shr", oxy_shr as _),
        ("oxy_neg", oxy_neg as _),
        ("oxy_not", oxy_not as _),
        ("oxy_bitnot", oxy_bitnot as _),
        ("oxy_is_falsy", oxy_is_falsy as _),
        ("oxy_is_truthy", oxy_is_truthy as _),
        ("oxy_call", oxy_call as _),
        ("oxy_return", oxy_return as _),
        ("oxy_panic", oxy_panic as _),
        ("oxy_make_array", oxy_make_array as _),
        ("oxy_make_fixed_array", oxy_make_fixed_array as _),
        ("oxy_make_tuple", oxy_make_tuple as _),
        ("oxy_make_iter", oxy_make_iter as _),
        ("oxy_iter_len", oxy_iter_len as _),
        ("oxy_vec_index", oxy_vec_index as _),
        ("oxy_vec_index_store", oxy_vec_index_store as _),
        ("oxy_make_range", oxy_make_range as _),
        ("oxy_to_string", oxy_to_string as _),
        ("oxy_fstring_concat", oxy_fstring_concat as _),
        ("oxy_format", oxy_format as _),
        ("oxy_struct_init", oxy_struct_init as _),
        ("oxy_struct_update", oxy_struct_update as _),
        ("oxy_field_access", oxy_field_access as _),
        ("oxy_field_store", oxy_field_store as _),
        ("oxy_method_call", oxy_method_call as _),
        ("oxy_try_pop", oxy_try_pop as _),
        ("oxy_cast_int", oxy_cast_int as _),
        ("oxy_cast_float", oxy_cast_float as _),
        ("oxy_cast_to_char", oxy_cast_to_char as _),
        ("oxy_bind_ident", oxy_bind_ident as _),
        ("oxy_enum_data_get", oxy_enum_data_get as _),
        ("oxy_path_call_builtin", oxy_path_call_builtin as _),
        ("oxy_display_arg", oxy_display_arg as _),
        ("oxy_await_ffi", oxy_await_ffi as _),
        ("oxy_spawn_ffi", oxy_spawn_ffi as _),
        ("oxy_sleep_ffi", oxy_sleep_ffi as _),
        ("oxy_select_ffi", oxy_select_ffi as _),
    ];

    for (name, ptr) in syms {
        builder.symbol(*name, *ptr);
    }
}
