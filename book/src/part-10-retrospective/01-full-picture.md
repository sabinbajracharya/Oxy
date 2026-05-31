# The Full Picture: Every Layer Explained

We covered a lot of ground. Let's see what it looks like from up here.

Start with a single line of source — `let x = 2 + 3 * 4;` — and watch it fall through every layer
you now know by name. The **lexer** (Part 1) chops the text into labeled tokens, knowing nothing of
what they mean. The **parser** (Part 2) arranges those tokens into an AST, and in doing so quietly
solves precedence by making the multiplication a subtree of the addition. The **type checker**
(Part 4) walks that same tree asking not "what is the value" but "what is the type," and proves
nothing impossible is being attempted. Then **IR gen** (Part 6) flattens the tree into register
IR — named values, basic blocks, terminators — and from there the road forks. On native, the
**Cranelift JIT** (Part 7) lowers that IR to real x86 or arm64 instructions, maps them executable,
and the CPU runs them directly. In the browser, the **IR interpreter** (Part 8) walks the very same
IR instead, calling the very same `oxy_*` runtime functions the JIT calls. Two destinations, one
road for almost the entire trip.

What's striking from this height is how little magic there turns out to be. Every layer is just a
program that reads one representation and writes another: text to tokens, tokens to tree, tree to
IR, IR to machine code. Each one can be opened, read, and understood on its own — the type checker
doesn't need to know how Cranelift works, the lexer doesn't care that an interpreter exists. The
road was hard to walk in places; the 129 failures of Part 7 were not a pleasant afternoon. But from
the summit it's a clear path, and a short one: a handful of source files, each doing one honest
transformation. That's the whole thing. The rest of this chapter lays the trip out one more time,
layer by layer, so you can see the full descent in a single view.

## A single line, all the way down

Take this Oxy program:
```rust
fn main() {
    let x = 2 + 3 * 4;
    println(x);
}
```

Here is what happens, layer by layer:

---

### Layer 1: Lexer (Part 1)

The source text `fn main() { let x = 2 + 3 * 4; println(x); }` is passed to `tokenize()`.
The lexer reads character by character, grouping them into labeled pieces:

```
Fn, Ident("main"), LParen, RParen, LBrace,
Let, Ident("x"), Eq, IntLiteral(2), Plus, IntLiteral(3), Star, IntLiteral(4), Semicolon,
Ident("println"), LParen, Ident("x"), RParen, Semicolon,
RBrace, Eof
```

21 tokens. The lexer has no idea what they mean. It just labels them.

**Source:** `crates/oxy-core/src/lexer/mod.rs`

---

### Layer 2: Parser (Part 2)

The parser reads the token stream and builds an AST:

```
Program {
  items: [
    Function(FnDef {
      name: "main",
      body: Block {
        stmts: [
          Let { name: "x", value: BinaryOp {
            left: IntLiteral(2),
            op: Add,
            right: BinaryOp {
              left: IntLiteral(3),
              op: Mul,
              right: IntLiteral(4),
            }
          }},
          Expr { expr: Call {
            callee: Ident("println"),
            args: [Ident("x")]
          }}
        ]
      }
    })
  ]
}
```

The Pratt parser's precedence rules place `3 * 4` deeper in the tree than `2 + ...`,
correctly encoding that multiplication binds tighter than addition.

**Source:** `crates/oxy-core/src/parser/`

---

### Layer 3: Type checker (Part 4)

Two passes:
1. Collect: register `main` with return type `Unit`
2. Check: verify `2 + 3 * 4` produces `int` (it does), verify `println` accepts any value (it does)

No type errors. The program is type-safe.

**Source:** `crates/oxy-core/src/type_checker/`

---

### Layer 4: IR gen (Part 6)

The type-checked AST is compiled to register IR:

```
fn main [params: ] -> ():
  block 0:
    v0 = ConstInt(2)
    v1 = ConstInt(3)
    v2 = ConstInt(4)
    v3 = Mul(v1, v2)     ; 3 * 4 = 12
    v4 = Add(v0, v3)     ; 2 + 12 = 14
    StoreLocal(0, v4)    ; x = 14
    v5 = LoadLocal(0)    ; load x
    v6 = CallBuiltin { func: "oxy_println_val", args: [v5] }
    v7 = ConstUnit
    Ret(v7)
```

The AST's tree structure is gone. The precedence is encoded in the order of operations.
A flat sequence of register operations.

**Source:** `crates/oxy-core/src/vm/jit/ir_gen/`

---

### Layer 5a: Cranelift JIT (Part 7, native)

On native (`x86-64`, `aarch64`): the IR is translated to Cranelift CLIF and compiled:

```asm
; roughly what Cranelift emits for the arithmetic
mov  rax, 2          ; v0 = 2
mov  rbx, 3          ; v1 = 3
mov  rcx, 4          ; v2 = 4
imul rbx, rcx        ; v3 = 3 * 4 = 12
add  rax, rbx        ; v4 = 2 + 12 = 14
; ... store x, call oxy_println_val
```

The `call` instruction for `oxy_println_val` jumps to the Rust FFI function, which does
the actual printing. The CPU executes native instructions.

**Source:** `crates/oxy-core/src/vm/jit/codegen.rs` + `ffi/`

---

### Layer 5b: IR interpreter (Part 8, wasm32)

On `wasm32` (browser): the same IR is walked by the interpreter. `ConstInt(v0, 2)` inserts
`Value::I64(2)` into the register map. `Mul(v3, v1, v2)` calls `oxy_mul` via the FFI table.
`CallBuiltin { "oxy_println_val" }` calls the same Rust function.

Different executor. Same IR. Same FFI. Same result: `14` printed.

**Source:** `crates/oxy-core/src/vm/interp.rs`

---

## The full stack in one table

| Layer | Input | Output | Key file |
|-------|-------|--------|---------|
| Lexer | Source text | `Vec<Token>` | `lexer/mod.rs` |
| Parser | `Vec<Token>` | `Program` (AST) | `parser/mod.rs` |
| Type checker | `Program` | validation | `type_checker/mod.rs` |
| IR gen | `Program` | `Vec<IrFunction>` | `jit/ir_gen/mod.rs` |
| JIT codegen | `Vec<IrFunction>` | native code | `jit/codegen.rs` |
| JIT runtime | Native calls | `Value` | `jit/ffi/mod.rs` |
| IR interpreter | `Vec<IrFunction>` | `Value` | `vm/interp.rs` |

Each row's output is the next row's input. Each transformation is a program that reads one
representation and produces another. A compiler is a pipeline of transformations.

That's the whole thing.
