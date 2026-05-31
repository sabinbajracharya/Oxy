# Why Register Machines Beat Stacks

The last part ended with a decision: skip the stack, emit register IR. This chapter is the
justification. And the deepest reason is physical — real CPUs are register machines, not stack
machines. An x86-64 chip has named registers: RAX, RBX, RCX, RDX, and a couple dozen more. ARM64
has X0 through X30. When the processor adds two numbers, it loads them into named registers and
writes the result to a named register. There is no hardware stack of operands; the stack is a
fiction layered on top. So if your end goal is machine code, register-style thinking isn't one
option among several — it's the native tongue of the target. Stack bytecode was always going to
require translating *back* into registers eventually. It was a detour.

There's a second reason, just as important and more about your own sanity than the hardware's. In a
stack machine, values are anonymous — a thing is "second from the top" right now and something else
in three instructions. To know what you're operating on, you have to mentally replay every push and
pop. In a register machine, every value has a *name* that never moves: `v0` is `v0` forever, and
you can trace exactly where it came from and where it's used by following that name. Named values
are vastly easier to analyze, fold, eliminate, and reason about — which is exactly why LLVM,
Cranelift, and essentially every serious compiler backend speaks register IR, and why Oxy switched
to it the moment native code became the goal.

## Real CPUs use registers

When a CPU adds two numbers, it does:
```asm
; x86-64
mov  rax, 2     ; load 2 into register rax
mov  rbx, 3     ; load 3 into register rbx  
add  rax, rbx   ; rax = rax + rbx = 5
```

Values live in named registers. Operations read named registers and write to named registers.
There is no "push 2, push 3, add" — the stack is an abstraction that lives on top of the
register machine.

To emit native code from a stack-based VM, you would need to map stack slots back to registers:
"the value on top of the stack after this operation goes into rax." This mapping (called
"register allocation") is exactly what compilers spend a lot of effort on.

The alternative: use register IR from the start. Then register allocation is the compiler
backend's problem — which is what Cranelift solves. Your job is just to emit register IR.

## Register IR is easier to reason about

In a stack machine, values are positional — "second from the top." In register IR,
every value has a name:

```
# Stack bytecode for `let z = x + y`
LoadLocal(0)   # push x
LoadLocal(1)   # push y
Add            # pop two, push sum
StoreLocal(2)  # pop, store to z

# Register IR for the same
v2 = Add(v0, v1)    # v0=x, v1=y, result in v2
StoreLocal(2, v2)   # z = v2
```

In the register form, every value is named (`v0`, `v1`, `v2`). You can trace where any
value comes from by following its register. In the stack form, the "third value on the
stack at this point" requires mentally simulating the push/pop sequence.

This matters for optimization: constant folding, dead code elimination, and value reuse
are all easier when every value has a stable name.

## The "infinite registers" model

Real CPUs have ~16-32 registers. Oxy's IR uses **infinite virtual registers** — just
incrementing integers. Each operation gets a fresh register number for its result:

```
v0 = ConstInt(2)
v1 = ConstInt(3)
v2 = ConstInt(4)
v3 = Mul(v1, v2)    # 3 * 4 = 12
v4 = Add(v0, v3)    # 2 + 12 = 14
```

The Cranelift backend maps these virtual registers to real CPU registers (or spills to
the stack when there are more live values than CPU registers). This mapping is called
**register allocation** — it is Cranelift's job, not Oxy's.

Oxy's IR gen never thinks about running out of registers. It just allocates a new `Reg`
(a `usize` counter) for each new value.

## Why this maps naturally to SSA

Infinite registers where each register is defined exactly once is called **Static Single
Assignment** (SSA) form. SSA is the standard IR form used by LLVM, GCC, and modern
compiler backends because it makes dataflow analysis simple.

Oxy's IR is "SSA-like" — registers are defined once by one operation and then read-only.
Mutation (assignment to a variable) is represented by `StoreLocal(slot, reg)` followed
by `LoadLocal(reg, slot)` — the slot is mutated, but each loaded value gets a fresh register.

This is why Cranelift (which uses SSA internally) can work directly with Oxy's IR:
they speak the same language.
