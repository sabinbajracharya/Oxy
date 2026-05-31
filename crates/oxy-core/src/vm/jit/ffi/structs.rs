//! Struct construction and field access FFI — part of the shared oxy_* runtime. See `mod.rs`
//! for the core machinery (push/pop, call stack, ffi_symbols).
//!
//! # Safety
//!
//! All functions are `extern "C"` entry points from Cranelift JIT code. `ctx` is a
//! valid, non-aliased `*mut JitContext`. String pointers from the JIT reference
//! valid memory for the call's duration. `pop`/`push` operate on a pre-allocated
//! operand stack whose capacity is guaranteed.

use super::*;

pub(super) extern "C" fn oxy_struct_init(
    ctx: *mut JitContext,
    name_ptr: *const u8,
    name_len: usize,
    field_names_ptr: *const u8,
    field_names_len: usize,
    field_count: usize,
) {
    // Safety: ctx is a valid, non-aliased JitContext from JIT codegen.
    let ctx = unsafe { &mut *ctx };
    // Safety: JIT-guaranteed valid string buffer for the call's duration.
    let name_bytes = unsafe { std::slice::from_raw_parts(name_ptr, name_len) };
    let name = String::from_utf8_lossy(name_bytes).into_owned();
    // Safety: field_names_ptr/len describe a NUL-separated buffer from the JIT, valid for this call.
    let fn_bytes = unsafe { std::slice::from_raw_parts(field_names_ptr, field_names_len) };
    let field_names: Vec<&str> = fn_bytes
        .split(|b| *b == 0)
        .map(|s| std::str::from_utf8(s).unwrap_or(""))
        .collect();
    let mut fields = HashMap::new();
    for i in (0..field_count).rev() {
        // Safety: each pop drains one field value from the operand stack; field_count
        // is guaranteed by IR codegen to match the number of pushed fields.
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

pub(super) extern "C" fn oxy_struct_update(
    ctx: *mut JitContext,
    field_names_ptr: *const u8,
    field_names_len: usize,
    field_count: usize,
) {
    // Safety: ctx is a valid JitContext pointer from JIT codegen.
    let ctx = unsafe { &mut *ctx };
    // Safety: pop from valid operand stack; base struct is guaranteed present.
    let base = unsafe { pop(ctx) };
    // Safety: JIT-guaranteed valid NUL-separated field name buffer.
    let fn_bytes = unsafe { std::slice::from_raw_parts(field_names_ptr, field_names_len) };
    let field_names: Vec<&str> = fn_bytes
        .split(|b| *b == 0)
        .map(|s| std::str::from_utf8(s).unwrap_or(""))
        .collect();
    let mut overrides = Vec::with_capacity(field_count);
    for _ in 0..field_count {
        // Safety: field_count matches operand stack depth per IR codegen.
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

pub(super) extern "C" fn oxy_field_access(
    ctx: *mut JitContext,
    name_ptr: *const u8,
    name_len: usize,
) {
    // Safety: ctx is a valid JitContext pointer from JIT codegen.
    let ctx = unsafe { &mut *ctx };
    // Safety: pop the struct/tuple value from the valid operand stack.
    let obj = unsafe { pop(ctx) };
    // Safety: name_ptr/name_len describe a valid JIT-owned string buffer.
    let name_bytes = unsafe { std::slice::from_raw_parts(name_ptr, name_len) };
    let name = String::from_utf8_lossy(name_bytes);
    let result = match &obj {
        Value::Struct { fields, .. } => fields.get(name.as_ref()).cloned().unwrap_or(Value::Unit),
        Value::Tuple(ref elements) => {
            if let Ok(idx) = name.parse::<usize>() {
                match elements.get(idx).cloned() {
                    Some(v) => v,
                    None => {
                        // Out-of-range tuple index is a runtime error, not a
                        // silent `Unit` — mirrors sequence indexing in
                        // oxy_vec_index.
                        set_error(
                            ctx,
                            format!(
                                "index out of bounds: the len is {} but the index is {idx}",
                                elements.len()
                            ),
                        );
                        unsafe {
                            push(ctx, Value::Unit);
                        }
                        return;
                    }
                }
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

pub(super) extern "C" fn oxy_field_store(
    ctx: *mut JitContext,
    name_ptr: *const u8,
    name_len: usize,
) {
    // Safety: ctx is a valid JitContext pointer from JIT codegen.
    let ctx = unsafe { &mut *ctx };
    // Safety: pop value and struct from the valid operand stack in IR-guaranteed order.
    let value = unsafe { pop(ctx) };
    let obj = unsafe { pop(ctx) };
    // Safety: JIT-guaranteed valid string buffer for the field name.
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
