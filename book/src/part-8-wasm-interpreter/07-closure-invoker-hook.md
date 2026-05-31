# The Closure-Invoker Hook

This is the cleverest single piece of the wasm interpreter, and it solves a problem that the
"just call the shared FFI" story quietly skips over. Most ops are easy: the interpreter calls
`oxy_add`, `oxy_add` does its thing, done. But some builtins need to call back into *user* code.
Think about `vec.map(|x| x * 2)`. The implementation of `map` lives in Rust, and somewhere in the
middle of its loop it has to invoke that closure — code the user wrote, code that exists as an
`IrFunction`. On native, the closure carries an index into the JIT's function-pointer table, and
`map` just calls through it into compiled machine code. On wasm, that table is *empty* — there is
no machine code, no native pointer to call. The closure is right there, but the runtime has no way
to run it.

The fix is a thread-local hook, and the framing that makes it click is this: the shared runtime
doesn't know, and shouldn't know, which backend is driving it. So at startup the interpreter
essentially leaves a note — *"when you need to call a user function and you can't find a native
pointer, call me instead"* — by installing a callback that runs a function's IR through the
interpreter. When `map`'s call into the closure misses the empty function table, it falls back to
that hook, and the hook re-enters the interpreter to run the closure's body. One callback turns
"the runtime can't reach user code on wasm" into "the runtime reaches user code the same way on
both backends, it just lands in a different executor." It's the piece that closed the last parity
gaps — higher-order builtins, async, user `Display::fmt` — and it's worth understanding in detail.

## The problem: callees inside the shared runtime

Consider `Vec::map(closure)`. The implementation in `ffi/collections.rs` (simplified):

```rust
extern "C" fn oxy_vec_map(ctx: *mut JitContext) {
    let ctx = unsafe { &mut *ctx };
    let closure_val = unsafe { ffi::pop(ctx) };
    let vec_val = unsafe { ffi::pop(ctx) };

    let results: Vec<Value> = vec.iter().map(|item| {
        // Push item as argument
        unsafe { ffi::push(ctx, item.clone()); }
        // Call the closure — HOW?
        ffi::jit_closure_invoker(ctx, &closure_val);
        // Pop result
        unsafe { ffi::pop(ctx) }
    }).collect();
}
```

`ffi::jit_closure_invoker(ctx, &closure_val)` needs to call the closure. On native:
the closure holds a `fn_index`, and `invoke_jit_fn(fn_index, ctx)` calls the native pointer.
On wasm: `fn_table[fn_index]` is null (no native pointers). The call fails silently or panics.

## The hook mechanism

The solution: a thread-local callback:

```rust
// ffi/mod.rs
thread_local! {
    static INTERP_INVOKE: Cell<Option<unsafe fn(*mut JitContext, *const u8) -> ()>>
        = Cell::new(None);
}

pub fn set_interp_invoke(f: Option<unsafe fn(*mut JitContext, *const u8) -> ()>) {
    INTERP_INVOKE.with(|h| h.set(f));
}
```

The interpreter installs this hook at startup:

```rust
// interp.rs (in InterpEngine::compile)
let engine_ptr = self as *const _ as *const u8;
ffi::set_interp_invoke(Some(interp_invoke_fn));
```

Where `interp_invoke_fn` is a raw function that casts the pointer back to `&InterpEngine`
and calls `engine.interpret(ctx, func)`.

## How `jit_closure_invoker` uses the hook

```rust
// ffi/mod.rs
pub(crate) fn jit_closure_invoker(ctx: &mut JitContext, closure: &Value) {
    let fn_index = match closure {
        Value::Function(f) => match f.body {
            FunctionBody::Compiled { fn_index, .. } => fn_index,
            _ => return,
        },
        _ => return,
    };

    // Try the native JIT table first
    if let Some(ptr) = ctx.tables.fn_table.get(&fn_index) {
        if !ptr.is_null() {
            unsafe { invoke_native(ptr, ctx); }
            return;
        }
    }

    // fn_table miss — use the interpreter hook
    INTERP_INVOKE.with(|h| {
        if let Some(f) = h.get() {
            let target_ip = fn_index as *const u8;
            unsafe { f(ctx as *mut _, target_ip); }
        }
    });
}
```

On native: native pointer found → call directly. On wasm: no native pointer → call the hook.
The hook is the interpreter's `interpret` method, which runs the function's IR.

## What the hook enables

With the hook installed, the following work correctly on the interpreter:

**Higher-order builtins:**
- `vec.map(|x| x * 2)` → `oxy_vec_map` loops, calls `jit_closure_invoker` per element
- `vec.filter(|x| x > 0)` → same pattern
- `vec.fold(0, |acc, x| acc + x)` → same
- `Option::map(some_fn)`, `Result::and_then(fn)` → same

**Async:**
- `spawn(async_fn)` → task body called through the hook
- `.await` on a `Future` → resolves by calling the task body through the hook

**User `Display::fmt`:**
- `println(my_struct)` → calls `Type::fmt` through the hook

Before the hook, all of these were `unsupported_on_wasm`. After the hook, all of them
reach full parity with the JIT. This is why the CLAUDE.md says:

> *"Known interpreter gaps: None. The whole `examples/features/**` corpus is at parity."*

## The JIT doesn't need the hook

On native, the JIT installs no hook (`ffi::set_interp_invoke(None)`). The JIT always has
native function pointers in `fn_table`. Every function call goes through the pointer. The
hook call site in `jit_closure_invoker` — the `INTERP_INVOKE.with(...)` branch — is never
reached.

One code path. One `jit_closure_invoker`. The only difference: whether the native pointer
is found in the first branch (JIT) or the hook is called (interpreter).
