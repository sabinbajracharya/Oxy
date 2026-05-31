# What Is WebAssembly and Why Does It Matter?

<!-- OPUS_FILL
Write a 3-paragraph hook.
WebAssembly (wasm) is a binary instruction format that runs in browsers with near-native
speed. It is the technology that lets complex applications (Figma, Google Earth, video
editors) run at full performance in a browser tab.

The key property: wasm is a sandboxed environment. It cannot access the file system,
network, or OS directly — only through APIs the browser provides. This is why Cranelift
cannot run inside wasm: Cranelift needs to mmap executable memory, and wasm's sandbox
prohibits that.

For Oxy: the goal was a browser-based playground where users could write and run Oxy code
without installing anything. Wasm makes this possible. The constraint: can't use the JIT.
Need a different backend.

End with the pivot: "The solution was elegant: don't compile to machine code. Walk the IR."
-->

## WebAssembly in one paragraph

WebAssembly is a binary format for programs that browsers can execute efficiently. Unlike
JavaScript (a text format that browsers interpret), wasm is a compact binary that browsers
can parse and compile to native machine code very quickly. It is designed to be a
compilation target — C, C++, Rust, and many other languages compile to wasm.

Crucially: wasm runs in a **sandbox**. It cannot access files, network, or OS directly.
It can only do what the host (the browser) explicitly allows through an API.

## Why Cranelift cannot run in the browser

Cranelift emits native machine code and maps it as executable memory using `mmap(PROT_EXEC)`.
This system call — "allocate a chunk of memory and mark it executable" — is what makes JIT
compilation work on native platforms.

In a wasm sandbox, `mmap` is not available. The wasm runtime (the browser) controls all
memory allocation. There is no way to allocate executable memory from inside a wasm program.

Additionally, Cranelift emits code for the *host* CPU architecture (x86 or arm64). When
running inside wasm, the host is the wasm virtual machine, which has its own instruction set.
Cranelift does not have a wasm-emitting backend.

Result: the JIT cannot run in the browser. Oxy needed a second execution engine.

## Why a browser playground matters

Many modern programming languages ship with browser playgrounds:
- Rust: play.rust-lang.org
- Go: play.golang.org
- Kotlin: play.kotlinlang.org

These lower the barrier to experimentation. You can try Oxy syntax, run examples, and
share programs without installing anything. For a new language, this is essential for adoption.

## The solution: walk the IR directly

The IR (`Vec<IrFunction>` from `ir_gen`) is just data — a list of basic blocks with
instructions. It can be **walked** just as well as compiled.

Instead of translating `IrOp::Add(v2, v0, v1)` into Cranelift instructions that emit
`add rax, rbx`, the interpreter does:

```rust
IrOp::Add(r, a, b) => {
    self.binary(ctx, regs, "oxy_add", *r, *a, *b);
}
```

Where `binary` calls the same `oxy_add` FFI function that the JIT calls. Same function,
different caller. The IR is the same. The FFI is the same. Only the mechanism changes.

This is the core insight of Part 8: **one IR, two backends, one runtime**.
