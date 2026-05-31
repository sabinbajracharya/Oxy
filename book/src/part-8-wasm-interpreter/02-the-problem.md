# The Problem: Cranelift Cannot Run in a Browser

<!-- OPUS_FILL
Write a 1-paragraph hook. Just frame the problem cleanly. "The JIT works great on native.
And then someone asked: can we have a browser playground? And the answer was: not like this."
Short and punchy — the solution chapter is where the content lives.
-->

## What "running Oxy in the browser" requires

To run Oxy code in a browser, Oxy itself must be compiled to WebAssembly. The compilation:

```bash
cargo build --target wasm32-unknown-unknown --no-default-features -p oxy-core
```

The `wasm32-unknown-unknown` target means: compile Rust to WebAssembly, targeting the
browser's wasm virtual machine.

This works for most Rust code. The problems arise from three Cranelift-specific needs:

**1. Executable memory mapping**

Cranelift needs `mmap(PROT_EXEC)` to create executable regions. The `cranelift-jit` crate
pulls in platform-specific code to do this. On wasm32, `mmap` does not exist — the wasm
runtime manages all memory.

Cargo error: `cranelift-jit` has no wasm32 target support. Compilation fails.

**2. The JIT module architecture**

`JITModule` from `cranelift_jit` requires a real OS process with proper memory layout.
A wasm program does not have this — it runs in the wasm virtual machine's own memory model.

**3. Native code pointers**

The JIT produces `*const u8` function pointers to native machine code. In wasm, there is
no native machine code — only wasm instructions. A `*const u8` to "native code" is meaningless.

## The Cargo feature gate

Oxy handles this with `#[cfg(target_arch = "wasm32")]`:

```rust
// crates/oxy-core/src/vm/api.rs
pub fn run_compiled(source: &str) -> VmResult {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Use the JIT on native
        JitEngine::compile_and_run(source)
    }
    #[cfg(target_arch = "wasm32")]
    {
        // Use the IR interpreter on wasm
        InterpEngine::compile_and_run(source)
    }
}
```

The same public API. Different implementation selected at compile time. When compiled for
native, the JIT is used. When compiled for wasm32, the interpreter is used.

This is how Oxy has one codebase that runs on both platforms — the selection is a compile-time
`#[cfg]` check, not a runtime branch.

## The WASM build in CI

The CLAUDE.md pre-commit checklist includes:

```bash
rustup target add wasm32-unknown-unknown
cargo check --target wasm32-unknown-unknown -p oxy-core --no-default-features
```

This verifies that Oxy compiles for wasm32 without errors. It does not run the tests on wasm
(that would require a wasm runtime in CI), but it catches compilation failures — including
any accidentally-introduced dependency on JIT-only code paths.

If `std::thread::sleep` sneaks into a code path that runs on wasm32, this check fails
(`sleep` calls `unreachable!()` on wasm). The check is the gatekeeper.
