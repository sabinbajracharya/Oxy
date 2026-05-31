# Reading Oxy IR with OXY_VM_TRACE=1

The IR is not a secret buried inside the compiler — you can just look at it. Set the environment
variable `OXY_VM_TRACE=1` and Oxy dumps the register IR for every function it compiles straight to
stderr, exactly the form we've been drawing by hand for the last few chapters. This is the single
most useful debugging tool in the whole project: when a program does the wrong thing, the IR trace
shows you precisely what was generated, layer by layer, so you can pin the bug to the stage that
produced it. Let's turn it on and learn to read what comes out.

## Enabling IR dumps

```bash
OXY_VM_TRACE=1 docker compose run --rm dev bash -c \
  "cargo run --bin oxy -- run examples/hello.ox" 2>&1 | head -50
```

The output goes to stderr. The `2>&1` redirects it to stdout so you can pipe it.

## A simple example

For this Oxy program:
```rust
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let result = add(3, 4);
    println(result);
}
```

The IR dump looks like:
```
fn add [params: a@0, b@1] -> int:
  block 0:
    v2 = LoadLocal(0)       ; load a
    v3 = LoadLocal(1)       ; load b
    v4 = Add(v2, v3)        ; a + b
    Ret(v4)

fn main [params: ] -> ():
  block 0:
    v0 = ConstInt(3)
    v1 = ConstInt(4)
    v2 = CallBuiltin { func: "oxy_call", args: [v0, v1], strings: ["add"] }
    v3 = StoreLocal(0, v2)  ; result = ...
    v4 = LoadLocal(0)       ; load result
    v5 = CallBuiltin { func: "oxy_println_val", args: [v4] }
    v6 = ConstUnit
    Ret(v6)
```

## How to read the trace

**Register naming:** `v0`, `v1`, etc. are virtual registers. Each is defined exactly once
(by the op on its left) and used zero or more times.

**`LoadLocal(slot)` / `StoreLocal(slot, reg)`:** locals are indexed slots. In `add`,
parameter `a` is at slot 0, `b` at slot 1. `LoadLocal(0)` loads `a`.

**`CallBuiltin { func: "oxy_call", strings: ["add"] }`:** calling a user-defined function.
The function name is in `strings[0]`. The args are in the listed registers. This is how
the JIT and interpreter both know which function to call.

**`Ret(v4)`:** return the value in register `v4`.

## Tracing an if/else

For:
```rust
fn abs(x: int) -> int {
    if x < 0 { -x } else { x }
}
```

IR:
```
fn abs [params: x@0] -> int:
  block 0:
    v1 = LoadLocal(0)        ; x
    v2 = ConstInt(0)
    v3 = Lt(v1, v2)          ; x < 0
    Branch(v3, then=1, else=2)

  block 1:  ; then branch
    v4 = LoadLocal(0)        ; x
    v5 = Neg(v4)             ; -x
    StoreLocal(1, v5)        ; store to join slot
    Jump(3)

  block 2:  ; else branch
    v6 = LoadLocal(0)        ; x
    StoreLocal(1, v6)        ; store to join slot
    Jump(3)

  block 3:  ; join
    v7 = LoadLocal(1)        ; load join result
    Ret(v7)
```

Notice the join slot (local slot 1, a synthetic `__if_result` slot). Both branches
store their result there. Block 3 loads it and returns.

## Tracing a while loop

For:
```rust
fn main() {
    let mut i = 0;
    while i < 5 {
        i = i + 1;
    }
}
```

IR:
```
fn main:
  block 0:  ; entry
    v0 = ConstInt(0)
    StoreLocal(0, v0)   ; i = 0
    Jump(1)

  block 1:  ; condition
    v1 = LoadLocal(0)   ; i
    v2 = ConstInt(5)
    v3 = Lt(v1, v2)     ; i < 5
    Branch(v3, then=2, else=3)

  block 2:  ; body
    v4 = LoadLocal(0)   ; i
    v5 = ConstInt(1)
    v6 = Add(v4, v5)    ; i + 1
    StoreLocal(0, v6)   ; i = i + 1
    Jump(1)             ; ← back-edge!

  block 3:  ; after loop
    v7 = ConstUnit
    Ret(v7)
```

The `Jump(1)` in block 2 creates the loop back-edge. The condition is re-evaluated on each
iteration. When it becomes false, block 1's `Branch` goes to block 3.

## Using IR traces to debug

When a test fails unexpectedly, the IR trace shows what code was generated. Common patterns:

**Wrong value in a register:** the op that produced it is emitting the wrong arguments.
Trace back which `gen_expr` call produced that op.

**Missing operation:** an expression that should produce an effect (a method call, an
assignment) produces no IR. Look for where `gen_expr` for that expression type is in
`ir_gen/` — it may return early or miss a branch.

**Incorrect branch target:** a `Branch` goes to the wrong block. The `gen_if` or `gen_while`
function has a block allocation or `terminate` call in the wrong order.

The IR dump is your primary debugging tool for IR gen issues.
