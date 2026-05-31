# IrOp and Terminator: Every Instruction Explained

<!-- OPUS_FILL
1-paragraph intro. "Let's read the instruction set."
Reference the file. Keep it very short — this chapter is reference material.
-->

**File:** `crates/oxy-core/src/vm/jit/ir.rs`

Open it now. This chapter explains every `IrOp` variant.

---

## Constants

| Op | Meaning |
|----|---------|
| `ConstInt(r, n)` | `r = n` (integer) |
| `ConstFloat(r, f)` | `r = f` (float) |
| `ConstBool(r, b)` | `r = b` |
| `ConstChar(r, c)` | `r = c` |
| `ConstUnit(r)` | `r = ()` |
| `ConstString(r, s)` | `r = s` (string — allocated via FFI) |

Constants are always the first thing emitted for a literal value. The result register
holds the value for use by subsequent ops.

---

## Locals

| Op | Meaning |
|----|---------|
| `LoadLocal(r, slot)` | `r = locals[slot]` (with Cell unwrapping) |
| `LoadLocalRaw(r, slot)` | `r = locals[slot]` (without Cell unwrapping) |
| `StoreLocal(slot, r)` | `locals[slot] = r` |
| `MakeCell(slot)` | Convert `locals[slot]` to a `Value::Cell` (for captured mutable variables) |

`locals` are variable storage slots. `LoadLocal` vs `LoadLocalRaw`: when a mutable variable
is captured by a closure, it becomes a `Value::Cell` (shared mutable box). `LoadLocal`
transparently unwraps the cell; `LoadLocalRaw` does not (used when the cell itself is
being passed to a closure, not read).

`MakeCell` is emitted when `ir_gen` detects that a `let mut` binding is captured by a closure.

---

## Binary arithmetic (inlined in CLIF)

All take `(result, left, right)` — three registers.

| Op | Meaning |
|----|---------|
| `Add(r, a, b)` | `r = a + b` |
| `Sub(r, a, b)` | `r = a - b` |
| `Mul(r, a, b)` | `r = a * b` |
| `Div(r, a, b)` | `r = a / b` |
| `Rem(r, a, b)` | `r = a % b` |
| `Eq(r, a, b)` | `r = (a == b)` |
| `Neq(r, a, b)` | `r = (a != b)` |
| `Lt(r, a, b)` | `r = (a < b)` |
| `Gt(r, a, b)` | `r = (a > b)` |
| `Le(r, a, b)` | `r = (a <= b)` |
| `Ge(r, a, b)` | `r = (a >= b)` |
| `And(r, a, b)` | `r = (a && b)` |
| `Or(r, a, b)` | `r = (a \|\| b)` |
| `BitAnd(r, a, b)` | `r = a & b` |
| `BitOr(r, a, b)` | `r = a \| b` |
| `BitXor(r, a, b)` | `r = a ^ b` |
| `Shl(r, a, b)` | `r = a << b` |
| `Shr(r, a, b)` | `r = a >> b` |

"Inlined in CLIF" means the JIT emits a single Cranelift instruction (e.g., `ins.iadd(a, b)`).
The wasm interpreter implements them directly in Rust (e.g., integer addition in a match arm).

---

## Unary operations

| Op | Meaning |
|----|---------|
| `Neg(r, a)` | `r = -a` |
| `Not(r, a)` | `r = !a` |
| `BitNot(r, a)` | `r = ~a` |

---

## `CallBuiltin` — the FFI operation

```rust
CallBuiltin {
    result: Reg,
    func: &'static str,      // e.g. "oxy_push_int", "oxy_struct_init"
    args: Vec<Reg>,          // register values to pass
    immediates: Vec<usize>,  // numeric metadata
    strings: Vec<String>,    // string metadata
}
```

This is the most common non-trivial op. Every collection method, struct operation, function call,
closure creation, and runtime dispatch goes through `CallBuiltin`.

The `func` name is always an `oxy_*` function from `jit/ffi.rs`. The `args` are the register
values to push before calling. The `immediates` and `strings` carry compile-time metadata
the runtime function needs.

---

## Special ops

| Op | Meaning |
|----|---------|
| `Copy(r, src)` | `r = src` (when a value is needed in multiple places) |
| `CheckError(r)` | Check if ctx has an error set; result is 0 or 1 |
| `Phi(r, a, b)` | Select `a` or `b` based on which predecessor block was taken |

`Phi` is a classic SSA construct. When two branches join (if/else join point), and both
branches define the same variable, Phi selects the right value. Oxy uses Phi sparingly —
most joins use locals (store in both branches, load after).

---

## Terminators

| Terminator | Meaning |
|-----------|---------|
| `Return(r)` | Return the value in register `r` from this function |
| `Jump(b)` | Jump unconditionally to block `b` |
| `Branch { cond, then_block, else_block }` | If `cond` is truthy, jump to `then_block`, else `else_block` |
| `Halt` | End program execution |

Every basic block ends with exactly one terminator. The terminator is not an `IrOp` —
it is separate, making it impossible to emit a terminator mid-block.

---

## The exhaustive match constraint

The IR interpreter (`vm/interp.rs`) has a `match op { ... }` over `IrOp` with **no wildcard arm**.
This means every `IrOp` variant must be handled. Adding a new variant breaks the build until
the interpreter handles it.

This is "guard #1" from the CLAUDE.md: compile-time enforcement of backend parity. If you
add `IrOp::FooBar` to `ir.rs`, the project will not compile until you add a `FooBar` arm to
both `interp.rs` and any other exhaustive match over `IrOp`. The compiler tells you exactly
where to look.
