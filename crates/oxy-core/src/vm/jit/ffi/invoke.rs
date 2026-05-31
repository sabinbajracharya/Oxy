//! Function-call infrastructure: CalleeFrame, closure creation/ invocation,
//! return, panic, and the interpreter call-back hook (wasm backend).
//!
//! Extracted from [`super`] to keep mod.rs under ~900 lines.
//!
//! # Safety
//!
//! `CalleeFrame` encapsulates the buffer-swap pattern: allocate a fresh buffer,
//! swap it into ctx for the callee's duration, then restore the caller's state.
//! The raw pointer manipulations are safe because the buffer is independently
//! allocated with a known layout, and the caller's buffer/sp/capacity/local_count
//! are atomically saved and restored. `transmute` of fn pointers is safe because
//! the pointers come from the JIT symbol table and have the correct C ABI signature.
//! `extern "C"` functions receive a valid `*mut JitContext` from Cranelift; string
//! pointers from the JIT are valid for the call's duration.

use std::cell::Cell;

use super::*;
use crate::types::{FutureData, Value};
use crate::vm::jit::context::JitContext;

// ── CalleeFrame: buffer lifecycle for JIT function calls ────────────────
//
// Every JIT function call needs a fresh buffer where the callee's locals
// and operand stack live. CalleeFrame encapsulates the alloc / swap / call /
// drop / dealloc / restore pattern so it isn't duplicated at every call site.

pub(crate) const STACK_CAP: usize = 2048;

pub(crate) struct CalleeFrame {
    buf: *mut Value,
    layout: std::alloc::Layout,
    capacity: usize,
    local_count: usize,
}

impl CalleeFrame {
    pub(crate) fn new(min_locals: usize) -> Self {
        let capacity = min_locals + STACK_CAP;
        let layout = std::alloc::Layout::array::<Value>(capacity).unwrap();
        // Safety: layout is computed from a known Value size and valid capacity.
        // The resulting buffer is zero-initialized and valid for `capacity` Values.
        let buf = unsafe { std::alloc::alloc_zeroed(layout) as *mut Value };
        Self {
            buf,
            layout,
            capacity,
            local_count: min_locals,
        }
    }

    pub(crate) fn buf_mut(&mut self) -> *mut Value {
        self.buf
    }

    /// Swap this frame into ctx, call fn_ptr, drop callee state, dealloc,
    /// restore the caller's buffer, and push the result onto the caller's stack.
    ///
    /// `saved_local_count` is the caller's local_count before the call.
    /// `result_sp` is the caller's sp value after consuming args (where the
    ///   callee's result should be pushed).
    pub(crate) unsafe fn execute(
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
pub(crate) fn invoke_jit_fn(
    ctx: &mut JitContext,
    fn_ptr: *const u8,
    local_count: usize,
    args: Vec<Value>,
) {
    // The caller already popped the args, so the operand stack top is where the
    // call's result will land.
    let result_sp = ctx.sp;
    let mut frame = CalleeFrame::new(local_count);
    for (i, arg) in args.into_iter().enumerate() {
        // Safety: frame.buf_mut() returns a valid buffer; i < local_count ≤ capacity.
        unsafe { frame.buf_mut().add(i).write(arg) };
    }

    let saved_local_count = ctx.local_count;
    // Safety: execute() swaps the callee's buffer into ctx, calls fn_ptr
    // (a valid JIT entry point), and restores the caller's state.
    unsafe {
        frame.execute(ctx, fn_ptr, saved_local_count, result_sp);
    }
}

// ── Closures ─────────────────────────────────────────────────────────────

pub(super) extern "C" fn oxy_push_closure(
    ctx: *mut JitContext,
    name_ptr: *const u8,
    name_len: usize,
    meta_idx: usize,
) {
    // Safety: ctx is a valid JitContext from JIT codegen.
    let ctx = unsafe { &mut *ctx };

    // Safety: name_ptr/name_len describe a valid JIT-owned string buffer.
    let name = unsafe {
        let bytes = std::slice::from_raw_parts(name_ptr, name_len);
        String::from_utf8_lossy(bytes).into_owned()
    };

    // Look up captures metadata.
    // Safety: ctx.tables is set to a valid JitTables pointer by the JIT engine.
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
        // Safety: outer_slot is a compile-time constant within local_count;
        // the slot holds an initialized Value. We forget the shallow copy
        // so its Drop doesn't double-free the original's heap data.
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
pub(super) extern "C" fn oxy_push_named_fn(
    ctx: *mut JitContext,
    name_ptr: *const u8,
    name_len: usize,
) {
    // Safety: ctx is a valid JitContext from JIT codegen.
    let ctx = unsafe { &mut *ctx };
    // Safety: JIT-guaranteed valid string buffer.
    let name = unsafe {
        let bytes = std::slice::from_raw_parts(name_ptr, name_len);
        String::from_utf8_lossy(bytes).into_owned()
    };
    // Safety: ctx.tables is valid and non-null during execution.
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

pub(super) extern "C" fn oxy_push_async_block(
    ctx: *mut JitContext,
    name_ptr: *const u8,
    name_len: usize,
    meta_idx: usize,
) {
    // Safety: ctx is a valid JitContext from JIT codegen.
    let ctx = unsafe { &mut *ctx };

    // Safety: JIT-guaranteed valid string buffer.
    let name = unsafe {
        let bytes = std::slice::from_raw_parts(name_ptr, name_len);
        String::from_utf8_lossy(bytes).into_owned()
    };

    // Safety: ctx.tables is valid and non-null during execution.
    let tables = unsafe { &*ctx.tables };
    let fn_index = tables.name_to_index(&name).unwrap_or(usize::MAX);

    let meta = tables.closure_meta(meta_idx).cloned();
    let captured = meta.map(|m| m.captured.clone()).unwrap_or_default();

    let closure_env = crate::env::Environment::new();
    for (captured_name, outer_slot, is_mut) in &captured {
        // Safety: outer_slot is within local_count; slot holds an initialized Value.
        // forget() prevents double-free of the shallow copy's heap data.
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
    let future_data = FutureData {
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

pub(super) extern "C" fn oxy_call_closure(ctx: *mut JitContext, arg_count: usize) {
    // Safety: ctx is a valid JitContext from JIT codegen.
    let ctx = unsafe { &mut *ctx };

    // Pop closure value (receiver) from below the args.
    // Safety: closure_idx is computed from ctx.sp and arg_count; the IR guarantees
    // sufficient stack depth. We forget() the shallow read to avoid double-free.
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
        let future = FutureData {
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
    // Safety: drain_start is within the allocated buffer; each src.read() moves
    // one Value, and src.write(Value::Unit) prevents double-free of the original.
    let mut args_vals = Vec::with_capacity(arg_count);
    for i in 0..arg_count {
        let src = unsafe { ctx.buffer.add(ctx.local_count + drain_start + 1 + i) };
        args_vals.push(unsafe { src.read() });
        unsafe { src.write(Value::Unit) };
    }
    // Safety: clear the closure slot to prevent double-free.
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
        // Safety: frame.buf_mut() returns a valid buffer; i < total_frame ≤ capacity.
        unsafe {
            frame.buf_mut().add(i).write(val);
        }
    }
    for (i, arg) in args_vals.into_iter().enumerate() {
        // Safety: captures_end + i < total_frame ≤ capacity.
        unsafe {
            frame.buf_mut().add(captures_end + i).write(arg);
        }
    }

    // Safety: frame.execute swaps in the callee buffer, calls fn_ptr (valid JIT
    // entry point), and restores the caller's state atomically.
    unsafe {
        frame.execute(ctx, fn_ptr, saved_local_count, ctx.sp);
    }
}

// ── Return / panic ──────────────────────────────────────────────────────

pub(super) extern "C" fn oxy_error_discriminant(ctx: *const JitContext) -> u64 {
    // Safety: ctx is a valid, non-null JitContext pointer from JIT codegen.
    // This is a read-only access to the error flag.
    let ctx = unsafe { &*ctx };
    if ctx.error_len > 0 {
        2
    } else {
        0
    }
}

pub(super) extern "C" fn oxy_return(ctx: *mut JitContext) {
    // Safety: ctx is a valid JitContext from the JIT.
    let ctx = unsafe { &mut *ctx };
    let result = if ctx.sp == 0 {
        Value::Unit
    } else {
        // Safety: pop from valid operand stack with at least one value.
        unsafe { pop(ctx) }
    };
    ctx.result = result;
}

pub(super) extern "C" fn oxy_panic(ctx: *mut JitContext) {
    // Safety: ctx is a valid JitContext from the JIT.
    let ctx = unsafe { &mut *ctx };
    // Safety: pop the panic message from the valid operand stack.
    let msg_val = unsafe { pop(ctx) };
    let msg = format!("{msg_val:?}");
    let len = msg.len().min(1023);
    ctx.error_msg[..len].copy_from_slice(&msg.as_bytes()[..len]);
    ctx.error_len = len;
}

// ── Interpreter call-back hook ──────────────────────────────────────────
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
    static INTERP_INVOKE: Cell<Option<(InterpInvokeFn, *const ())>> =
        const { Cell::new(None) };
}

/// Install (or clear, with `None`) the interpreter call-back hook, returning the
/// previous value so a guard can restore it — supporting reentrant/nested runs.
pub(crate) fn set_interp_invoke(
    hook: Option<(InterpInvokeFn, *const ())>,
) -> Option<(InterpInvokeFn, *const ())> {
    INTERP_INVOKE.with(|c| c.replace(hook))
}

/// The currently-installed interpreter call-back hook, if any. `None` on the JIT.
pub(crate) fn interp_invoke() -> Option<(InterpInvokeFn, *const ())> {
    INTERP_INVOKE.with(|c| c.get())
}
