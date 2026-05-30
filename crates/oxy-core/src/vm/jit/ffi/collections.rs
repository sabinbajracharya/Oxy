//! Collection, iterator, and range FFI — part of the shared oxy_* runtime. See `mod.rs`
//! for the core machinery (push/pop, call stack, ffi_symbols).

use super::*;

pub(super) extern "C" fn oxy_make_array(ctx: *mut JitContext, count: usize) {
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

pub(super) extern "C" fn oxy_make_fixed_array(ctx: *mut JitContext, count: usize) {
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

pub(super) extern "C" fn oxy_make_tuple(ctx: *mut JitContext, count: usize) {
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

pub(super) extern "C" fn oxy_make_repeat(ctx: *mut JitContext) {
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

pub(super) extern "C" fn oxy_iter_next_destructure(ctx: *mut JitContext, state_slot: usize) -> i64 {
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

pub(super) extern "C" fn oxy_make_iter(ctx: *mut JitContext) {
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

pub(super) extern "C" fn oxy_iter_len(ctx: *mut JitContext) {
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

pub(super) extern "C" fn oxy_iter_next(
    ctx: *mut JitContext,
    state_slot: usize,
    var_slot: usize,
) -> i64 {
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

pub(super) extern "C" fn oxy_vec_index(ctx: *mut JitContext) {
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
    let ctx = unsafe { &mut *ctx };
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
