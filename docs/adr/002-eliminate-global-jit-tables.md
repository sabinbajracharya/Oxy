# ADR 002: Eliminate Global JIT Tables via Per-Engine `JitTables`

## Status

Accepted (May 2026)

## Context

The JIT backend used 9 global `OnceLock<Mutex<...>>` tables in `ffi.rs` to share compilation output between the compiler (which writes the tables) and the runtime FFI functions (which read them):

| Table | Type | Purpose |
|-------|------|---------|
| `FN_TABLE` | `HashMap<usize, usize>` | fn_index → native pointer |
| `FN_LOCAL_COUNTS` | `HashMap<usize, usize>` | fn_index → local slot count |
| `CLOSURE_NAME_TO_INDEX` | `HashMap<String, usize>` | function name → fn_index |
| `CLOSURE_META` | `Vec<ClosureRuntimeMeta>` | per-closure capture metadata |
| `ASYNC_FN_META` | `Vec<AsyncFnMeta>` | async function metadata (dead code) |
| `BUILTIN_PATHS` | `Vec<Vec<String>>` | builtin path segments (dead code) |
| `STRUCT_INIT_META` | `Vec<StructInitMeta>` | struct init metadata (dead code) |
| `CONST_ENUM_VARIANTS` | `Vec<...>` | const enum variant data (dead code) |
| `METHOD_IPS` | `HashMap<(String,String), usize>` | method dispatch table (dead code) |

These were written during `JitEngine::compile()` via `set_*` functions and read at runtime by FFI functions like `oxy_call_closure`, `oxy_push_closure`, etc. A `COMPILE_LOCK` mutex serialized compilation to prevent races on the globals.

### Problems

1. **Global mutable state**: The tables were `OnceLock<Mutex<...>>` statics. Every test run shared the same tables, causing test isolation issues and requiring a `reset_runtime_state()` workaround.

2. **Mutex overhead at runtime**: Every table lookup acquired a mutex lock, even though the tables are read-only after compilation.

3. **Unclear ownership**: The tables were populated by `JitEngine::compile()` but lived in global statics. When the engine was dropped, stale data remained in the globals.

4. **5 of 9 tables were dead code**: Never written to, never read from. They accumulated as the codebase evolved and were never cleaned up.

5. **`COMPILE_LOCK` prevented parallel compilation**: The mutex existed solely to serialize writes to the global tables.

## Decision

Replace all 9 global tables with a single `JitTables` struct owned by `JitEngine`. A `*const JitTables` pointer on `JitContext` gives every FFI function access without globals or mutexes.

### Architecture

```
JitEngine {
    tables: JitTables,              // OWNS the tables (dropped with engine)
    functions: HashMap<String, *const u8>,
    ...
}

JitContext {
    ...
    tables: *const JitTables,       // BORROWS from engine (read-only at runtime)
    ...
}

JitTables {
    fn_table: HashMap<usize, usize>,
    fn_local_counts: HashMap<usize, usize>,
    name_to_index: HashMap<String, usize>,
    closure_meta: Vec<ClosureRuntimeMeta>,
}
```

### Lifetime Safety

The `*const JitTables` pointer is safe because all JIT function calls are synchronous within a frame established by `JitVm::run()` or `JitVm::run_function()`. The `JitEngine` (which owns `JitTables`) outlives every `JitContext`. The one async path (`oxy_spawn_ffi`) runs tasks eagerly and synchronously — the result is available before the engine is dropped.

### Access Pattern Change

```rust
// Before: global mutex lookup
fn_table_lock().get(&ip).map(|&p| p as *const u8)

// After: pointer dereference
unsafe { &*ctx.tables }.fn_ptr(ip)
```

The `JitTables` struct provides safe accessor methods (`fn_ptr()`, `local_count()`, `name_to_index()`, `closure_meta()`) that encapsulate the unsafe pointer dereference.

## Alternatives Considered

| Approach | Global state | Mutex overhead | Implementation | Verdict |
|----------|-------------|----------------|----------------|---------|
| A: `*const JitTables` on JitContext | None | None | Medium | **Chosen** |
| B: Thread-local storage | Yes (per-thread) | None | Low | Doesn't fix ownership |
| C: Embed tables in JitContext | None | None | High | Breaks `#[repr(C)]` layout |
| D: Global AtomicPtr swap | Yes (one pointer) | None | Low | Fragile lifetime management |

Approach A was chosen because it is the only one that eliminates global state, has no runtime overhead, and is practical to implement.

## Consequences

### Benefits

1. **No global mutable state**: Tables are owned by `JitEngine` and dropped with it. Two engines can coexist without interference.

2. **No mutex overhead at runtime**: Table reads are raw pointer dereferences instead of mutex locks.

3. **No `COMPILE_LOCK` needed**: Since tables are per-engine, parallel compilation is now possible (though not yet enabled).

4. **Clear ownership**: The data flow is obvious — `JitEngine` owns the tables, `JitContext` borrows them.

5. **461 lines of dead code removed**: All 9 global statics, their lock functions, setter functions, and lookup wrappers were deleted. The dead `oxy_make_future` function was also removed.

6. **Test isolation by construction**: Each `JitEngine` has its own tables. No state leaks between test runs.

### Costs

- `JitContext` gained one pointer field (8 bytes on 64-bit).
- `jit_closure_invoker` and related functions gained a `tables: &JitTables` parameter.
- 3 JitContext construction sites must set `ctx.tables`.

## Lessons Learned

1. **Prefer explicit data flow over globals, even with synchronization.** The `OnceLock<Mutex<...>>` pattern is seductive for "write once, read many" data. But it obscures ownership, prevents parallel compilation, and makes test isolation fragile. Passing a pointer through context is simpler and more robust.

2. **Compile-time and runtime data should have clear ownership boundaries.** The compilation output (fn pointers, local counts, metadata) is fundamentally owned by the compilation unit, not by the process. The natural owner is the engine struct.

3. **Dead code accumulates when ownership is unclear.** The 5 dead tables existed because nobody knew who was responsible for cleaning them up. When the tables were global, "just leave it" was the path of least resistance. Explicit ownership on a struct makes it obvious what's alive and what's dead.

4. **A pointer field on a context struct is the simplest FFI data channel.** Every FFI function already receives `*mut JitContext`. Adding one field makes new data available to the entire FFI surface with zero signature changes. This pattern should be preferred over new global statics.
