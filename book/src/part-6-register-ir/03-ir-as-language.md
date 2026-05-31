# The IR as a Language Between Languages

<!-- OPUS_FILL
Write a 2-paragraph hook. The IR is a private language understood only by the compiler
and the two backends. It is designed by the compiler author for the compiler author —
not for humans to write, not for the machine to execute directly.

Reference: LLVM IR, JVM bytecode, WebAssembly — all IRs. They are all designed to be:
easy to generate (from the front-end), easy to analyze (for optimization), easy to lower
(to machine code or interpretation).

The Oxy IR has a specific design constraint: it must be interpretable by the wasm interpreter
as well as compilable by Cranelift. This rules out low-level choices (raw memory addresses,
unboxed values) that the interpreter couldn't handle.
-->

## Two consumers, one IR

Oxy's register IR is consumed by two backends:

| Backend | How it uses the IR |
|---------|-------------------|
| Cranelift JIT | Translates `IrOp` → CLIF instructions; terminators → CLIF block transitions |
| WASM interpreter | Walks `IrOp`s directly; calls same `oxy_*` FFI for `CallBuiltin` ops |

The IR must be expressive enough that both backends can implement all of Oxy's semantics.
This shapes many design decisions.

## Why `CallBuiltin` instead of native ops for everything

Simple arithmetic (`Add`, `Sub`, `Mul`, etc.) is emitted as native IR ops because:
- The JIT can emit efficient CLIF integer/float instructions
- The interpreter can implement them with simple Rust arithmetic

Complex operations — struct initialization, method calls, collection mutations — are emitted
as `CallBuiltin { func: "oxy_struct_init", args: [...], ... }` because:
- The same Rust function handles both backends (FFI dispatch table)
- The logic is complex enough that duplicating it in both backends would cause drift
- New operations can be added without changing IR or either backend's dispatch loop

The split: "does it compile to a single CPU instruction?" → native IR op. "Does it require
Rust logic?" → `CallBuiltin`.

## The `CallBuiltin` anatomy

```rust
IrOp::CallBuiltin {
    result: Reg,        // register where the return value goes
    func: &'static str, // name of the oxy_* function
    args: Vec<Reg>,     // registers to pass as arguments
    immediates: Vec<usize>, // numeric metadata (field_count, etc.)
    strings: Vec<String>,   // string metadata (field names, function names)
}
```

Example — `println(x)`:
```
CallBuiltin {
    result: v99,          // println returns Unit; we still need a result reg
    func: "oxy_println_val",
    args: [v0],           // v0 holds the value to print
    immediates: [],
    strings: [],
}
```

Example — `Point { x: 1.0, y: 2.0 }`:
```
v0 = ConstFloat(1.0)
v1 = ConstFloat(2.0)
CallBuiltin {
    result: v2,
    func: "oxy_struct_init",
    args: [v0, v1],
    immediates: [2],                   // field_count = 2
    strings: ["Point", "x", "y"],     // struct name + field names
}
```

The strings and immediates carry the metadata that the FFI function needs but that doesn't
fit into a register value.

## The IR is pretty-printable

Oxy can dump the register IR for any program:

```bash
OXY_VM_TRACE=1 cargo run -- run examples/hello.ox 2> ir.txt
```

The output looks like:
```
fn main:
  block 0:
    v0 = ConstString("Hello, world!")
    CallBuiltin { result: v1, func: "oxy_println_val", args: [v0] }
    v2 = ConstUnit
    Ret(v2)
```

This is the `ir_snapshot.rs` pretty-printer — the same output used by IR snapshot tests.
When a test changes behavior, the snapshot test shows exactly which IR operations changed.

## What the IR does not include

The IR deliberately omits:
- **Source spans**: the IR has no line/column information. Errors at the IR level are
  internal compiler errors, not user-facing. User errors are caught by the type checker.
- **Type annotations on registers**: registers are untyped in the IR. The `TypeInfo` on
  `IrFunction` covers the function signature; individual ops rely on the FFI to handle
  runtime type dispatch.
- **Debug information**: no DWARF, no symbol tables. Oxy programs are not debuggable at
  the native level (yet).

These omissions keep the IR simple and both backends fast. They are the right trade-offs
for Oxy's current goals.
