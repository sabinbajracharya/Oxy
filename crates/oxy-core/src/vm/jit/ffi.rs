//! FFI bridge: Rust functions callable from JIT-compiled code.
//!
//! All functions use `extern "C"` ABI and operate on `*mut JitContext`.
//!
//! The JitContext buffer layout:
//! ```text
//! [locals: local_count × Value] [operand stack: sp × Value]
//! ```

/// Set ctx.result directly (bypasses operand stack for simple returns).
#[no_mangle]
extern "C" fn oxy_set_result_i64(ctx: *mut super::JitContext, val: i64) {
    let ctx = unsafe { &mut *ctx };
    ctx.result = crate::types::Value::I64(val);
}

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

/// Write an error message to the context, replacing any existing error.
fn set_error(ctx: &mut JitContext, msg: String) {
    let len = msg.len().min(1023);
    ctx.error_msg[..len].copy_from_slice(&msg.as_bytes()[..len]);
    ctx.error_len = len;
}

unsafe fn pop(ctx: &mut JitContext) -> Value {
    if ctx.sp == 0 {
        return Value::Unit;
    }
    ctx.sp -= 1;
    let ptr = unsafe { ctx.buffer.add(ctx.local_count + ctx.sp) };
    let val = unsafe { ptr.read() };
    // Zero the slot so the original Value isn't dropped later by JitContext::drop.
    // Without this, heap-allocated fields (String, Vec) would be double-freed:
    // once when the popped copy drops, once when JitContext::drop cleans up.
    unsafe { ptr.write(Value::Unit) };
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
    let cell = Value::Cell(std::rc::Rc::new(std::cell::RefCell::new(val)));
    // val was a shallow copy; it has been moved into the Rc so it won't Drop.
    // The original in the buffer is overwritten by write() below (write does
    // not drop the old value, so no double-free).
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
    if !ctx.output.is_null() {
        let output = unsafe { &*ctx.output };
        output.borrow_mut().push(format!("{val}\n"));
    } else {
        println!("{val}");
    }
}

// ── Binary arithmetic ────────────────────────────────────────────────

macro_rules! binary_op {
    ($name:ident, $func:path) => {
        extern "C" fn $name(ctx: *mut JitContext) {
            let ctx = unsafe { &mut *ctx };
            let rhs = unsafe { pop(ctx) };
            let lhs = unsafe { pop(ctx) };
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
        _ => Value::Bool(false),
    }
}
fn vm_gt(lhs: Value, rhs: Value) -> Value {
    match (&lhs, &rhs) {
        (Value::I64(a), Value::I64(b)) => Value::Bool(a > b),
        (Value::F64(a), Value::F64(b)) => Value::Bool(a > b),
        (Value::U8(a), Value::U8(b)) => Value::Bool(a > b),
        _ => Value::Bool(false),
    }
}
fn vm_le(lhs: Value, rhs: Value) -> Value {
    match (&lhs, &rhs) {
        (Value::I64(a), Value::I64(b)) => Value::Bool(a <= b),
        (Value::F64(a), Value::F64(b)) => Value::Bool(a <= b),
        (Value::U8(a), Value::U8(b)) => Value::Bool(a <= b),
        _ => Value::Bool(false),
    }
}
fn vm_ge(lhs: Value, rhs: Value) -> Value {
    match (&lhs, &rhs) {
        (Value::I64(a), Value::I64(b)) => Value::Bool(a >= b),
        (Value::F64(a), Value::F64(b)) => Value::Bool(a >= b),
        (Value::U8(a), Value::U8(b)) => Value::Bool(a >= b),
        _ => Value::Bool(false),
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
    captured: Vec<(String, usize, bool)>,
    target_ip: usize,
    is_async: bool,
}

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
            target_ip: 0,
            is_async: false,
        });
    }
}

// ── Async function metadata ───────────────────────────────────────────

struct AsyncFnMeta {
    name: String,
    params: Vec<crate::ast::Param>,
    return_type: Option<crate::ast::TypeAnnotation>,
    body: Box<crate::ast::Block>,
    target_ip: usize,
}
unsafe impl Send for AsyncFnMeta {}
unsafe impl Sync for AsyncFnMeta {}

static ASYNC_FN_META: std::sync::OnceLock<std::sync::Mutex<Vec<AsyncFnMeta>>> =
    std::sync::OnceLock::new();

fn async_fn_meta_lock() -> std::sync::MutexGuard<'static, Vec<AsyncFnMeta>> {
    ASYNC_FN_META
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .unwrap()
}

pub(crate) fn set_async_fn_meta(
    meta: Vec<(
        String,
        Vec<crate::ast::Param>,
        Option<crate::ast::TypeAnnotation>,
        crate::ast::Block,
        usize,
    )>,
) {
    let mut lock = async_fn_meta_lock();
    lock.clear();
    for (name, params, return_type, body, target_ip) in meta {
        lock.push(AsyncFnMeta {
            name,
            params,
            return_type,
            body: Box::new(body),
            target_ip,
        });
    }
}

// ── Builtin path table ────────────────────────────────────────────────

/// Global registry of PathCallBuiltin segment lists, keyed by index.
static BUILTIN_PATHS: std::sync::OnceLock<std::sync::Mutex<Vec<Vec<String>>>> =
    std::sync::OnceLock::new();

fn builtin_paths_lock() -> std::sync::MutexGuard<'static, Vec<Vec<String>>> {
    BUILTIN_PATHS
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .unwrap()
}

pub(crate) fn register_builtin_path(segments: Vec<String>) -> usize {
    let mut lock = builtin_paths_lock();
    let idx = lock.len();
    lock.push(segments);
    idx
}

// ── Struct init registry ──────────────────────────────────────────────

/// Struct init metadata stored in a global registry.
struct StructInitMeta {
    name: String,
    field_names: Vec<String>,
    field_count: usize,
}

static STRUCT_INIT_META: std::sync::OnceLock<std::sync::Mutex<Vec<StructInitMeta>>> =
    std::sync::OnceLock::new();

fn struct_init_meta_lock() -> std::sync::MutexGuard<'static, Vec<StructInitMeta>> {
    STRUCT_INIT_META
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .unwrap()
}

pub(crate) fn register_struct_init(
    name: String,
    field_names: Vec<String>,
    field_count: usize,
) -> usize {
    let mut lock = struct_init_meta_lock();
    let idx = lock.len();
    lock.push(StructInitMeta {
        name,
        field_names,
        field_count,
    });
    idx
}

// ── Const enum variant registry ──────────────────────────────────────
//
// Uses a Box behind a newtype that manually implements Send+Sync because
// Value contains non-thread-safe types (Rc, RefCell) but we only access
// them from a single thread (JIT compilation + execution are sequential).

struct ConstEnumVariantStore(Vec<(String, String, Vec<Value>)>);
unsafe impl Send for ConstEnumVariantStore {}
unsafe impl Sync for ConstEnumVariantStore {}

static CONST_ENUM_VARIANTS: std::sync::OnceLock<std::sync::Mutex<ConstEnumVariantStore>> =
    std::sync::OnceLock::new();

fn const_enum_variants_lock() -> std::sync::MutexGuard<'static, ConstEnumVariantStore> {
    CONST_ENUM_VARIANTS
        .get_or_init(|| std::sync::Mutex::new(ConstEnumVariantStore(Vec::new())))
        .lock()
        .unwrap()
}

pub(crate) fn register_const_enum_variant(
    enum_name: String,
    variant: String,
    data: Vec<Value>,
) -> usize {
    let mut lock = const_enum_variants_lock();
    let idx = lock.0.len();
    lock.0.push((enum_name, variant, data));
    idx
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

pub(super) fn lookup_fn_ptr(ip: usize) -> Option<*const u8> {
    fn_table_lock().get(&ip).map(|&p| p as *const u8)
}

pub(crate) fn set_fn_table(table: HashMap<usize, *const u8>) {
    let mut m = fn_table_lock();
    m.clear();
    for (ip, ptr) in table {
        m.insert(ip, ptr as usize);
    }
}

/// Method IPs: (type_name, method_name) → bytecode IP
static METHOD_IPS: std::sync::OnceLock<std::sync::Mutex<HashMap<(String, String), usize>>> =
    std::sync::OnceLock::new();

fn method_ips_lock() -> std::sync::MutexGuard<'static, HashMap<(String, String), usize>> {
    METHOD_IPS
        .get_or_init(|| std::sync::Mutex::new(HashMap::new()))
        .lock()
        .unwrap()
}

pub(crate) fn set_method_ips(ips: HashMap<(String, String), usize>) {
    let mut m = method_ips_lock();
    m.clear();
    m.extend(ips);
}

extern "C" fn oxy_call(ctx: *mut JitContext, target_ip: usize, arg_count: usize) {
    let ctx = unsafe { &mut *ctx };
    let fn_ptr = {
        let table = fn_table_lock();
        match table.get(&target_ip).copied() {
            Some(p) => p,
            None => {
                set_error(ctx, format!("JIT: no function at ip={target_ip}"));
                return;
            }
        }
    };

    // Save caller state on the call stack
    {
        let mut call_stack = call_stack_lock();
        call_stack.push(CallFrame {
            caller_local_count: ctx.local_count,
            caller_sp: ctx.sp,
        });
    }

    // Create a separate buffer for the callee to avoid corrupting
    // the caller's locals.
    let callee_cap = arg_count + 256;
    let callee_layout = std::alloc::Layout::array::<Value>(callee_cap).unwrap();
    let callee_buf = unsafe { std::alloc::alloc_zeroed(callee_layout) as *mut Value };

    // Move args from the caller's operand stack to the callee's buffer
    let args_start = ctx.sp - arg_count;
    for i in 0..arg_count {
        let src = unsafe { ctx.buffer.add(ctx.local_count + args_start + i).read() };
        unsafe {
            callee_buf.add(i).write(src);
        }
    }
    ctx.sp = args_start; // pop args from caller's stack

    // Swap to callee's buffer
    let saved_buffer = ctx.buffer;
    let saved_capacity = ctx.capacity;
    let saved_local_count = ctx.local_count;
    ctx.buffer = callee_buf;
    ctx.capacity = callee_cap;
    ctx.local_count = arg_count;

    // Call the JIT function
    let fn_ptr: extern "C" fn(*mut JitContext) -> u64 =
        unsafe { std::mem::transmute(fn_ptr as *const ()) };
    let _discriminant = fn_ptr(ctx);

    // Drop callee's locals and stack, free callee buffer
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

    // Restore caller's buffer and push result
    let result = std::mem::replace(&mut ctx.result, Value::Unit);
    ctx.buffer = saved_buffer;
    ctx.capacity = saved_capacity;
    ctx.local_count = saved_local_count;
    unsafe {
        push(ctx, result);
    }
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
        let shallow = unsafe { ctx.buffer.add(*outer_slot).read() };
        let val = match &shallow {
            Value::Cell(rc) => rc.borrow().clone(),
            other => other.clone(),
        };
        // shallow is a bitwise copy that shares heap pointers with
        // the original in the buffer. Forget it to prevent a double-free
        // when the shallow copy is dropped at end of scope.
        std::mem::forget(shallow);
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
        let shallow = unsafe { ctx.buffer.add(*outer_slot).read() };
        let val = match &shallow {
            Value::Cell(rc) => rc.borrow().clone(),
            other => other.clone(),
        };
        std::mem::forget(shallow);
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
        match table.get(&target_ip).copied() {
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
        }
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

    // Create a separate buffer for the callee so the caller's locals
    // (at buffer[0..saved_local_count]) are never touched.
    let total_frame = captures_end + arg_count;
    let callee_cap = total_frame + 256; // small buffer for callee
    let callee_layout = std::alloc::Layout::array::<Value>(callee_cap).unwrap();
    let callee_buf = unsafe { std::alloc::alloc_zeroed(callee_layout) as *mut Value };

    // Write captures and args into the callee's buffer
    for (i, name) in captured_names.iter().enumerate() {
        let val = closure_env.borrow().get(name).ok().unwrap_or(Value::Unit);
        unsafe {
            callee_buf.add(i).write(val);
        }
    }
    for (i, arg) in args_vals.into_iter().enumerate() {
        unsafe {
            callee_buf.add(captures_end + i).write(arg);
        }
    }

    // Save caller's buffer info, swap to callee's buffer
    let saved_buffer = ctx.buffer;
    let saved_capacity = ctx.capacity;
    ctx.buffer = callee_buf;
    ctx.capacity = callee_cap;
    ctx.local_count = total_frame;
    ctx.sp = 0;

    // Call
    let fn_ptr: extern "C" fn(*mut JitContext) -> u64 =
        unsafe { std::mem::transmute(fn_ptr as *const ()) };
    let _discriminant = fn_ptr(ctx);

    // Drop callee's locals and stack, then free callee's buffer
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

    // Restore caller's buffer
    ctx.buffer = saved_buffer;
    ctx.capacity = saved_capacity;

    // Restore and push result
    let result = std::mem::replace(&mut ctx.result, Value::Unit);
    ctx.local_count = saved_local_count;
    ctx.sp = saved_sp - arg_count - 1; // we popped closure + args
    unsafe {
        push(ctx, result);
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

// ── Collections ──────────────────────────────────────────────────────

extern "C" fn oxy_make_array(ctx: *mut JitContext, count: usize) {
    let ctx = unsafe { &mut *ctx };
    let mut elements = Vec::with_capacity(count);
    for _ in 0..count {
        elements.push(unsafe { pop(ctx) });
    }
    elements.reverse();
    unsafe {
        push(
            ctx,
            Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(elements))),
        );
    }
}

extern "C" fn oxy_make_fixed_array(ctx: *mut JitContext, count: usize) {
    let ctx = unsafe { &mut *ctx };
    let mut elements = Vec::with_capacity(count);
    for _ in 0..count {
        elements.push(unsafe { pop(ctx) });
    }
    elements.reverse();
    unsafe {
        push(ctx, Value::Array(elements));
    }
}

extern "C" fn oxy_make_tuple(ctx: *mut JitContext, count: usize) {
    let ctx = unsafe { &mut *ctx };
    let mut elements = Vec::with_capacity(count);
    for _ in 0..count {
        elements.push(unsafe { pop(ctx) });
    }
    elements.reverse();
    unsafe {
        push(ctx, Value::Tuple(elements));
    }
}

extern "C" fn oxy_make_repeat(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let count_val = unsafe { pop(ctx) };
    let value = unsafe { pop(ctx) };
    let count = match &count_val {
        Value::I64(n) => *n as usize,
        Value::U8(n) => *n as usize,
        _ => {
            set_error(
                ctx,
                format!("repeat count must be integer, got {count_val:?}"),
            );
            unsafe {
                push(ctx, Value::Unit);
            }
            return;
        }
    };
    let mut elements = Vec::with_capacity(count);
    for _ in 0..count {
        elements.push(value.clone());
    }
    unsafe {
        push(
            ctx,
            Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(elements))),
        );
    }
}

extern "C" fn oxy_iter_next_destructure(ctx: *mut JitContext, state_slot: usize) {
    // Like oxy_iter_next, but stores each destructured element to local slots
    // state_slot..state_slot+n. The element is also pushed to the stack for truthiness check.
    let ctx = unsafe { &mut *ctx };
    let target_ptr = unsafe { ctx.buffer.add(state_slot) };
    let target = unsafe { &*target_ptr };

    let (vec_clone, index) = match target {
        Value::Tuple(ref elements) if elements.len() >= 2 => (
            elements[0].clone(),
            match &elements[1] {
                Value::I64(n) => *n as usize,
                Value::U8(n) => *n as usize,
                _ => 0,
            },
        ),
        _ => {
            unsafe {
                push(ctx, Value::Unit);
            }
            return;
        }
    };

    let len = match &vec_clone {
        Value::Vec(rc) => rc.borrow().len(),
        Value::Array(a) => a.len(),
        _ => 0,
    };

    if index < len {
        let elem = match &vec_clone {
            Value::Vec(rc) => rc.borrow().get(index).cloned().unwrap_or(Value::Unit),
            Value::Array(a) => a.get(index).cloned().unwrap_or(Value::Unit),
            _ => Value::Unit,
        };
        // Store destructured bindings: each element of the tuple to state_slot+i
        if let Value::Tuple(ref fields) = elem {
            for (i, field) in fields.iter().enumerate() {
                let dest_ptr = unsafe { ctx.buffer.add(state_slot + 1 + i) };
                unsafe {
                    std::ptr::drop_in_place(dest_ptr);
                }
                unsafe {
                    dest_ptr.write(field.clone());
                }
            }
        }
        unsafe {
            std::ptr::drop_in_place(target_ptr);
            target_ptr.write(Value::Tuple(vec![
                vec_clone,
                Value::I64((index + 1) as i64),
            ]));
            push(ctx, elem);
        }
    } else {
        unsafe {
            push(ctx, Value::Unit);
        }
    }
}

// ── Iteration ─────────────────────────────────────────────────────────

extern "C" fn oxy_make_iter(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let result = match val.into_iterable() {
        Ok(vec) => {
            // Store (Vec, index: 0) as a tuple to track iteration state.
            Value::Tuple(vec![
                Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(vec))),
                Value::I64(0),
            ])
        }
        Err(e) => {
            set_error(ctx, e);
            Value::Tuple(vec![Value::Unit, Value::I64(0)])
        }
    };
    unsafe {
        push(ctx, result);
    }
}

extern "C" fn oxy_iter_len(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let len = match &val {
        Value::Tuple(elements) if elements.len() >= 2 => match &elements[0] {
            Value::Vec(rc) => rc.borrow().len() as i64,
            Value::String(s) => s.len() as i64,
            Value::Array(a) => a.len() as i64,
            _ => {
                set_error(
                    ctx,
                    format!("cannot get length of {}", elements[0].type_name()),
                );
                0i64
            }
        },
        Value::Vec(rc) => rc.borrow().len() as i64,
        Value::String(s) => s.len() as i64,
        Value::Array(a) => a.len() as i64,
        _ => {
            set_error(ctx, format!("cannot get length of {}", val.type_name()));
            0i64
        }
    };
    unsafe {
        push(ctx, Value::I64(len));
    }
}

extern "C" fn oxy_iter_next(ctx: *mut JitContext, state_slot: usize) {
    let ctx = unsafe { &mut *ctx };
    let target_ptr = unsafe { ctx.buffer.add(state_slot) };
    let target = unsafe { &*target_ptr };

    let (vec_clone, index) = match target {
        Value::Tuple(ref elements) if elements.len() >= 2 => (
            elements[0].clone(),
            match &elements[1] {
                Value::I64(n) => *n as usize,
                Value::U8(n) => *n as usize,
                _ => 0,
            },
        ),
        _ => {
            unsafe {
                push(ctx, Value::Unit);
            }
            return;
        }
    };

    let len = match &vec_clone {
        Value::Vec(rc) => rc.borrow().len(),
        Value::Array(a) => a.len(),
        Value::String(s) => s.chars().count(),
        _ => 0,
    };

    if index < len {
        let elem = match &vec_clone {
            Value::Vec(rc) => rc.borrow().get(index).cloned().unwrap_or(Value::Unit),
            Value::Array(a) => a.get(index).cloned().unwrap_or(Value::Unit),
            Value::String(s) => {
                let c = s.chars().nth(index).unwrap_or('\0');
                Value::Char(c)
            }
            _ => Value::Unit,
        };
        unsafe {
            std::ptr::drop_in_place(target_ptr);
            target_ptr.write(Value::Tuple(vec![
                vec_clone,
                Value::I64((index + 1) as i64),
            ]));
            push(ctx, elem);
        }
    } else {
        unsafe {
            push(ctx, Value::Unit);
        }
    }
}

extern "C" fn oxy_vec_index(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let index_val = unsafe { pop(ctx) };
    let collection = unsafe { pop(ctx) };
    let idx = crate::vm::arith::value_to_i64(&index_val) as usize;
    let result = match collection {
        Value::Vec(rc) => rc.borrow().get(idx).cloned().unwrap_or(Value::Unit),
        Value::Array(ref a) => a.get(idx).cloned().unwrap_or(Value::Unit),
        Value::String(ref s) => {
            let c = s.chars().nth(idx).unwrap_or('\0');
            Value::Char(c)
        }
        Value::Tuple(ref t) => t.get(idx).cloned().unwrap_or(Value::Unit),
        _ => {
            set_error(ctx, format!("cannot index {collection:?}"));
            Value::Unit
        }
    };
    unsafe {
        push(ctx, result);
    }
}

extern "C" fn oxy_vec_index_store(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let value = unsafe { pop(ctx) };
    let index_val = unsafe { pop(ctx) };
    let collection = unsafe { pop(ctx) };
    let idx = crate::vm::arith::value_to_i64(&index_val) as usize;
    match collection {
        Value::Vec(rc) => {
            let mut v = rc.borrow_mut();
            if idx < v.len() {
                v[idx] = value.clone();
            }
        }
        _ => {
            set_error(ctx, format!("cannot index-store {collection:?}"));
            unsafe {
                push(ctx, value);
            }
            return;
        }
    }
    unsafe {
        push(ctx, value);
    }
}

extern "C" fn oxy_make_range(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let end = unsafe { pop(ctx) };
    let start = unsafe { pop(ctx) };
    let s = match &start {
        Value::I64(n) => *n,
        Value::U8(n) => *n as i64,
        _ => {
            set_error(ctx, format!("range start must be integer, got {start:?}"));
            unsafe {
                push(ctx, Value::Unit);
            }
            return;
        }
    };
    let e = match &end {
        Value::I64(n) => *n,
        Value::U8(n) => *n as i64,
        _ => {
            set_error(ctx, format!("range end must be integer, got {end:?}"));
            unsafe {
                push(ctx, Value::Unit);
            }
            return;
        }
    };
    unsafe {
        push(ctx, Value::Range(s, e));
    }
}

// ── String operations ─────────────────────────────────────────────────

extern "C" fn oxy_to_string(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let s = val.to_string();
    unsafe {
        push(ctx, Value::String(s));
    }
}

extern "C" fn oxy_fstring_concat(ctx: *mut JitContext, count: usize) {
    let ctx = unsafe { &mut *ctx };
    let mut parts = Vec::with_capacity(count);
    for _ in 0..count {
        parts.push(unsafe { pop(ctx) });
    }
    parts.reverse();
    let result: String = parts.iter().map(|v| v.to_string()).collect();
    unsafe {
        push(ctx, Value::String(result));
    }
}

extern "C" fn oxy_format(ctx: *mut JitContext, count: usize) {
    let ctx = unsafe { &mut *ctx };
    let mut vals = Vec::with_capacity(count);
    for _ in 0..count {
        vals.push(unsafe { pop(ctx) });
    }
    vals.reverse();
    if vals.is_empty() {
        unsafe {
            push(ctx, Value::String(String::new()));
        }
        return;
    }
    let template = vals[0].to_string();
    if count == 1 {
        unsafe {
            push(ctx, Value::String(template));
        }
        return;
    }
    let mut result = String::new();
    let mut arg_idx = 1;
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' && chars.peek() == Some(&'}') {
            chars.next();
            if arg_idx < vals.len() {
                result.push_str(&vals[arg_idx].to_string());
            }
            arg_idx += 1;
        } else {
            result.push(c);
        }
    }
    unsafe {
        push(ctx, Value::String(result));
    }
}

// ── Structs ───────────────────────────────────────────────────────────

extern "C" fn oxy_struct_init(
    ctx: *mut JitContext,
    name_ptr: *const u8,
    name_len: usize,
    field_names_ptr: *const u8,
    field_names_len: usize,
    field_count: usize,
) {
    let ctx = unsafe { &mut *ctx };
    let name_bytes = unsafe { std::slice::from_raw_parts(name_ptr, name_len) };
    let name = String::from_utf8_lossy(name_bytes).into_owned();
    let fn_bytes = unsafe { std::slice::from_raw_parts(field_names_ptr, field_names_len) };
    let field_names: Vec<&str> = fn_bytes
        .split(|b| *b == 0)
        .map(|s| std::str::from_utf8(s).unwrap_or(""))
        .collect();
    let mut fields = HashMap::new();
    for i in (0..field_count).rev() {
        let val = unsafe { pop(ctx) };
        let fname = field_names
            .get(i)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("_f{i}"));
        fields.insert(fname, val);
    }
    unsafe {
        push(ctx, Value::Struct { name, fields });
    }
}

extern "C" fn oxy_struct_update(
    ctx: *mut JitContext,
    field_names_ptr: *const u8,
    field_names_len: usize,
    field_count: usize,
) {
    let ctx = unsafe { &mut *ctx };
    let base = unsafe { pop(ctx) };
    let fn_bytes = unsafe { std::slice::from_raw_parts(field_names_ptr, field_names_len) };
    let field_names: Vec<&str> = fn_bytes
        .split(|b| *b == 0)
        .map(|s| std::str::from_utf8(s).unwrap_or(""))
        .collect();
    let mut overrides = Vec::with_capacity(field_count);
    for _ in 0..field_count {
        overrides.push(unsafe { pop(ctx) });
    }
    overrides.reverse();
    match base {
        Value::Struct { name, fields } => {
            let mut new_fields = fields.clone();
            for (i, val) in overrides.into_iter().enumerate() {
                let fname = field_names
                    .get(i)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("_f{i}"));
                new_fields.insert(fname, val);
            }
            unsafe {
                push(
                    ctx,
                    Value::Struct {
                        name,
                        fields: new_fields,
                    },
                );
            }
        }
        _ => {
            set_error(ctx, format!("struct update on non-struct: {base:?}"));
            unsafe {
                push(ctx, base);
            }
        }
    }
}

extern "C" fn oxy_field_access(ctx: *mut JitContext, name_ptr: *const u8, name_len: usize) {
    let ctx = unsafe { &mut *ctx };
    let obj = unsafe { pop(ctx) };
    let name_bytes = unsafe { std::slice::from_raw_parts(name_ptr, name_len) };
    let name = String::from_utf8_lossy(name_bytes);
    let result = match &obj {
        Value::Struct { fields, .. } => fields.get(name.as_ref()).cloned().unwrap_or(Value::Unit),
        _ => {
            set_error(ctx, format!("field access on non-struct: {obj:?}"));
            unsafe {
                push(ctx, Value::Unit);
            }
            return;
        }
    };
    unsafe {
        push(ctx, result);
    }
}

extern "C" fn oxy_field_store(ctx: *mut JitContext, name_ptr: *const u8, name_len: usize) {
    let ctx = unsafe { &mut *ctx };
    let value = unsafe { pop(ctx) };
    let obj = unsafe { pop(ctx) };
    let name_bytes = unsafe { std::slice::from_raw_parts(name_ptr, name_len) };
    let name = String::from_utf8_lossy(name_bytes);
    match obj {
        Value::Struct {
            name: sname,
            mut fields,
        } => {
            fields.insert(name.into_owned(), value);
            unsafe {
                push(
                    ctx,
                    Value::Struct {
                        name: sname,
                        fields,
                    },
                );
            }
        }
        _ => {
            set_error(ctx, format!("field store on non-struct: {obj:?}"));
            unsafe {
                push(ctx, obj);
            }
        }
    }
}

// ── Method dispatch ───────────────────────────────────────────────────

/// JIT-compatible closure invoker callback. Matches the signature
/// `Fn(&Value, &[Value]) -> Result<Value, String>` used by builtins.
fn jit_closure_invoker(func: &Value, args: &[Value]) -> Result<Value, String> {
    let ft = match func {
        Value::Function(f) => f.clone(),
        _ => return Err("not a callable function".into()),
    };
    let target_ip = ft.target_ip.ok_or("function has no target_ip")?;
    let fn_ptr = {
        let table = fn_table_lock();
        table
            .get(&target_ip)
            .copied()
            .ok_or(format!("JIT: no function for closure at ip={target_ip}"))?
    };

    // Build a temporary JitContext for the closure call
    let captures_end = ft.captured_names.len();
    let frame_size = captures_end + args.len();
    let mut call_ctx = JitContext::new(frame_size);
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
    call_ctx.local_count = frame_size;

    let fn_ptr: extern "C" fn(*mut JitContext) -> u64 =
        unsafe { std::mem::transmute(fn_ptr as *const ()) };
    let disc = fn_ptr(&mut call_ctx as *mut JitContext);
    if disc == 0 {
        Ok(std::mem::replace(&mut call_ctx.result, Value::Unit))
    } else {
        Err(String::from_utf8_lossy(&call_ctx.error_msg[..call_ctx.error_len]).into_owned())
    }
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

    // Determine lookup name from receiver type (like old VM does)
    let type_name = receiver.type_name().to_string();
    let lookup_name = match &receiver {
        Value::Struct { name, .. } => name.clone(),
        Value::EnumVariant { enum_name, .. } => enum_name.clone(),
        _ => type_name,
    };

    // First try direct method dispatch (for struct/impl methods)
    let method_ip = method_ips_lock()
        .get(&(lookup_name.clone(), method_name.to_string()))
        .copied();
    if let Some(target_ip) = method_ip {
        let fn_ptr = {
            let table = fn_table_lock();
            table.get(&target_ip).copied()
        };
        if let Some(fp) = fn_ptr {
            // Build callee frame: locals = [receiver, args...]
            let frame_size = 1 + arg_count;
            while ctx.local_count + frame_size > ctx.capacity {
                let new_cap = (ctx.local_count + frame_size) * 2;
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
            let saved_local_count = ctx.local_count;
            let saved_sp = ctx.sp;
            ctx.local_count = frame_size;
            ctx.sp = 0;
            unsafe {
                ctx.buffer.add(0).write(receiver);
            }
            for (i, arg) in args.into_iter().enumerate() {
                unsafe {
                    ctx.buffer.add(1 + i).write(arg);
                }
            }
            let fn_ptr: extern "C" fn(*mut JitContext) -> u64 =
                unsafe { std::mem::transmute(fp as *const ()) };
            let _disc = fn_ptr(ctx);
            let result = std::mem::replace(&mut ctx.result, Value::Unit);
            ctx.local_count = saved_local_count;
            ctx.sp = saved_sp;
            // disc == 0 (done), disc == 2 (error, already in ctx.error_msg)
            unsafe {
                push(ctx, result);
            }
            return;
        }
    }

    // Fall back to built-in dispatch
    let result = dispatch_builtin_method(receiver.clone(), &method_name, args.clone());

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
fn dispatch_builtin_method(
    receiver: Value,
    method_name: &str,
    args: Vec<Value>,
) -> Result<Value, String> {
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
                |f, fa| jit_closure_invoker(&f, fa),
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
                jit_closure_invoker(&f, fa)
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
                jit_closure_invoker(&f, fa)
            })
        }
        Value::EnumVariant { enum_name, .. } if enum_name == "Result" => {
            crate::vm::builtins::result::dispatch(receiver, method_name, &args, |f, fa| {
                jit_closure_invoker(&f, fa)
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
            "is_match" => {
                // Stubbed for now
                Ok(Value::Bool(false))
            }
            "find" | "find_all" | "replace" => Err(format!(
                "regex method '{method_name}' not yet supported in JIT"
            )),
            _ => Err(format!("no method '{method_name}' on type Regex")),
        },
        Value::Struct { .. } => match method_name {
            "clone" => Ok(receiver.clone()),
            "to_string" => Ok(Value::String(receiver.to_string())),
            _ => Err(format!("no method '{method_name}' on struct")),
        },
        Value::Iterator(_) => {
            crate::vm::builtins::iterator::dispatch(receiver, method_name, &args, |f, fa| {
                jit_closure_invoker(&f, fa)
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
                jit_closure_invoker(&f, fa)
            })
        }
        _ => Err(format!(
            "no method '{method_name}' on type {}",
            receiver.type_name()
        )),
    }
}

// ── Try / Cast ────────────────────────────────────────────────────────

extern "C" fn oxy_try_pop(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    match &val {
        Value::EnumVariant {
            enum_name,
            variant,
            data,
        } if enum_name == "Result" && variant == "Err" => {
            // Early return: push the error and return from the function.
            // The JIT function will see this on the stack and handle via oxy_return.
            ctx.result = val;
            // Signal early return by setting resume_ip to a sentinel
            ctx.resume_ip = usize::MAX;
        }
        Value::EnumVariant {
            enum_name, variant, ..
        } if enum_name == "Option" && variant == "None" => {
            ctx.result = val;
            ctx.resume_ip = usize::MAX;
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

extern "C" fn oxy_cast_int(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let result = crate::vm::arith::cast_to_int(&val, crate::types::IntegerWidth::I64);
    unsafe {
        push(ctx, result);
    }
}

extern "C" fn oxy_cast_float(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let result = crate::vm::arith::cast_to_float(&val, crate::types::FloatWidth::F64);
    unsafe {
        push(ctx, result);
    }
}

extern "C" fn oxy_cast_to_char(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let n = crate::vm::arith::value_to_i64(&val);
    let c = char::from_u32(n as u32).unwrap_or('\u{FFFD}');
    unsafe {
        push(ctx, Value::Char(c));
    }
}

// ── Pattern matching ──────────────────────────────────────────────────

extern "C" fn oxy_bind_ident(ctx: *mut JitContext, index: usize) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    unsafe {
        ctx.buffer.add(index).write(val);
    }
}

extern "C" fn oxy_enum_data_get(ctx: *mut JitContext, index: usize) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    match &val {
        Value::EnumVariant { data, .. } => {
            let inner = data.get(index).cloned().unwrap_or(Value::Unit);
            unsafe {
                push(ctx, inner);
            }
        }
        _ => {
            set_error(ctx, format!("EnumDataGet on non-enum: {val:?}"));
            unsafe {
                push(ctx, Value::Unit);
            }
        }
    }
}

extern "C" fn oxy_enum_variant_equal(
    ctx: *mut JitContext,
    enum_name_ptr: *const u8,
    enum_name_len: usize,
    variant_ptr: *const u8,
    variant_len: usize,
) {
    let ctx = unsafe { &mut *ctx };
    let enum_name = unsafe {
        let slice = std::slice::from_raw_parts(enum_name_ptr, enum_name_len);
        String::from_utf8_lossy(slice).to_string()
    };
    let variant = unsafe {
        let slice = std::slice::from_raw_parts(variant_ptr, variant_len);
        String::from_utf8_lossy(slice).to_string()
    };
    let val = unsafe { pop(ctx) };
    let matched = matches!(
        &val,
        Value::EnumVariant { enum_name: en, variant: v, .. }
            if en == &enum_name && v == &variant
    );
    unsafe {
        push(ctx, Value::Bool(matched));
    }
}

extern "C" fn oxy_make_enum_variant(
    ctx: *mut JitContext,
    enum_name_ptr: *const u8,
    enum_name_len: usize,
    variant_ptr: *const u8,
    variant_len: usize,
    arg_count: usize,
) {
    let ctx = unsafe { &mut *ctx };
    let enum_name = unsafe {
        let slice = std::slice::from_raw_parts(enum_name_ptr, enum_name_len);
        String::from_utf8_lossy(slice).to_string()
    };
    let variant = unsafe {
        let slice = std::slice::from_raw_parts(variant_ptr, variant_len);
        String::from_utf8_lossy(slice).to_string()
    };
    let mut data = Vec::with_capacity(arg_count);
    for _ in 0..arg_count {
        data.push(unsafe { pop(ctx) });
    }
    data.reverse();
    unsafe {
        push(
            ctx,
            Value::EnumVariant {
                enum_name,
                variant,
                data,
            },
        );
    }
}

extern "C" fn oxy_const_enum_variant(
    ctx: *mut JitContext,
    enum_name_ptr: *const u8,
    enum_name_len: usize,
    variant_ptr: *const u8,
    variant_len: usize,
) {
    let ctx = unsafe { &mut *ctx };
    let enum_name = unsafe {
        let slice = std::slice::from_raw_parts(enum_name_ptr, enum_name_len);
        String::from_utf8_lossy(slice).into_owned()
    };
    let variant = unsafe {
        let slice = std::slice::from_raw_parts(variant_ptr, variant_len);
        String::from_utf8_lossy(slice).into_owned()
    };
    unsafe {
        push(
            ctx,
            Value::EnumVariant {
                enum_name,
                variant,
                data: vec![],
            },
        );
    }
}

// ── PathCall builtins ─────────────────────────────────────────────────

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

    // Try module dispatch: [module, fn] or [std, module, fn]
    let module_route = match seg_refs.as_slice() {
        [module, func] => Some((module.to_string(), func.to_string())),
        ["std", module, func] => Some((module.to_string(), func.to_string())),
        _ => None,
    };
    if let Some((module, func)) = module_route {
        if let Some(call) = registry::lookup_module(&module) {
            match call_stdlib_jit(call, &func, &args) {
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
    module_call: crate::stdlib::registry::ModuleCall,
    func: &str,
    args: &[Value],
) -> Result<Value, String> {
    let mut cb = |f: &Value, fargs: &[Value]| jit_closure_invoker(f, fargs);
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

/// Global scheduler for async task management.
static SCHEDULER: std::sync::OnceLock<std::sync::Mutex<crate::vm::scheduler::Scheduler>> =
    std::sync::OnceLock::new();

pub(super) fn scheduler_lock() -> std::sync::MutexGuard<'static, crate::vm::scheduler::Scheduler> {
    SCHEDULER
        .get_or_init(|| std::sync::Mutex::new(crate::vm::scheduler::Scheduler::new()))
        .lock()
        .unwrap()
}

/// Build a JitTaskState from the current JitContext.
pub(super) fn jit_state_from_ctx(
    ctx: &mut JitContext,
    resume_ip: usize,
) -> crate::vm::scheduler::JitTaskState {
    let entry_ip = ctx.entry_ip;
    let local_count = ctx.local_count;
    let sp = ctx.sp;
    let mut locals = Vec::new();
    for i in 0..local_count {
        locals.push(unsafe { ctx.buffer.add(i).read() });
    }
    let mut operand_stack = Vec::new();
    for i in 0..sp {
        operand_stack.push(unsafe { ctx.buffer.add(local_count + i).read() });
    }
    // Prevent JitContext::drop from dropping these values — they're now owned
    // by the JitTaskState. Without this, ptr::read above creates shallow copies
    // and the Drop impl would free the same heap memory twice.
    ctx.local_count = 0;
    ctx.sp = 0;
    crate::vm::scheduler::JitTaskState {
        entry_ip,
        resume_ip,
        locals,
        operand_stack,
        local_count,
        yield_reason: ctx.yield_reason,
        yield_data: ctx.yield_data,
    }
}

/// Restore JitContext from a JitTaskState (takes ownership to prevent double-free).
pub(super) fn ctx_from_jit_state(ctx: &mut JitContext, state: crate::vm::scheduler::JitTaskState) {
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
    ctx.entry_ip = state.entry_ip;
    ctx.yield_reason = state.yield_reason;
    ctx.yield_data = state.yield_data;
    // Take ownership of values from the state (into_iter consumes the Vec,
    // preventing JitTaskState::drop from freeing them).
    for (i, v) in state.locals.into_iter().enumerate() {
        unsafe {
            ctx.buffer.add(i).write(v);
        }
    }
    for (i, v) in state.operand_stack.into_iter().enumerate() {
        unsafe {
            ctx.buffer.add(state.local_count + i).write(v);
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
                match table.get(&target_ip) {
                    Some(p) => *p,
                    None => {
                        set_error(
                            ctx,
                            format!("JIT: no function for future at ip={target_ip}"),
                        );
                        unsafe {
                            push(ctx, Value::Unit);
                        }
                        return 2;
                    }
                }
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
            // Not done — push JoinHandle back so the resume state includes it,
            // then save state and yield.
            unsafe {
                push(ctx, Value::JoinHandle { task_id });
            }
            ctx.yield_reason = 2;
            ctx.yield_data = task_id as u64;
            let jit_state = jit_state_from_ctx(ctx, ctx.resume_ip);
            scheduler_lock().yield_jit_for_task(task_id, jit_state);
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
                entry_ip: target_ip,
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
        _ => {
            set_error(ctx, "spawn requires a closure".to_string());
            unsafe {
                push(ctx, Value::Unit);
            }
        }
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
    // Push the ms value back so the resume state includes it.
    unsafe {
        push(ctx, ms_val);
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
    // None ready — push JoinHandles back so resume state includes them,
    // then save state and yield.
    for &tid in task_ids.iter().rev() {
        unsafe {
            push(ctx, Value::JoinHandle { task_id: tid });
        }
    }
    ctx.yield_reason = 3;
    ctx.yield_data = task_ids.first().copied().unwrap_or(0) as u64;
    let jit_state = jit_state_from_ctx(ctx, ctx.resume_ip);
    scheduler_lock().yield_jit_for_multiple(task_ids.clone(), jit_state);
    1
}

extern "C" fn oxy_make_future(ctx: *mut JitContext, target_ip: usize, arg_count: usize) {
    let ctx = unsafe { &mut *ctx };
    let mut args = Vec::with_capacity(arg_count);
    for _ in 0..arg_count {
        args.push(unsafe { pop(ctx) });
    }
    args.reverse();
    let meta = {
        let lock = async_fn_meta_lock();
        lock.iter().find(|m| m.target_ip == target_ip).map(|m| {
            (
                m.name.clone(),
                m.params.clone(),
                m.return_type.clone(),
                m.body.clone(),
            )
        })
    };
    let (name, params, return_type, body) = match meta {
        Some(m) => m,
        None => {
            set_error(
                ctx,
                format!("MakeFuture: no async function found at target_ip={target_ip}"),
            );
            unsafe {
                push(ctx, Value::Unit);
            }
            return;
        }
    };
    let future = Value::Future(Box::new(crate::types::FutureData {
        name,
        params,
        return_type,
        body: (*body).clone(),
        closure_env: crate::env::Environment::new(),
        args,
        target_ip,
        captured_names: Vec::new(),
    }));
    unsafe {
        push(ctx, future);
    }
}

// ── Symbol registry ──────────────────────────────────────────────────

pub(crate) fn register_ffi_symbols(builder: &mut JITBuilder) {
    let syms: &[(&str, *const u8)] = &[
        ("oxy_set_result_i64", oxy_set_result_i64 as _),
        ("oxy_push_unit", oxy_push_unit as _),
        ("oxy_push_bool", oxy_push_bool as _),
        ("oxy_push_int", oxy_push_int as _),
        ("oxy_push_float", oxy_push_float as _),
        ("oxy_push_char", oxy_push_char as _),
        ("oxy_push_string", oxy_push_string as _),
        ("oxy_pop", oxy_pop as _),
        ("oxy_dup", oxy_dup as _),
        ("oxy_load_local", oxy_load_local as _),
        ("oxy_read_local_i64", oxy_read_local_i64 as _),
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
        ("oxy_error_discriminant", oxy_error_discriminant as _),
        ("oxy_panic", oxy_panic as _),
        ("oxy_make_array", oxy_make_array as _),
        ("oxy_make_fixed_array", oxy_make_fixed_array as _),
        ("oxy_make_tuple", oxy_make_tuple as _),
        ("oxy_make_iter", oxy_make_iter as _),
        ("oxy_make_repeat", oxy_make_repeat as _),
        ("oxy_iter_len", oxy_iter_len as _),
        ("oxy_iter_next", oxy_iter_next as _),
        ("oxy_iter_next_destructure", oxy_iter_next_destructure as _),
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
        ("oxy_enum_variant_equal", oxy_enum_variant_equal as _),
        ("oxy_make_enum_variant", oxy_make_enum_variant as _),
        ("oxy_const_enum_variant", oxy_const_enum_variant as _),
        ("oxy_path_call_builtin", oxy_path_call_builtin as _),
        ("oxy_display_arg", oxy_display_arg as _),
        ("oxy_await_ffi", oxy_await_ffi as _),
        ("oxy_spawn_ffi", oxy_spawn_ffi as _),
        ("oxy_sleep_ffi", oxy_sleep_ffi as _),
        ("oxy_select_ffi", oxy_select_ffi as _),
        ("oxy_make_future", oxy_make_future as _),
    ];

    for (name, ptr) in syms {
        builder.symbol(*name, *ptr);
    }
}
