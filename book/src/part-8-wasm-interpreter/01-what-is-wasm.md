# What Is WebAssembly and Why Does It Matter?

WebAssembly — wasm for short — is a binary instruction format that runs inside web browsers at
something close to native speed. It's the reason a tab can hold Figma, or Google Earth, or a video
editor, and have them feel like real applications instead of toys. Under the hood, the browser
takes the compact wasm binary, compiles it to actual machine code for your CPU, and runs it fast.
A lot of languages — C, C++, Rust, and more — can compile to wasm as a target, which is exactly
why it became the universal "run anything in the browser" layer.

But wasm buys that safety with a wall: it runs in a **sandbox**. A wasm program cannot touch the
filesystem, the network, or the operating system directly. It can only do what the host browser
explicitly hands it through an API. And that wall is precisely the thing that locks our JIT out.
Cranelift's whole trick, from Part 7, is to allocate a chunk of memory, mark it executable with
`mmap(PROT_EXEC)`, and jump into it — and "allocate your own executable memory" is exactly what a
sandbox exists to forbid. On top of that, Cranelift emits code for the *host* CPU, but inside the
browser the host is the wasm VM, which Cranelift has no backend for. The JIT simply cannot run
there.

This matters because the goal was a browser playground — somewhere a curious person could type Oxy
and hit Run without installing a thing, the way Rust, Go, and Kotlin all offer. Wasm is what makes
that possible, and the JIT is what wasm won't allow. So Oxy needed a second way to execute. And the
solution turned out to be almost suspiciously elegant: don't compile to machine code at all. Walk
the IR.

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
