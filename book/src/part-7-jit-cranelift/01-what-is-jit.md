# What Is JIT Compilation?

JIT stands for Just-In-Time, and the phrase is more literal than it sounds: the program is compiled
to native machine code at *runtime*, at the last possible moment before it executes. That's a third
option distinct from the two you already know. A C compiler does its work *ahead* of time, long
before the user runs anything. The tree-walker did its work *during* execution, re-examining the
same code on every single pass. A JIT splits the difference — it waits until you actually run the
program, then compiles it to real machine instructions, then jumps into those instructions. Just in
time: not too early, not never.

This is not an exotic technique; it's the quiet engine under most of modern computing. Java's
HotSpot JIT is the reason "Java is slow" stopped being true sometime around 2005. V8 is the reason
the JavaScript in your browser runs within shouting distance of C. LuaJIT, PyPy, the .NET CLR —
all JITs. When something dynamic turns out to be surprisingly fast, there is very often a JIT
underneath doing exactly what this part of the book describes.

And the reason native code is so much faster comes down to what *isn't* there. No `match` on an
opcode to decide what to do next. No boxing an integer into a heap object just to add it. No
recursive `eval` call per node. The CPU just executes instructions, directly, the way the hardware
was built to. Oxy gets there using **Cranelift** — a code generator written in Rust, designed for
exactly this job — which takes the register IR we built in Part 6 and emits native x86 or arm64
machine code. How that translation works, and what it costs, is what this part is about.

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
