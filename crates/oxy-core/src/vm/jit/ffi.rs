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

use super::context::{JitContext, JitTables};
use crate::types::Value;
use cranelift_jit::JITBuilder;
use std::collections::HashMap;

// ── Stack helpers (used by JIT and FFI internally) ──────────────────

/// Move a Value from one buffer slot to another, leaving Unit behind at the source.
///
/// This is the **only** way to transfer ownership of a `Value` between buffer slots.
/// `Value` contains heap-allocated types (`Rc`, `String`, `Vec`) — a bare `ptr::read`
/// followed by a `write` on the destination creates two owners of the same allocation.
/// Always clear the source slot with `Value::Unit` after moving so the buffer's `Drop`
/// doesn't double-free when it cleans up.
unsafe fn move_value(src: *mut Value, dst: *mut Value) {
    dst.write(src.read());
    src.write(Value::Unit);
}

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
    // Ensure error_len is non-zero even for empty messages (e.g. ? propagation).
    ctx.error_len = if len == 0 { 1 } else { len };
}

unsafe fn pop(ctx: &mut JitContext) -> Value {
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
        result
    };
    if !ctx.output.is_null() {
        let output = unsafe { &*ctx.output };
        output.borrow_mut().push(format!("{line}\n"));
    } else {
        println!("{line}");
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

binary_op!(oxy_add, super::runtime::vm_add);
binary_op!(oxy_sub, super::runtime::vm_sub);
binary_op!(oxy_mul, super::runtime::vm_mul);
binary_op!(oxy_div, super::runtime::vm_div);
binary_op!(oxy_mod, super::runtime::vm_rem);

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
        (Value::String(a), Value::String(b)) => Value::Bool(a < b),
        (Value::Char(a), Value::Char(b)) => Value::Bool(a < b),
        _ => Value::Bool(false),
    }
}
fn vm_gt(lhs: Value, rhs: Value) -> Value {
    match (&lhs, &rhs) {
        (Value::I64(a), Value::I64(b)) => Value::Bool(a > b),
        (Value::F64(a), Value::F64(b)) => Value::Bool(a > b),
        (Value::U8(a), Value::U8(b)) => Value::Bool(a > b),
        (Value::String(a), Value::String(b)) => Value::Bool(a > b),
        (Value::Char(a), Value::Char(b)) => Value::Bool(a > b),
        _ => Value::Bool(false),
    }
}
fn vm_le(lhs: Value, rhs: Value) -> Value {
    match (&lhs, &rhs) {
        (Value::I64(a), Value::I64(b)) => Value::Bool(a <= b),
        (Value::F64(a), Value::F64(b)) => Value::Bool(a <= b),
        (Value::U8(a), Value::U8(b)) => Value::Bool(a <= b),
        (Value::String(a), Value::String(b)) => Value::Bool(a <= b),
        (Value::Char(a), Value::Char(b)) => Value::Bool(a <= b),
        _ => Value::Bool(false),
    }
}
fn vm_ge(lhs: Value, rhs: Value) -> Value {
    match (&lhs, &rhs) {
        (Value::I64(a), Value::I64(b)) => Value::Bool(a >= b),
        (Value::F64(a), Value::F64(b)) => Value::Bool(a >= b),
        (Value::U8(a), Value::U8(b)) => Value::Bool(a >= b),
        (Value::String(a), Value::String(b)) => Value::Bool(a >= b),
        (Value::Char(a), Value::Char(b)) => Value::Bool(a >= b),
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

binary_op!(oxy_bitand, super::runtime::vm_bitand);
binary_op!(oxy_bitor, super::runtime::vm_bitor);
binary_op!(oxy_bitxor, super::runtime::vm_bitxor);
binary_op!(oxy_shl, super::runtime::vm_shl);
binary_op!(oxy_shr, super::runtime::vm_shr);

// ── Unary ────────────────────────────────────────────────────────────

extern "C" fn oxy_neg(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let result = super::runtime::vm_neg(val);
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

// ── Global state reset ──────────────────────────────────────────────

/// Reset the async scheduler between compilations to prevent task state
/// from leaking across test runs.
pub(crate) fn reset_runtime_state() {
    scheduler_lock().reset();
}

// ── Function calls ───────────────────────────────────────────────────

/// Call stack for nested Oxy function invocations.
static CALL_STACK: std::sync::OnceLock<std::sync::Mutex<Vec<CallFrame>>> =
    std::sync::OnceLock::new();

struct CallFrame {
    caller_local_count: usize,
    caller_sp: usize,
}

fn call_stack_lock() -> std::sync::MutexGuard<'static, Vec<CallFrame>> {
    CALL_STACK
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .unwrap()
}

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
        ctx.local_count = saved_local_count;
        ctx.sp = result_sp;
        push(ctx, result);
    }
}

/// Call a JIT-compiled function identified by fn_ptr with args already on
/// the caller's operand stack. This is the shared call path used by both
/// oxy_call (simple calls) and oxy_path_call_builtin (module-qualified calls).
fn invoke_jit_fn(ctx: &mut JitContext, fn_ptr: *const u8, local_count: usize, arg_count: usize) {
    // Save caller state on the call stack
    {
        let mut call_stack = call_stack_lock();
        call_stack.push(CallFrame {
            caller_local_count: ctx.local_count,
            caller_sp: ctx.sp,
        });
    }

    // Move args from the caller's operand stack to the callee's buffer.
    // Uses move_value so source slots are cleared to Value::Unit, preventing
    // double-free of heap-allocated data when the caller's buffer is dropped.
    let args_start = ctx.sp - arg_count;
    let mut frame = CalleeFrame::new(local_count);
    for i in 0..arg_count {
        let src = unsafe { ctx.buffer.add(ctx.local_count + args_start + i) };
        let dst = unsafe { frame.buf_mut().add(i) };
        unsafe { move_value(src, dst) };
    }
    let result_sp = args_start;

    let saved_local_count = ctx.local_count;
    unsafe {
        frame.execute(ctx, fn_ptr, saved_local_count, result_sp);
    }
}

// ── Closures ─────────────────────────────────────────────────────────

extern "C" fn oxy_push_closure(ctx: *mut JitContext, name_ptr: i64, name_len: i64, meta_idx: i64) {
    let ctx = unsafe { &mut *ctx };

    let name = unsafe {
        let bytes = std::slice::from_raw_parts(name_ptr as *const u8, name_len as usize);
        String::from_utf8_lossy(bytes).into_owned()
    };

    // Look up captures metadata.
    let tables = unsafe { &*ctx.tables };
    let meta = tables.closure_meta(meta_idx as usize).cloned();
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
extern "C" fn oxy_push_named_fn(ctx: *mut JitContext, name_ptr: i64, name_len: i64) {
    let ctx = unsafe { &mut *ctx };
    let name = unsafe {
        let bytes = std::slice::from_raw_parts(name_ptr as *const u8, name_len as usize);
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
    name_ptr: i64,
    name_len: i64,
    meta_idx: i64,
) {
    let ctx = unsafe { &mut *ctx };

    let name = unsafe {
        let bytes = std::slice::from_raw_parts(name_ptr as *const u8, name_len as usize);
        String::from_utf8_lossy(bytes).into_owned()
    };

    let tables = unsafe { &*ctx.tables };
    let fn_index = tables.name_to_index(&name).unwrap_or(usize::MAX);

    let meta = tables.closure_meta(meta_idx as usize).cloned();
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
        frame.execute(ctx, fn_ptr as *const u8, saved_local_count, ctx.sp);
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

extern "C" fn oxy_iter_next_destructure(ctx: *mut JitContext, state_slot: usize) -> i64 {
    // Like oxy_iter_next, but stores each destructured element field to
    // local slots state_slot+1..state_slot+n. Returns 1 (has_next) or 0 (done).
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
            return 0;
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
        }
        1 // has next element
    } else {
        0 // no more elements
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

extern "C" fn oxy_iter_next(ctx: *mut JitContext, state_slot: usize, var_slot: usize) -> i64 {
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
            return 0; // no more elements
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
        // Store raw element in the loop variable's local slot.
        let dest_ptr = unsafe { ctx.buffer.add(var_slot) };
        unsafe {
            std::ptr::drop_in_place(dest_ptr);
            dest_ptr.write(elem);
        }
        unsafe {
            std::ptr::drop_in_place(target_ptr);
            target_ptr.write(Value::Tuple(vec![
                vec_clone,
                Value::I64((index + 1) as i64),
            ]));
        }
        1 // has next element
    } else {
        0 // no more elements
    }
}

extern "C" fn oxy_vec_index(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let index_val = unsafe { pop(ctx) };
    let collection = unsafe { pop(ctx) };

    // Handle range slicing.
    if let Value::Range(start, end) = &index_val {
        let result = match collection {
            Value::String(ref s) => {
                let len = s.chars().count() as i64;
                let s_start = *start;
                let s_end = if *end == i64::MAX { len } else { *end };
                let clamped_start = s_start.max(0).min(len) as usize;
                let clamped_end = s_end.max(0).min(len) as usize;
                if clamped_end <= clamped_start {
                    Value::String(String::new())
                } else {
                    Value::String(
                        s.chars()
                            .skip(clamped_start)
                            .take(clamped_end - clamped_start)
                            .collect(),
                    )
                }
            }
            Value::Vec(rc) => {
                let v = rc.borrow();
                let len = v.len() as i64;
                let s_start = *start;
                let s_end = if *end == i64::MAX { len } else { *end };
                let clamped_start = s_start.max(0).min(len) as usize;
                let clamped_end = s_end.max(0).min(len) as usize;
                Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(
                    v[clamped_start..clamped_end].to_vec(),
                )))
            }
            Value::Array(ref a) => {
                let len = a.len() as i64;
                let s_start = *start;
                let s_end = if *end == i64::MAX { len } else { *end };
                let clamped_start = s_start.max(0).min(len) as usize;
                let clamped_end = s_end.max(0).min(len) as usize;
                Value::Array(a[clamped_start..clamped_end].to_vec())
            }
            _ => {
                set_error(ctx, format!("cannot slice {collection:?}"));
                Value::Unit
            }
        };
        unsafe {
            push(ctx, result);
        }
        return;
    }

    let idx = super::runtime::value_to_i64(&index_val) as usize;
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
    let idx = super::runtime::value_to_i64(&index_val) as usize;
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

extern "C" fn oxy_make_range(ctx: *mut JitContext, inclusive: i64) {
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
    let e = if inclusive != 0 { e + 1 } else { e };
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
        Value::Tuple(ref elements) => {
            if let Ok(idx) = name.parse::<usize>() {
                elements.get(idx).cloned().unwrap_or(Value::Unit)
            } else {
                set_error(ctx, format!("tuple field not an integer: {name}"));
                unsafe {
                    push(ctx, Value::Unit);
                }
                return;
            }
        }
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
fn jit_closure_invoker(tables: &JitTables, func: &Value, args: &[Value]) -> Result<Value, String> {
    let ft = match func {
        Value::Function(f) => f.clone(),
        _ => return Err("not a callable function".into()),
    };
    let target_ip = ft.target_ip.ok_or("function has no target_ip")?;
    let fn_ptr = tables
        .fn_ptr(target_ip)
        .ok_or(format!("JIT: no function for closure at ip={target_ip}"))?;

    let captures_end = ft.captured_names.len();
    let actual_local_count = tables.local_count(target_ip);
    let min_locals = captures_end + args.len();
    let local_count = actual_local_count.max(min_locals);
    let mut call_ctx = JitContext::new(local_count);
    call_ctx.tables = tables as *const JitTables;
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

    // JIT name-based lookup for user-defined struct/enum methods.
    let tables = unsafe { &*ctx.tables };
    // The old method_ips table is bytecode-only; JIT-compiled methods are
    // registered by qualified name (e.g. "Counter::inc") in the fn table.
    let qualified = format!("{lookup_name}::{method_name}");
    if let Some(fn_index) = tables.name_to_index(&qualified) {
        if let Some(fp) = tables.fn_table.get(&fn_index).copied() {
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
            unsafe {
                push(ctx, result);
            }
            return;
        }
    }

    // Fall back to built-in dispatch
    let result = dispatch_builtin_method(tables, receiver.clone(), &method_name, args.clone());

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
    tables: &JitTables,
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
                |f, fa| jit_closure_invoker(tables, &f, fa),
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
                jit_closure_invoker(tables, &f, fa)
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
                jit_closure_invoker(tables, &f, fa)
            })
        }
        Value::EnumVariant { enum_name, .. } if enum_name == "Result" => {
            crate::vm::builtins::result::dispatch(receiver, method_name, &args, |f, fa| {
                jit_closure_invoker(tables, &f, fa)
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
                jit_closure_invoker(tables, &f, fa)
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
                jit_closure_invoker(tables, &f, fa)
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

extern "C" fn oxy_cast_int(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let result = super::runtime::cast_to_int(&val, crate::types::IntegerWidth::I64);
    unsafe {
        push(ctx, result);
    }
}

extern "C" fn oxy_cast_byte(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let result = super::runtime::cast_to_int(&val, crate::types::IntegerWidth::U8);
    unsafe {
        push(ctx, result);
    }
}

extern "C" fn oxy_cast_float(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let result = super::runtime::cast_to_float(&val, crate::types::FloatWidth::F64);
    unsafe {
        push(ctx, result);
    }
}

extern "C" fn oxy_cast_to_char(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let n = super::runtime::value_to_i64(&val);
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
        Value::Tuple(ref t) => {
            let inner = t.get(index).cloned().unwrap_or(Value::Unit);
            unsafe {
                push(ctx, inner);
            }
        }
        Value::Array(ref a) => {
            let inner = a.get(index).cloned().unwrap_or(Value::Unit);
            unsafe {
                push(ctx, inner);
            }
        }
        Value::Vec(ref rc) => {
            let inner = rc.borrow().get(index).cloned().unwrap_or(Value::Unit);
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

    // Try function call lookup FIRST: join path segments with "::" and look
    // up in the JIT function table. Must come before module dispatch so
    // user-defined modules (e.g. `mod math { fn double }`) take priority
    // over stdlib modules with the same name (e.g. math::sqrt).
    let fn_name = seg_refs.join("::");
    let tables = unsafe { &*ctx.tables };
    if let Some(fn_idx) = tables.name_to_index(&fn_name) {
        if let Some(fn_ptr) = tables.fn_ptr(fn_idx) {
            let local_count = tables.local_count(fn_idx);
            for a in args.into_iter().rev() {
                unsafe {
                    push(ctx, a);
                }
            }
            invoke_jit_fn(ctx, fn_ptr, local_count, arg_count);
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
            match call_stdlib_jit(tables, call, &func, &args) {
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
    module_call: crate::stdlib::registry::ModuleCall,
    func: &str,
    args: &[Value],
) -> Result<Value, String> {
    let mut cb = |f: &Value, fargs: &[Value]| jit_closure_invoker(tables, f, fargs);
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
                    set_error(
                        ctx,
                        format!("JIT: no function for future at ip={target_ip}"),
                    );
                    unsafe { push(ctx, Value::Unit) };
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
                frame.execute(ctx, fn_ptr as *const u8, ctx.local_count, saved_sp);
            }
        }
        Value::JoinHandle { task_id } => {
            // Tasks run eagerly in oxy_spawn_ffi, so the result is always ready.
            let result = scheduler_lock().task_result(task_id);
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
            let task_result = if let Some(fn_ptr) = tables.fn_ptr(target_ip) {
                let mut task_ctx = JitContext::new(local_count);
                task_ctx.tables = tables as *const JitTables;
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
            } else {
                Value::Unit
            };

            let mut sched = scheduler_lock();
            let task_id = sched.create_task();
            // Create the task entry first so complete() can find it.
            sched.save_new_task(
                task_id,
                crate::vm::scheduler::TaskSnapshot {
                    ip: target_ip,
                    stack: vec![],
                    jit_state: None,
                },
            );
            // Mark the task as done with the eagerly-computed result.
            let _awoken = sched.complete(task_id, task_result);
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

extern "C" fn oxy_sleep_ffi(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let ms_val = unsafe { pop(ctx) };
    let ms = match ms_val {
        Value::I64(n) => n as u64,
        Value::U8(n) => n as u64,
        _ => 0,
    };
    // sleep(0) is a no-op — used in tests to force async task scheduling.
    // Non-zero sleep requires yield/resume which needs the full event loop.
    unsafe {
        push(ctx, Value::Unit);
    }
    if ms > 0 {
        // For non-zero sleep, we'd need to yield and let the scheduler
        // resume us after the delay. Not yet implemented for JIT.
        let _ = ms;
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
    // Tasks run eagerly, so all should be done. Pick the first one.
    let sched = scheduler_lock();
    for &tid in &task_ids {
        if let Some(v) = sched.task_result(tid) {
            unsafe {
                push(ctx, v);
            }
            return;
        }
    }
    unsafe {
        push(ctx, Value::Unit);
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
        ("oxy_load_local_raw", oxy_load_local_raw as _),
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
        ("oxy_push_named_fn", oxy_push_named_fn as _),
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
        ("oxy_cast_byte", oxy_cast_byte as _),
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
    ];

    for (name, ptr) in syms {
        builder.symbol(*name, *ptr);
    }
}
