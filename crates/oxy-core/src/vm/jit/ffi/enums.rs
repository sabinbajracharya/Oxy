//! Enum construction and inspection FFI — part of the shared oxy_* runtime. See `mod.rs`
//! for the core machinery (push/pop, call stack, ffi_symbols).

use super::*;

pub(super) extern "C" fn oxy_enum_data_get(ctx: *mut JitContext, index: usize) {
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

pub(super) extern "C" fn oxy_enum_variant_equal(
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

pub(super) extern "C" fn oxy_make_enum_variant(
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

pub(super) extern "C" fn oxy_const_enum_variant(
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

/// Resolve a module-level constant path (e.g. `math::PI`, `std::math::PI`) to
/// its value. Errors if the path doesn't name a known module constant.
pub(super) extern "C" fn oxy_module_const(
    ctx: *mut JitContext,
    path_ptr: *const u8,
    path_len: usize,
) {
    let ctx = unsafe { &mut *ctx };
    let path = unsafe {
        let slice = std::slice::from_raw_parts(path_ptr, path_len);
        String::from_utf8_lossy(slice).into_owned()
    };
    let segments: Vec<&str> = path.split("::").collect();
    let lookup = match segments.as_slice() {
        [module, name] => crate::stdlib::registry::lookup_constant(module, name),
        ["std", module, name] => crate::stdlib::registry::lookup_constant(module, name),
        _ => None,
    };
    match lookup {
        Some(val) => unsafe { push(ctx, val) },
        None => {
            set_error(ctx, format!("unknown constant: {path}"));
            unsafe { push(ctx, Value::Unit) };
        }
    }
}
