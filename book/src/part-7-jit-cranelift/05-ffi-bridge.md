# The FFI Bridge: How Rust Becomes a Runtime

<!-- OPUS_FILL
Write a 2-paragraph intro.
The FFI bridge is the shared runtime — the layer that both the JIT and the wasm interpreter
call into. It is the single source of truth for what Oxy operations DO.

Reference the key insight: "one runtime, both backends." The JIT calls these functions as
native machine code calls. The interpreter dispatches to the same function pointers.
The semantics cannot diverge because they share the implementation.
-->

## The `oxy_*` function family

Every runtime operation in Oxy is implemented as a `#[no_mangle] extern "C"` function
in `vm/jit/ffi/mod.rs` (and its submodules). These are the `oxy_*` functions:

| Function | What it does |
|---------|-------------|
| `oxy_push_int(ctx, val)` | Push an integer onto the operand stack |
| `oxy_push_bool(ctx, val)` | Push a boolean |
| `oxy_push_unit(ctx)` | Push unit |
| `oxy_add(ctx)` | Pop two, add, push result |
| `oxy_println_val(ctx)` | Pop one, print it |
| `oxy_struct_init(ctx, field_count, name_ptr, name_len, ...)` | Create a struct |
| `oxy_method_call(ctx, method_ptr, method_len, ...)` | Call a method on the top value |
| `oxy_call(ctx, fn_ptr, fn_len, ...)` | Call a user-defined function |
| `oxy_call_closure(ctx)` | Call the closure on top of the stack |
| `oxy_make_enum_variant(ctx, ...)` | Create an enum variant |
| `oxy_push_named_fn(ctx, name_ptr, name_len)` | Push a function value |
| `oxy_load_local(ctx, slot)` | Load a value from a local slot |
| `oxy_store_local(ctx, slot)` | Store top of stack into a local slot |
| ... | (100+ functions total) |

**Files:**
- `ffi/mod.rs` — most operations (~2200 lines)
- `ffi/collections.rs` — Vec, HashMap, BTreeMap operations
- `ffi/structs.rs` — struct init, field access, update
- `ffi/enums.rs` — enum construction, matching
- `ffi/strings_fmt.rs` — string operations, formatting
- `ffi/casts.rs` — type casts (`as int`, `as float`, `as byte`)

## The calling convention

Every FFI function takes `ctx: *mut JitContext` as its first argument. Arguments are
passed via the operand stack in `ctx`, not as regular function parameters (except for
immediates and string pointers).

The pattern for a binary operation:

```
push arg1 onto operand stack (by JIT-compiled code or by caller FFI)
push arg2 onto operand stack
call oxy_foo(ctx)
   → pop arg2 from stack
   → pop arg1 from stack
   → compute result
   → push result onto stack
```

The pattern for a function with immediate metadata:

```
push value_arg onto operand stack (if any)
call oxy_struct_init(ctx, field_count, name_ptr, name_len, field1_ptr, field1_len, ...)
   → pop field_count values from stack (the field values)
   → create Value::Struct with name and fields
   → push result onto stack
```

String metadata (function names, field names, method names) is passed as `(ptr, len)` pairs
of raw bytes — not Rust `&str` (which has lifetime requirements). The FFI function
reconstructs the `&str` from the raw pointer and length.

## How the JIT calls FFI functions

The JIT calls FFI functions through Cranelift's call mechanism:

```rust
// In codegen.rs
let fref = ffi_refs["oxy_add"];  // Cranelift FuncRef for oxy_add
builder.ins().call(fref, &[ctx]);  // emit a call instruction
```

Cranelift emits a native `call` instruction targeting the Rust function's address. The
address is resolved when `module.finalize_definitions()` patches the call sites.

## How the interpreter calls FFI functions

The wasm interpreter uses a function pointer table:

```rust
// From ffi/mod.rs
pub fn ffi_symbols() -> HashMap<&'static str, *const u8> {
    let mut m = HashMap::new();
    m.insert("oxy_push_int", oxy_push_int as *const u8);
    m.insert("oxy_push_bool", oxy_push_bool as *const u8);
    m.insert("oxy_add", oxy_add as *const u8);
    // ... all oxy_* functions
    m
}
```

When the interpreter encounters `IrOp::CallBuiltin { func: "oxy_add", ... }`, it:
1. Looks up `"oxy_add"` in `ffi_symbols()`
2. Gets a `*const u8` — the raw function pointer
3. Transmutes it to the correct function signature
4. Calls it

Same function. Different calling mechanism. Identical semantics.

## The consistency guard

`ffi_decls()` (in `codegen.rs`) and `ffi_symbols()` (in `ffi/mod.rs`) are two independent
lists of FFI functions. A consistency test verifies they describe the same set:

```rust
// From jit/mod.rs tests
#[test]
fn ffi_consistency() {
    let decls: HashSet<&str> = ffi_decls().iter().map(|(name, ..)| *name).collect();
    let symbols: HashSet<&str> = ffi_symbols().keys().copied().collect();
    assert_eq!(decls, symbols, "ffi_decls and ffi_symbols must list the same functions");
}
```

Add a new `oxy_*` to `ffi_symbols()` but forget `ffi_decls()` → the consistency test fails.
Add it to `ffi_decls()` but forget `ffi_symbols()` → the interpreter can't find it.
Add it to both → works on both backends.

This is "guard #2" from CLAUDE.md: the FFI surface consistency test.
