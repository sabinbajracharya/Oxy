# Rust Concepts: Unsafe, Raw Pointers, and Memory

<!-- OPUS_FILL
Write a 2-paragraph intro.
`unsafe` in Rust is not a red flag — it is an explicit acknowledgment of "here be dragons."
The Rust compiler cannot verify these operations, so the programmer takes responsibility.
The JIT's FFI layer uses unsafe extensively and correctly.

Frame it as: unsafe is not about doing dangerous things unsafely. It is about doing the
same inherently-dangerous things (raw memory access, C interop, pointer arithmetic) but
with explicit acknowledgment of the responsibility. Good unsafe code is well-reasoned
and well-commented. Oxy's FFI code is that.
-->

## What `unsafe` means in Rust

`unsafe` in Rust unlocks four extra capabilities:
1. Dereference raw pointers (`*const T`, `*mut T`)
2. Call `unsafe fn` (functions that have preconditions the compiler can't verify)
3. Access mutable static variables
4. Implement `unsafe trait`

Everything else — borrow checking, lifetime checking, type safety — still applies. `unsafe`
is a block that says "I, the programmer, have verified that this code is sound even though
the compiler cannot."

In Oxy's JIT, `unsafe` is used in exactly the right places:

```rust
// Dereference a *mut JitContext passed from compiled code
extern "C" fn oxy_push_int(ctx: *mut JitContext, val: i64) {
    let ctx = unsafe { &mut *ctx };  // dereference the raw pointer
    unsafe { push(ctx, Value::I64(val)); }
}
```

The `*mut JitContext` comes from Cranelift-compiled code that follows the calling convention.
The compiler cannot verify this — Cranelift's code is not type-checked by Rust. But we,
the authors, know the pointer is valid because we set it up before calling the compiled function.

## Raw pointers vs Rust references

Rust references (`&T`, `&mut T`) come with guarantees:
- The referenced value is valid (not freed, not uninitialized)
- The reference follows the aliasing rules

Raw pointers (`*const T`, `*mut T`) have no guarantees — they might be null, dangling, or aliased.
The programmer is responsible for ensuring safety before dereferencing them.

The FFI layer uses raw pointers because:
1. `JitContext` is passed as `*mut JitContext` through a C ABI (`extern "C"`)
2. C doesn't have Rust's reference semantics — Cranelift generates C-ABI code
3. The lifetime of `JitContext` is managed by the JIT engine, not by any single Rust reference

## The `JitContext` buffer

The most significant unsafe code in Oxy is the `JitContext` buffer — a raw memory buffer
that holds both local variable slots and the operand stack for a single function call:

```rust
// crates/oxy-core/src/vm/jit/context.rs
pub struct JitContext {
    pub buffer: *mut Value,   // raw pointer to allocated memory
    pub local_count: usize,   // number of local slots
    pub sp: usize,            // operand stack pointer
    pub result: Value,        // return value
    pub error_len: usize,     // non-zero if error is set
    pub error_msg: [u8; 1024], // error message buffer
    // ...
}
```

The buffer layout:
```
[locals[0], locals[1], ..., locals[n-1], stack[0], stack[1], ..., stack[sp-1]]
```

The locals occupy indices `[0, local_count)`. The operand stack grows upward from
`local_count`. When the stack pointer (`sp`) and locals don't overlap, everything is safe.

The `push` and `pop` operations:

```rust
// ffi/mod.rs
pub(crate) unsafe fn push(ctx: &mut JitContext, val: Value) {
    let slot = ctx.push_slot();  // buffer[local_count + sp]; sp += 1
    unsafe { slot.write(val); }  // write Value into the slot
}

pub(crate) unsafe fn pop(ctx: &mut JitContext) -> Value {
    ctx.sp -= 1;
    let src = unsafe { ctx.buffer.add(ctx.local_count + ctx.sp) };
    let val = unsafe { src.read() };
    unsafe { src.write(Value::Unit) };  // clear to prevent double-free
    val
}
```

The `write` and `read` on raw pointers are the unsafe operations. They bypass Rust's
ownership checks — but they are correct because:
- `push_slot` returns a valid uninitialized slot (sp just incremented)
- `read` in `pop` returns the value and clears the slot (preventing double-free)
- The caller ensures push/pop are balanced

## The "clear after move" invariant

A subtle but critical invariant: when a `Value` is moved out of the buffer with `pop`,
the source slot must be cleared:

```rust
unsafe { src.write(Value::Unit) };  // ← this line matters
```

If the source slot were not cleared, the `Value` at that location would be dropped twice:
once when the returned `Value` is dropped by the caller, and once when `JitContext` itself
is dropped (which drops all values in the buffer). A double-drop on a `String` or `Vec`
means freeing the same memory twice — undefined behavior and likely a crash.

The CLAUDE.md anti-patterns section documents this as the `move_value` helper rationale:

> *"two of the three had the 'forgot to clear source' double-free bug. The moment you
> recognize a repeated unsafe pattern, encode the invariant once so it can't be forgotten
> at the next call site."*

## `extern "C"` and the C ABI

All FFI functions use `extern "C"`:

```rust
#[no_mangle]
extern "C" fn oxy_push_int(ctx: *mut JitContext, val: i64) {
```

`extern "C"` means "use the C calling convention." Cranelift emits code that calls these
functions using C's calling convention (specific registers for arguments, specific registers
for return values, specific stack frame layout). The Rust function must match this convention
exactly — which `extern "C"` ensures.

`#[no_mangle]` prevents Rust's name mangling (which would change `oxy_push_int` to something
like `_ZN3oxy9push_int17h123abc`). Cranelift looks up functions by their exact name, so
the name must not be mangled.
