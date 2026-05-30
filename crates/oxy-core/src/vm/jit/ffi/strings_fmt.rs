//! String conversion and formatting FFI — part of the shared oxy_* runtime. See `mod.rs`
//! for the core machinery (push/pop, call stack, ffi_symbols).

use super::*;

pub(super) extern "C" fn oxy_to_string(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let val = unsafe { pop(ctx) };
    let s = val.to_string();
    unsafe {
        push(ctx, Value::String(s));
    }
}

pub(super) extern "C" fn oxy_fstring_concat(ctx: *mut JitContext, count: usize) {
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

pub(super) extern "C" fn oxy_format(ctx: *mut JitContext, count: usize) {
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
    let result = format_template_with(&template, &vals[1..], |v| unsafe {
        display_via_user_fmt(ctx, v)
    });
    unsafe {
        push(ctx, Value::String(result));
    }
}

/// `dbg!(expr)` — debug-print the value and return it. Multiple args render as
/// a tuple (matching Rust's `dbg!(a, b)`); zero args print/return unit. The
/// value is left on the operand stack so `let x = dbg!(v)` binds it. Output
/// goes to the run's capture buffer when present, else stdout.
pub(super) extern "C" fn oxy_dbg(ctx: *mut JitContext, count: usize) {
    let ctx = unsafe { &mut *ctx };
    let mut vals = Vec::with_capacity(count);
    for _ in 0..count {
        vals.push(unsafe { pop(ctx) });
    }
    vals.reverse();
    let value = match count {
        1 => vals.pop().unwrap(),
        0 => Value::Unit,
        _ => Value::Tuple(vals),
    };
    let line = value.to_debug_string();
    if !ctx.output.is_null() {
        let output = unsafe { &*ctx.output };
        output.borrow_mut().push(format!("{line}\n"));
    } else {
        println!("{line}");
    }
    unsafe {
        push(ctx, value);
    }
}
