# Cranelift: The Rust-Native Code Generator

<!-- OPUS_FILL
Write a 2-paragraph intro.
Cranelift is the code generator that powers both wasmtime (the WebAssembly runtime used by
many production systems) and Firefox's SpiderMonkey JavaScript engine. It is written in
Rust and designed specifically for JIT compilation — fast to compile, reasonable code quality.

Contrast with LLVM: LLVM produces better code quality but compilation is slow (seconds per
module). Cranelift is faster to compile (milliseconds per module) at the cost of some
optimization quality. For a JIT where startup time matters, Cranelift is the right choice.

End with: Cranelift is what `codegen.rs` talks to. Let's understand how.
-->

## What Cranelift provides

Cranelift is a **code generation library** — not a compiler, not a linker, not an assembler.
It provides:

1. **An IR** (CLIF — Cranelift Intermediate Form): a register-based IR in SSA form
2. **A builder API**: create functions, create blocks, emit instructions
3. **Code emission**: translate CLIF → native machine code for the current CPU
4. **A JIT module**: allocate executable memory, resolve function references

You construct CLIF using the builder API, then ask Cranelift to emit native code. Cranelift
handles register allocation, instruction selection, and all the platform-specific details.

## Cranelift's type system

Cranelift has a simple type system for IR values:

| Cranelift type | Size | Used for |
|---------------|------|---------|
| `types::I64` | 64-bit integer | Most values (Oxy uses this for all `Value` pointers) |
| `types::I32` | 32-bit integer | Some immediate values |
| `types::I8` | 8-bit integer | Boolean results from comparisons |
| `types::F64` | 64-bit float | Float arithmetic |

In Oxy's JIT, almost everything is `I64`. The `Value` enum (32 bytes per value in most
cases) is passed between JIT code and Rust FFI functions as a pointer (`i64` holding a
memory address). This is why the JIT uses `types::I64` almost everywhere — it is not
integers, it is pointers.

## The `FunctionBuilder` API

Cranelift's builder API is used in `codegen.rs`:

```rust
// From crates/oxy-core/src/vm/jit/codegen.rs
use cranelift_codegen::ir::{InstBuilder, types, AbiParam};
use cranelift_frontend::FunctionBuilder;

// Every IR block maps to a Cranelift block
for b in &ir_fn.blocks {
    cl_blocks.insert(b.id, builder.create_block());
}

// Emit instructions into the current block
builder.switch_to_block(cl_block);
builder.seal_block(cl_block);

// Emit a Cranelift add instruction
let result = builder.ins().iadd(left_val, right_val);
```

The key calls:
- `builder.create_block()` — allocate a new block (parallel to Oxy's `alloc_block()`)
- `builder.switch_to_block(b)` — start emitting into block `b`
- `builder.seal_block(b)` — signal that all predecessors are known (required for SSA)
- `builder.ins().iadd(a, b)` — emit an integer add instruction

## CLIF values vs Oxy registers

In Oxy's IR, registers are `Reg = usize`. In Cranelift, values are `cranelift_codegen::ir::Value`.

The codegen translates between them using two maps:

```rust
// In codegen.rs
let mut regs: HashMap<Reg, cranelift_codegen::ir::Value> = HashMap::new();
let mut reg_slot: HashMap<Reg, usize> = HashMap::new();
```

- `regs`: for registers whose values are simple (constants, arithmetic results) — held
  as Cranelift values in SSA form
- `reg_slot`: for registers whose values go through the JIT context buffer (complex types
  like structs, strings) — held as slot indices into the operand stack

This two-map design reflects the two-layer execution model: simple values stay in Cranelift
SSA values (fast); complex values go through the `JitContext` buffer (correct).

## Declaring FFI functions

Before compiled code can call Rust functions, those functions must be declared to Cranelift:

```rust
// codegen.rs
pub fn declare_ffi(&mut self, name: &str, params: Vec<types::Type>, ret: Option<types::Type>) {
    let mut sig = self.module.make_signature();
    for p in &params {
        sig.params.push(AbiParam::new(*p));
    }
    if let Some(r) = ret {
        sig.returns.push(AbiParam::new(r));
    }
    let fid = self.module.declare_function(name, Linkage::Import, &sig).unwrap();
    self.ffi_ids.insert(name.to_string(), fid);
}
```

Every `oxy_*` function in `ffi/mod.rs` is declared this way before compilation starts.
The ABI is always: `(ctx: *mut JitContext, ...typed args...) -> (optional return)`.
The `ctx` pointer is always the first parameter — it is how compiled code passes values
to and from Rust functions.

## Compiling a function

The high-level flow in `codegen.rs`:

```rust
pub fn compile(&mut self, functions: Vec<IrFunction>) -> Result<(), String> {
    // 1. Compile each function to CLIF
    for func in functions {
        let (fid, name) = self.compile_fn(&func)?;
        // ...
    }

    // 2. Finalize all definitions (this is where Cranelift emits machine code)
    self.module.finalize_definitions()?;

    // 3. Get the native function pointer for each compiled function
    for (fid, name, local_count, idx) in pending {
        let ptr = self.module.get_finalized_function(fid);
        self.fn_ptrs.insert(idx, ptr);
    }
}
```

`finalize_definitions()` is the expensive step — this is where Cranelift runs register
allocation and emits machine code for all compiled functions. After it returns, `fn_ptrs`
holds `*const u8` pointers to executable memory.

## Calling compiled code

The compiled function pointer signature is:

```rust
type CompiledFn = extern "C" fn(*mut JitContext) -> i64;
```

Every compiled Oxy function takes one argument — the `JitContext` pointer — and returns
an `i64` (a discriminant indicating success/error). Results and arguments are passed
through the `JitContext` buffer, not as normal function parameters.

This uniform signature is why all compiled functions can be stored in the same pointer table
and called through a single `invoke_jit_fn` helper.
