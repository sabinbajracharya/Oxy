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

// ── Closure metadata ──────────────────────────────────────────────────

/// Runtime metadata for a single closure, stored globally for FFI access.
#[derive(Clone)]
struct ClosureRuntimeMeta {
    param_names: Vec<String>,
    captured: Vec<(String, usize, bool)>, // (name, outer_slot, is_mut)
    target_ip: usize,
    is_async: bool,
}

/// Closure metadata table: meta_idx → ClosureRuntimeMeta.
static CLOSURE_META: std::sync::OnceLock<std::sync::Mutex<Vec<ClosureRuntimeMeta>>> =
    std::sync::OnceLock::new();

fn closure_meta_lock() -> std::sync::MutexGuard<'static, Vec<ClosureRuntimeMeta>> {
    CLOSURE_META
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .unwrap()
}

pub(crate) fn set_closure_meta(
    meta: Vec<(Vec<String>, crate::ast::Expr, Vec<(String, usize, bool)>)>,
) {
    let mut lock = closure_meta_lock();
    lock.clear();
    for (param_names, _body_expr, captured) in meta {
        lock.push(ClosureRuntimeMeta {
            param_names,
            captured,
            target_ip: 0,    // populated by the compiler
            is_async: false, // overridden per-entry if needed
        });
    }
}

// ── Function calls ───────────────────────────────────────────────────

/// Call stack for nested Oxy function invocations.
static CALL_STACK: std::sync::OnceLock<std::sync::Mutex<Vec<CallFrame>>> =
    std::sync::OnceLock::new();

struct CallFrame {
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

extern "C" fn oxy_call(ctx: *mut JitContext, target_ip: usize, arg_count: usize) {
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
            caller_local_count: ctx.local_count,
            caller_sp: ctx.sp,
        });
    }

    // Move args from operand stack to start of buffer as callee locals.
    // Args are at buffer[local_count + sp - arg_count .. local_count + sp].
    // Callee expects them at buffer[0..arg_count].
    let args_start = ctx.sp - arg_count;
    for i in 0..arg_count {
        let src = unsafe { ctx.buffer.add(ctx.local_count + args_start + i).read() };
        unsafe { ctx.buffer.add(i).write(src) };
    }
    ctx.sp = args_start; // pop args from stack

    // Set up callee context
    let saved_local_count = ctx.local_count;
    ctx.local_count = arg_count;

    // Call the JIT function
    let fn_ptr: extern "C" fn(*mut JitContext) -> u64 =
        unsafe { std::mem::transmute(fn_ptr as *const ()) };
    let _discriminant = fn_ptr(ctx);

    // Restore caller context and push result
    let result = std::mem::replace(&mut ctx.result, Value::Unit);
    ctx.local_count = saved_local_count;
    unsafe {
        push(ctx, result);
    }

    // Restore stack depth
    let _call_stack = call_stack_lock();
}

// ── Closures ─────────────────────────────────────────────────────────

extern "C" fn oxy_push_closure(
    ctx: *mut JitContext,
    target_ip: usize,
    _param_count: usize,
    meta_idx: usize,
    is_async: u8,
) {
    let ctx = unsafe { &mut *ctx };
    let meta = {
        let lock = closure_meta_lock();
        lock.get(meta_idx).cloned()
    };
    let (_param_names, captured) = meta
        .map(|m| (m.param_names.clone(), m.captured.clone()))
        .unwrap_or_default();

    // Build captured values from current locals at the outer slots
    let closure_env = crate::env::Environment::new();
    for (name, outer_slot, is_mut) in &captured {
        let val = unsafe { ctx.buffer.add(*outer_slot).read() };
        let val = match &val {
            Value::Cell(rc) => rc.borrow().clone(),
            other => other.clone(),
        };
        closure_env.borrow_mut().define(name.clone(), val, *is_mut);
    }

    let captured_names: Vec<String> = captured.iter().map(|(n, _, _)| n.clone()).collect();
    let fn_data = crate::types::FunctionData {
        name: "<closure>".into(),
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
        target_ip: Some(target_ip),
        captured_names,
        is_async: is_async != 0,
    };
    unsafe {
        push(ctx, Value::Function(Box::new(fn_data)));
    }
}

extern "C" fn oxy_push_async_block(ctx: *mut JitContext, target_ip: usize, meta_idx: usize) {
    let ctx = unsafe { &mut *ctx };
    let meta = {
        let lock = closure_meta_lock();
        lock.get(meta_idx).cloned()
    };
    let captured = meta.map(|m| m.captured.clone()).unwrap_or_default();

    let closure_env = crate::env::Environment::new();
    for (name, outer_slot, is_mut) in &captured {
        let val = unsafe { ctx.buffer.add(*outer_slot).read() };
        let val = match &val {
            Value::Cell(rc) => rc.borrow().clone(),
            other => other.clone(),
        };
        closure_env.borrow_mut().define(name.clone(), val, *is_mut);
    }

    let captured_names: Vec<String> = captured.iter().map(|(n, _, _)| n.clone()).collect();
    let future_data = crate::types::FutureData {
        name: "<async_block>".into(),
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
        target_ip,
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
        _ => panic!("CallClosure: value is not a callable closure"),
    };

    let target_ip = target_ip.unwrap_or_else(|| panic!("CallClosure: no target_ip"));
    if target_ip == usize::MAX {
        panic!("CallClosure: invalid target_ip");
    }

    if is_async {
        // Create Future instead of executing
        // Pop closure + args, push Future
        let drain_start = ctx.sp - arg_count - 1;
        let mut args = Vec::new();
        for i in 0..arg_count {
            args.push(unsafe { ctx.buffer.add(ctx.local_count + drain_start + 1 + i).read() });
        }
        ctx.sp = drain_start; // pop everything

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
    let fn_ptr = {
        let table = fn_table_lock();
        table
            .get(&target_ip)
            .copied()
            .unwrap_or_else(|| panic!("JIT: no function for closure at ip={target_ip}"))
    };

    // Save caller state
    let saved_local_count = ctx.local_count;
    let saved_sp = ctx.sp;

    // Build callee frame: captures at [0..N], args at [N..N+arg_count]
    let captures_end = captured_names.len();
    let drain_start = ctx.sp - arg_count - 1;

    // Read args
    let mut args_vals = Vec::new();
    for i in 0..arg_count {
        args_vals.push(unsafe { ctx.buffer.add(ctx.local_count + drain_start + 1 + i).read() });
    }
    // Pop everything
    ctx.sp = drain_start;

    // Fill locals: captures first, then args
    let total_frame = captures_end + arg_count;
    // Grow buffer if needed
    while ctx.local_count + total_frame > ctx.capacity {
        // simple: just ensure enough room
        let new_cap = (ctx.local_count + total_frame) * 2;
        let new_layout = std::alloc::Layout::array::<Value>(new_cap).unwrap();
        let new_buf = unsafe { std::alloc::alloc_zeroed(new_layout) as *mut Value };
        unsafe {
            std::ptr::copy_nonoverlapping(ctx.buffer, new_buf, ctx.capacity);
        }
        unsafe {
            std::alloc::dealloc(
                ctx.buffer as *mut u8,
                std::alloc::Layout::array::<Value>(ctx.capacity).unwrap(),
            );
        }
        ctx.buffer = new_buf;
        ctx.capacity = new_cap;
    }
    ctx.local_count = total_frame;

    // Write captures
    for (i, name) in captured_names.iter().enumerate() {
        let val = closure_env.borrow().get(name).ok().unwrap_or(Value::Unit);
        unsafe {
            ctx.buffer.add(i).write(val);
        }
    }
    // Write args
    for (i, arg) in args_vals.into_iter().enumerate() {
        unsafe {
            ctx.buffer.add(captures_end + i).write(arg);
        }
    }

    ctx.sp = 0; // callee starts with empty stack

    // Call
    let fn_ptr: extern "C" fn(*mut JitContext) -> u64 =
        unsafe { std::mem::transmute(fn_ptr as *const ()) };
    let _discriminant = fn_ptr(ctx);

    // Restore and push result
    let result = std::mem::replace(&mut ctx.result, Value::Unit);
    ctx.local_count = saved_local_count;
    ctx.sp = saved_sp - arg_count - 1; // we popped closure + args
    unsafe {
        push(ctx, result);
    }
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
// ── Async runtime ────────────────────────────────────────────────────

/// Global scheduler for async task management.
static SCHEDULER: std::sync::OnceLock<std::sync::Mutex<crate::vm::scheduler::Scheduler>> =
    std::sync::OnceLock::new();

fn scheduler_lock() -> std::sync::MutexGuard<'static, crate::vm::scheduler::Scheduler> {
    SCHEDULER
        .get_or_init(|| std::sync::Mutex::new(crate::vm::scheduler::Scheduler::new()))
        .lock()
        .unwrap()
}

/// Build a JitTaskState from the current JitContext.
fn jit_state_from_ctx(ctx: &JitContext, resume_ip: usize) -> crate::vm::scheduler::JitTaskState {
    let mut locals = Vec::new();
    for i in 0..ctx.local_count {
        locals.push(unsafe { ctx.buffer.add(i).read() });
    }
    let mut operand_stack = Vec::new();
    for i in 0..ctx.sp {
        operand_stack.push(unsafe { ctx.buffer.add(ctx.local_count + i).read() });
    }
    crate::vm::scheduler::JitTaskState {
        resume_ip,
        locals,
        operand_stack,
        local_count: ctx.local_count,
        yield_reason: ctx.yield_reason,
        yield_data: ctx.yield_data,
    }
}

/// Restore JitContext from a JitTaskState.
fn ctx_from_jit_state(ctx: &mut JitContext, state: &crate::vm::scheduler::JitTaskState) {
    // Ensure buffer is large enough
    let needed = state.local_count + state.operand_stack.len();
    while ctx.capacity < needed {
        let new_cap = ctx.capacity * 2;
        let new_layout = std::alloc::Layout::array::<Value>(new_cap).unwrap();
        let new_buf = unsafe { std::alloc::alloc_zeroed(new_layout) as *mut Value };
        unsafe {
            std::ptr::copy_nonoverlapping(ctx.buffer, new_buf, ctx.capacity);
            std::alloc::dealloc(
                ctx.buffer as *mut u8,
                std::alloc::Layout::array::<Value>(ctx.capacity).unwrap(),
            );
        }
        ctx.buffer = new_buf;
        ctx.capacity = new_cap;
    }
    ctx.local_count = state.local_count;
    ctx.sp = state.operand_stack.len();
    ctx.resume_ip = state.resume_ip;
    ctx.yield_reason = state.yield_reason;
    ctx.yield_data = state.yield_data;
    for (i, v) in state.locals.iter().enumerate() {
        unsafe {
            ctx.buffer.add(i).write(v.clone());
        }
    }
    for (i, v) in state.operand_stack.iter().enumerate() {
        unsafe {
            ctx.buffer.add(state.local_count + i).write(v.clone());
        }
    }
}

extern "C" fn oxy_await_ffi(ctx: *mut JitContext) -> u64 {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };

    match val {
        Value::Future(fut) => {
            // Run the future's body synchronously
            let target_ip = fut.target_ip;
            let fn_ptr = {
                let table = fn_table_lock();
                *table
                    .get(&target_ip)
                    .unwrap_or_else(|| panic!("JIT: no function for future at ip={target_ip}"))
            };
            let saved_local_count = ctx.local_count;
            let saved_sp = ctx.sp;
            let captures_end = fut.captured_names.len();
            ctx.local_count = captures_end + fut.args.len();
            ctx.sp = 0;
            for (i, name) in fut.captured_names.iter().enumerate() {
                let v = fut
                    .closure_env
                    .borrow()
                    .get(name)
                    .ok()
                    .unwrap_or(Value::Unit);
                unsafe {
                    ctx.buffer.add(i).write(v);
                }
            }
            for (i, arg) in fut.args.iter().enumerate() {
                unsafe {
                    ctx.buffer.add(captures_end + i).write(arg.clone());
                }
            }
            let fn_ptr: extern "C" fn(*mut JitContext) -> u64 =
                unsafe { std::mem::transmute(fn_ptr as *const ()) };
            let disc = fn_ptr(ctx);
            if disc != 0 {
                ctx.local_count = saved_local_count;
                ctx.sp = saved_sp;
                unsafe {
                    push(ctx, Value::Future(fut));
                }
                return disc;
            }
            let result = std::mem::replace(&mut ctx.result, Value::Unit);
            ctx.local_count = saved_local_count;
            ctx.sp = saved_sp;
            unsafe {
                push(ctx, result);
            }
            0
        }
        Value::JoinHandle { task_id } => {
            let result = scheduler_lock().task_result(task_id);
            if let Some(v) = result {
                unsafe {
                    push(ctx, v);
                }
                return 0;
            }
            // Not done — yield the current JIT task
            ctx.yield_reason = 2;
            ctx.yield_data = task_id as u64;
            let jit_state = jit_state_from_ctx(ctx, ctx.resume_ip);
            scheduler_lock().yield_jit_for_task(task_id, jit_state);
            unsafe {
                push(ctx, Value::JoinHandle { task_id });
            }
            1
        }
        other => {
            unsafe {
                push(ctx, other);
            }
            0
        }
    }
}

extern "C" fn oxy_spawn_ffi(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let closure = unsafe { pop(ctx) };

    match closure {
        Value::Function(f) => {
            let target_ip = f.target_ip.unwrap_or(0);
            let capture_count = f.captured_names.len();
            let mut locals = Vec::with_capacity(capture_count);
            for name in &f.captured_names {
                locals.push(f.closure_env.borrow().get(name).ok().unwrap_or(Value::Unit));
            }
            let jit_state = crate::vm::scheduler::JitTaskState {
                resume_ip: target_ip,
                locals,
                operand_stack: vec![],
                local_count: capture_count,
                yield_reason: 0,
                yield_data: 0,
            };
            let mut sched = scheduler_lock();
            let task_id = sched.create_task();
            sched.save_new_task(
                task_id,
                crate::vm::scheduler::TaskSnapshot {
                    ip: target_ip,
                    stack: vec![],
                    call_stack: vec![],
                    jit_state: Some(jit_state),
                },
            );
            drop(sched);
            unsafe {
                push(ctx, Value::JoinHandle { task_id });
            }
        }
        _ => panic!("spawn requires a closure"),
    }
}

extern "C" fn oxy_sleep_ffi(ctx: *mut JitContext) -> u64 {
    let ctx = unsafe { &mut *ctx };
    let ms_val = unsafe { pop(ctx) };
    let ms = match ms_val {
        Value::I64(n) => n as u64,
        Value::U8(n) => n as u64,
        _ => 0,
    };
    if ms == 0 {
        unsafe {
            push(ctx, Value::Unit);
        }
        return 0;
    }
    let wake = crate::vm::scheduler::delay_from_now(ms);
    ctx.yield_reason = 1;
    ctx.yield_data = ms;
    let jit_state = jit_state_from_ctx(ctx, ctx.resume_ip);
    scheduler_lock().yield_jit_for_timer(jit_state, wake);
    1
}

extern "C" fn oxy_select_ffi(ctx: *mut JitContext, count: usize) -> u64 {
    let ctx = unsafe { &mut *ctx };
    let mut task_ids = Vec::new();
    for _ in 0..count {
        let val = unsafe { pop(ctx) };
        if let Value::JoinHandle { task_id } = val {
            task_ids.push(task_id);
        }
    }
    {
        let sched = scheduler_lock();
        for &tid in &task_ids {
            if let Some(v) = sched.task_result(tid) {
                drop(sched);
                unsafe {
                    push(ctx, v);
                }
                return 0;
            }
        }
    }
    // None ready — yield
    ctx.yield_reason = 3;
    ctx.yield_data = task_ids.first().copied().unwrap_or(0) as u64;
    let jit_state = jit_state_from_ctx(ctx, ctx.resume_ip);
    scheduler_lock().yield_jit_for_multiple(task_ids.clone(), jit_state);
    for &tid in task_ids.iter().rev() {
        unsafe {
            push(ctx, Value::JoinHandle { task_id: tid });
        }
    }
    1
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
        ("oxy_push_closure", oxy_push_closure as _),
        ("oxy_push_async_block", oxy_push_async_block as _),
        ("oxy_call_closure", oxy_call_closure as _),
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
