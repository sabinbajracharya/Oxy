//! Collection, iterator, and range FFI — part of the shared oxy_* runtime. See `mod.rs`
//! for the core machinery (push/pop, call stack, ffi_symbols).
//!
//! # Safety
//!
//! All functions are `extern "C"` entry points from Cranelift JIT code. `ctx` is a
//! valid, non-aliased `*mut JitContext`. `ctx.tables` and `ctx.output` are guaranteed
//! non-null during execution. `pop`/`push` operate on a pre-allocated operand stack.
//! Raw pointer access to local slots (`ctx.buffer.add(slot)`) is bounds-checked by
//! the IR compiler: slot indices are compile-time constants that stay within the
//! function's `local_count`. `ptr::read`/`ptr::write`/`ptr::drop_in_place` on buffer
//! slots are safe because each slot holds a valid, initialized `Value`, and the
//! buffer's layout is computed from known sizes.

use super::*;

pub(super) extern "C" fn oxy_make_array(ctx: *mut JitContext, count: usize) {
    // Safety: ctx is a valid, non-aliased JitContext from JIT codegen.
    let ctx = unsafe { &mut *ctx };
    let mut elements = Vec::with_capacity(count);
    for _ in 0..count {
        // Safety: count matches operand stack depth per IR codegen.
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

pub(super) extern "C" fn oxy_make_fixed_array(ctx: *mut JitContext, count: usize) {
    // Safety: ctx is a valid, non-aliased JitContext from JIT codegen.
    let ctx = unsafe { &mut *ctx };
    let mut elements = Vec::with_capacity(count);
    for _ in 0..count {
        // Safety: count matches operand stack depth per IR codegen.
        elements.push(unsafe { pop(ctx) });
    }
    elements.reverse();
    unsafe {
        push(ctx, Value::Array(elements));
    }
}

pub(super) extern "C" fn oxy_make_tuple(ctx: *mut JitContext, count: usize) {
    // Safety: ctx is a valid, non-aliased JitContext from JIT codegen.
    let ctx = unsafe { &mut *ctx };
    let mut elements = Vec::with_capacity(count);
    for _ in 0..count {
        // Safety: count matches operand stack depth per IR codegen.
        elements.push(unsafe { pop(ctx) });
    }
    elements.reverse();
    unsafe {
        push(ctx, Value::Tuple(elements));
    }
}

pub(super) extern "C" fn oxy_make_repeat(ctx: *mut JitContext) {
    // Safety: ctx is a valid JitContext from the JIT.
    let ctx = unsafe { &mut *ctx };
    // Safety: pop count and value from valid operand stack in IR-guaranteed order.
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

pub(super) extern "C" fn oxy_iter_next_destructure(ctx: *mut JitContext, state_slot: usize) -> i64 {
    // Like oxy_iter_next, but stores each destructured element field to
    // local slots state_slot+1..state_slot+n. Returns 1 (has_next) or 0 (done).
    // Safety: ctx is a valid JitContext from the JIT.
    let ctx = unsafe { &mut *ctx };
    // Safety: state_slot is a compile-time constant within local_count; the buffer
    // is valid and the slot holds an initialized Value.
    let target_ptr = unsafe { ctx.buffer.add(state_slot) };
    // Safety: target_ptr points to a valid, initialized Value.
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
                // Safety: state_slot + 1 + i stays within the IR-allocated local
                // slots; the slot holds a valid Value so drop_in_place is sound.
                let dest_ptr = unsafe { ctx.buffer.add(state_slot + 1 + i) };
                unsafe {
                    std::ptr::drop_in_place(dest_ptr);
                }
                // Safety: dest_ptr is valid and we own the slot after dropping the old value.
                unsafe {
                    dest_ptr.write(field.clone());
                }
            }
        }
        // Safety: target_ptr is valid. Drop the old iteration-state tuple and write
        // the updated (collection, next_index) pair.
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

pub(super) extern "C" fn oxy_make_iter(ctx: *mut JitContext) {
    // Safety: ctx is a valid JitContext from the JIT.
    let ctx = unsafe { &mut *ctx };
    // Safety: pop from valid operand stack.
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

pub(super) extern "C" fn oxy_iter_len(ctx: *mut JitContext) {
    // Safety: ctx is a valid JitContext from the JIT.
    let ctx = unsafe { &mut *ctx };
    // Safety: pop from valid operand stack.
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

pub(super) extern "C" fn oxy_iter_next(
    ctx: *mut JitContext,
    state_slot: usize,
    var_slot: usize,
) -> i64 {
    // Safety: ctx is a valid JitContext from the JIT.
    let ctx = unsafe { &mut *ctx };
    // Safety: state_slot is a compile-time constant within local_count; the slot
    // holds an initialized Value (the iteration-state tuple).
    let target_ptr = unsafe { ctx.buffer.add(state_slot) };
    // Safety: target_ptr points to a valid, initialized Value.
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
        // Safety: var_slot is a compile-time constant within local_count.
        let dest_ptr = unsafe { ctx.buffer.add(var_slot) };
        // Safety: dest_ptr holds a valid initialized Value; dropping it is sound
        // because the slot is being reassigned. Writing the new element then takes ownership.
        unsafe {
            std::ptr::drop_in_place(dest_ptr);
            dest_ptr.write(elem);
        }
        // Safety: target_ptr is valid. Drop old iteration state and write updated
        // (collection, next_index) tuple.
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

pub(super) extern "C" fn oxy_vec_index(ctx: *mut JitContext) {
    // Safety: ctx is a valid JitContext from the JIT.
    let ctx = unsafe { &mut *ctx };
    // Safety: pop index and collection from valid operand stack in IR-guaranteed order.
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

    // Map indexing (`m[key]`) keys by the index value itself, not a position.
    match &collection {
        Value::HashMap(rc) => {
            let result = match rc.borrow().get(&index_val) {
                Some(v) => v.clone(),
                None => {
                    set_error(ctx, format!("key not found: {index_val:?}"));
                    Value::Unit
                }
            };
            unsafe { push(ctx, result) };
            return;
        }
        Value::BTreeMap(rc) => {
            let result = match rc.borrow().get(&index_val) {
                Some(v) => v.clone(),
                None => {
                    set_error(ctx, format!("key not found: {index_val:?}"));
                    Value::Unit
                }
            };
            unsafe { push(ctx, result) };
            return;
        }
        _ => {}
    }

    let idx = super::super::runtime::value_to_i64(&index_val) as usize;
    // Indexing past the end is a runtime error, not a silent `Unit`. Each
    // sequence type reports its own length so the message is actionable; a
    // `None` from `.get()` (or `.nth()`) is the out-of-bounds signal.
    let result = match collection {
        Value::Vec(rc) => match rc.borrow().get(idx).cloned() {
            Some(v) => v,
            None => {
                set_error(
                    ctx,
                    format!(
                        "index out of bounds: the len is {} but the index is {idx}",
                        rc.borrow().len()
                    ),
                );
                Value::Unit
            }
        },
        Value::Array(ref a) => match a.get(idx).cloned() {
            Some(v) => v,
            None => {
                set_error(
                    ctx,
                    format!(
                        "index out of bounds: the len is {} but the index is {idx}",
                        a.len()
                    ),
                );
                Value::Unit
            }
        },
        Value::String(ref s) => match s.chars().nth(idx) {
            Some(c) => Value::Char(c),
            None => {
                set_error(
                    ctx,
                    format!(
                        "index out of bounds: the len is {} but the index is {idx}",
                        s.chars().count()
                    ),
                );
                Value::Unit
            }
        },
        Value::Tuple(ref t) => match t.get(idx).cloned() {
            Some(v) => v,
            None => {
                set_error(
                    ctx,
                    format!(
                        "index out of bounds: the len is {} but the index is {idx}",
                        t.len()
                    ),
                );
                Value::Unit
            }
        },
        _ => {
            set_error(ctx, format!("cannot index {collection:?}"));
            Value::Unit
        }
    };
    unsafe {
        push(ctx, result);
    }
}

pub(super) extern "C" fn oxy_vec_index_store(ctx: *mut JitContext) {
    // Safety: ctx is a valid JitContext from the JIT.
    let ctx = unsafe { &mut *ctx };
    // Safety: pop value, index, and collection from valid operand stack in IR order.
    let value = unsafe { pop(ctx) };
    let index_val = unsafe { pop(ctx) };
    let collection = unsafe { pop(ctx) };
    let idx = super::super::runtime::value_to_i64(&index_val) as usize;
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

pub(super) extern "C" fn oxy_make_range(ctx: *mut JitContext, inclusive: usize) {
    // Safety: ctx is a valid JitContext from the JIT.
    let ctx = unsafe { &mut *ctx };
    // Safety: pop end and start from valid operand stack in IR-guaranteed order.
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
