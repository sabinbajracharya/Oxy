# What Is JIT Compilation?

<!-- OPUS_FILL
Write a 3-paragraph hook.

JIT = Just-In-Time. The program is compiled to native machine code at runtime, right
before it executes — not before distribution (like C), not by walking the code every
time (like the tree-walker). The "just in time" means: "compiled at the last possible
moment before execution."

Reference real JITs: HotSpot (Java), V8 (JavaScript), PyPy (Python). They are the reason
your browser runs JavaScript at near-C speed, and why Java stopped being slow.

The key insight: native code has no dispatch overhead. No "match on opcode". No boxing
of integers. The CPU just executes instructions directly.

End with: Oxy uses Cranelift — a Rust-native code generator — to do this compilation.
Cranelift takes our register IR and emits native x86/arm64 machine code. That's this part.
-->

## AOT vs JIT vs interpretation

Three ways to run a program:

**AOT (Ahead-Of-Time):** compile everything to machine code before distribution. The user
runs a native binary. Examples: C, C++, Rust. Maximum performance. Compilation is slow
(done by the developer). Language must be statically typed.

**Interpretation:** execute the source (or bytecode) at runtime by walking it. Examples:
tree-walking interpreters, CPython, Ruby MRI. No startup cost. Easy to implement. Slow for
compute-intensive code.

**JIT (Just-In-Time):** compile to native code at runtime, immediately before execution.
Examples: Java HotSpot, V8, LuaJIT. Near-native performance after the first run. Higher
startup cost than pure interpretation. Complex to implement.

Oxy uses JIT: when you run an `.ox` file, Oxy compiles the IR to native code in memory,
maps that memory as executable, and jumps into it. The compilation takes a few milliseconds.
The execution is at native speed.

## What "compile to machine code in memory" means

The JIT process:
1. Generate the IR (AST → register IR, done by `ir_gen`)
2. Translate IR to Cranelift's representation (the `Codegen` struct does this)
3. Ask Cranelift to emit machine code for the current CPU architecture
4. Receive a blob of bytes — actual x86 or arm64 instructions
5. Map those bytes as executable memory (`mmap` with `PROT_EXEC` on Unix)
6. Cast the memory address to a function pointer
7. Call the function pointer

Steps 3-7 are what Cranelift handles. Oxy's job ends at step 2.

## Why native code is fast

A JIT-compiled integer addition `a + b` becomes:
```asm
mov rax, [rbp-8]    ; load a from its memory slot
mov rbx, [rbp-16]   ; load b from its memory slot
add rax, rbx        ; add them
mov [rbp-24], rax   ; store result
```

Four CPU instructions. No Rust function calls. No match statements. No heap allocation.
The CPU executes these in 4 clock cycles — nanoseconds.

The interpreted equivalent (with boxed `Value::Int` objects) takes ~100× longer.

## What Oxy's JIT compiles

The full Oxy program runs in two layers:

**Compiled layer** — Cranelift-emitted native code handles:
- All arithmetic operations (add, subtract, multiply, etc.)
- All comparisons
- All control flow (if/else → conditional jumps, while → loops)
- All function calls (compiled Oxy functions call each other via function pointers)

**Runtime layer** — Rust functions called from compiled code handle:
- All collection operations (push, pop, get, insert, etc.)
- All string operations
- Struct and enum creation/access
- Closures and higher-order functions
- I/O, HTTP, JSON, the stdlib

The boundary between the two layers is the **FFI bridge** — the `oxy_*` functions in
`vm/jit/ffi/`. Compiled code calls these Rust functions through the JIT's call mechanism.

This design means: simple code runs at full native speed; complex operations (collections,
strings) run at "fast Rust function" speed. In practice, both are fast enough.
