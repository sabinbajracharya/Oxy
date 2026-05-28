# ADR 001: Unified JIT Function Calling

## Status

Accepted (May 2026)

## Context: The Dual-Path Architecture

The Oxy JIT compiles Oxy source code to native code via Cranelift. Every Oxy function call — whether to a named function like `greet::yo()`, a closure like `f()`, or a higher-order parameter — must go through the JIT's FFI layer to set up a buffer, pass arguments, and invoke the compiled native code.

The implementation had **two separate calling paths**:

### Path A: `oxy_call` — call by name

```
ir_gen:  CallBuiltin { func: "oxy_call", strings: ["greet::yo"], immediates: [arg_count] }
         ↓
ffi:     oxy_call(ctx, name_ptr, name_len, arg_count)
           → lookup CLOSURE_NAME_TO_INDEX["greet::yo"] → fn_index
           → lookup FN_TABLE[fn_index] → native pointer
           → lookup FN_LOCAL_COUNTS[fn_index] → local_count
           → invoke_jit_fn(ctx, ptr, local_count, arg_count)
               → allocate callee buffer (local_count + 2048)
               → move args from caller stack to callee buffer[0..N]
               → swap ctx.buffer to callee
               → call native fn(ctx)
               → drop callee locals/stack, dealloc, restore caller
```

### Path B: `oxy_call_closure` — call by value

```
ir_gen:  CallBuiltin { func: "oxy_call_closure", args: [closure_reg, arg_regs...], immediates: [arg_count] }
         ↓
ffi:     oxy_call_closure(ctx, arg_count)
           → read Value::Function from ctx.buffer[ctx.sp - arg_count - 1]
           → extract target_ip (fn_index), captured_names, closure_env
           → lookup FN_TABLE[target_ip] → native pointer
           → allocate callee buffer
           → write captures from closure_env to callee buffer[0..N]
           → write args to callee buffer[N..N+arg_count]
           → swap ctx.buffer to callee
           → call native fn(ctx)
           → drop callee locals/stack, dealloc, restore caller
```

### How ir_gen Decided Which Path

The ir_gen used a field called `local_closure_names: HashMap<usize, (String, bool)>` to track which local variables held closures and whether those closures captured outer variables:

```rust
// In Stmt::Let handler (when assigning a closure to a variable):
let is_closure = matches!(val, crate::ast::Expr::Closure { .. });
let reg = self.gen_expr(val);  // generates the closure, pushes IrFunction
if is_closure {
    let closure_fn = self.functions.last().unwrap();
    let has_captures = !closure_fn.captures.is_empty();
    self.local_closure_names.insert(slot, (closure_fn.name, has_captures));
}

// In Expr::Call handler (when calling f()):
if let Some((closure_name, has_captures)) = self.local_closure_names.get(&slot) {
    if has_captures {
        // → oxy_call_closure (Path B) — sets up captures
    } else {
        fname = closure_name;
        // → oxy_call (Path A) — calls by name, no capture setup
    }
} else if local exists {
    // → oxy_call_closure (Path B) — parameter or dynamic closure
} else {
    // → oxy_call (Path A) — named function
}
```

## The Problem

### The Immediate Bug

Locally-defined closures with captures were routed through `oxy_call` (Path A), which calls the compiled function directly by name but **does not populate capture slots**. The closure function was compiled expecting captured values in local slots 0..N, but those slots contained zeroed memory. The result: captured values were garbage.

For example, this Oxy code produced wrong results:

```rust
let n = 10;
let f = || n * 2;
assert_eq!(f(), 20);  // got 0, because n read as 0 from uninitialized buffer
```

### The Deeper Problem

The ir_gen — which translates AST into register IR — had to make a runtime calling convention decision. It had to know:

1. Is this local a closure? → check `local_closure_names`
2. Does it have captures? → check the `bool` flag
3. If yes, use Path B. If no, use Path A.
4. Is it a parameter? → use Path B.
5. Is it a named function? → use Path A.

This is **layer violation**. The IR generator's job is to translate language semantics into a linear IR. It should not know about FFI function dispatch, buffer layouts, or capture setup. Those are codegen/runtime concerns.

### Other Symptoms

1. **Flaky tests**: Global OnceLock tables (`FN_TABLE`, `CLOSURE_NAME_TO_INDEX`, `SCHEDULER`) persist across test runs, causing state leaks.
2. **Duplicate buffer management**: `invoke_jit_fn` and `oxy_call_closure` both alloc/swap/call/drop/dealloc, with slightly different logic.
3. **Fragile ordering**: `self.functions.last()` to find the closure name assumed no intervening function pushes (breaks on nested closures).
4. **`move_value` not used uniformly**: `invoke_jit_fn` used it to transfer args safely; `oxy_call_closure` used raw `ptr::read` without source clearing.

## Why the Original Architecture Was Chosen

The original design evolved incrementally:

1. **First**: `oxy_call` was built for named function calls — look up by name, invoke via `invoke_jit_fn`. Simple and functional.

2. **Then**: Closures were added. A closure is a function PLUS captured variables. Rather than refactoring the calling path, a **second path** `oxy_call_closure` was added that reads captures from the `Value::Function`'s environment and writes them into the callee buffer before calling.

3. **Then**: To avoid "always using the slower path," a fast-path was added: if a closure has no captures, skip `oxy_call_closure` and use `oxy_call` by name. This optimization introduced the `local_closure_names` tracking and the branching logic.

Each step was reasonable in isolation. The architectural problem emerged from accretion — a second path was added alongside the first, then a third decision was layered on top (captures or not?), and the result was fragile.

## The Fix: Unify on a Single Path

### Principle

**All function values are called the same way. The ir_gen emits "here is a function value and some arguments — call it." The runtime handles everything else.**

### Implementation

1. **`oxy_push_named_fn(name)`** — new FFI function that creates a `Value::Function { target_ip, closure_env: empty, captured_names: [] }` for any compiled function. This lets named functions be represented as function values.

2. **ir_gen simplification**: The `Expr::Call` handler becomes:
   ```
   callee_reg = if callee_is_local { LoadLocal(slot) } else { oxy_push_named_fn(name) }
   oxy_call_closure(callee_reg, args...)
   ```
   No branching on closure vs named vs captures. `local_closure_names` deleted.

3. **`oxy_call` deleted**. All calls go through `oxy_call_closure`.

4. **`CalleeFrame` buffer abstraction**: Encapsulates buffer alloc/swap/call/drop/dealloc/restore, eliminating the duplicated unsafe pattern that was copied across `invoke_jit_fn`, `oxy_call_closure`, and `oxy_await_ffi`.

5. **Async closure fix**: `gen_closure` always emits `oxy_push_closure` (creates `Value::Function` with `is_async` flag). `oxy_call_closure` creates the `Future` at call time. Async blocks (`async {}`) still use `oxy_push_async_block` to create Futures directly.

### Before vs After

**Named function call `greet::yo()`:**

Before:
```
r0 = Call oxy_call(args=[], imm=[0], strs=["greet::yo"])
```
After:
```
r0 = Call oxy_push_named_fn(args=[], imm=[], strs=["greet::yo"])
r1 = Call oxy_call_closure(args=[0], imm=[0], strs=[])
```

**Closure with captures `f()` where `let f = || n * 2`:**

Before:
```
r0 = Call oxy_push_closure(args=[], imm=[0], strs=["closure_0"])
StoreLocal(0, r0)
r1 = Call oxy_call(args=[], imm=[0], strs=["closure_0"])   ← BUG: no captures
```
After:
```
r0 = Call oxy_push_closure(args=[], imm=[0], strs=["closure_0"])
StoreLocal(0, r0)
r1 = LoadLocalRaw(0)
r2 = Call oxy_call_closure(args=[1], imm=[0], strs=[])     ← captures set up from closure value
```

## Benefits

1. **Eliminates the bug class**: No dispatch decision → no wrong dispatch. The "forgot to set up captures" bug cannot exist because there is only one path and it always sets up captures.

2. **Simpler ir_gen**: ~50 lines of branching logic replaced by a straight-line sequence. The `local_closure_names` field is deleted.

3. **Fixes nested closure issue**: `self.functions.last()` hack is no longer needed since we don't predict closure names.

4. **Fixes flaky tests**: Scheduler reset between compilations ensures clean state.

5. **Shared buffer abstraction**: `CalleeFrame` eliminates the duplicated unsafe alloc/swap/call/drop pattern across 4 call sites.

6. **Easier to evolve**: Adding a new call feature requires changes in ONE place, not two.

## Cost

- Named function calls do one extra push/pop (creating the `Value::Function`). Negligible compared to Cranelift call overhead and buffer allocation.
- `CLOSURE_NAME_TO_INDEX` table is kept (still needed by `oxy_push_named_fn`, `oxy_push_closure`).
- `oxy_path_call_builtin` and `oxy_method_call` stay as separate paths — they do type-based dispatch, not simple function calls.

## Results

| Test Suite | Before | After |
|-----------|--------|-------|
| Lib tests | 429/429 | 429/429 |
| Feature examples | 1225/1429 | 1247/1429 |
| Crashes (SIGSEGV/SIGABRT) | intermittent | 0 |

## Lessons Learned

1. **Two paths for the same operation is a bug factory.** If you add a second way to do something, every future change must remember to update both. The `oxy_call` vs `oxy_call_closure` split meant captures had to be handled in both paths — but they were only handled in one.

2. **The IR layer should not make runtime calling convention decisions.** When the AST → IR translation has to decide which FFI function to call based on runtime properties (does this value have captures?), that decision belongs in the runtime. The IR should express *what* to call, not *how*.

3. **Incremental accretion creates invisible architecture.** Each step (add closures, then optimize no-capture fast path) was reasonable alone. Together they created a fragile system. Periodically step back and ask: "Does this still have one clear responsibility, or has it accumulated hidden complexity?"

4. **Global mutable state in compilers is a test isolation risk.** The OnceLock tables were convenient but made test state non-local. A `reset()` function is a minimal fix; a longer-term improvement would be to pass a `CompilationUnit` struct through the pipeline instead of using globals.

5. **Prefer data over dispatch.** If every function value carries its own `target_ip` (as `Value::Function` already does), you don't need a separate name-based lookup path. The data tells you what to call. The dispatch logic becomes: read the data, follow the pointer.
