# The Closure-Invoker Hook

<!-- OPUS_FILL
Write a 2-paragraph hook. This is the most architecturally interesting piece of the
wasm interpreter, and it deserves careful explanation.

The problem: higher-order builtins like `map` and `filter` are implemented in Rust.
They need to call a user-provided closure. On native, they call it through the JIT's
function pointer table. On wasm, the table is empty — no native pointers exist.

The solution: a thread-local callback that the interpreter installs at startup.
When the runtime needs to call a closure and the function table has no pointer,
it calls the hook instead. The hook calls back into the interpreter.

Frame it as: "The runtime doesn't know which backend is running it. The hook is how
the interpreter announces 'when you need to call a function, call me.'"
-->

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
