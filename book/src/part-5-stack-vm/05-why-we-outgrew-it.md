# Why We Outgrew It

<!-- OPUS_FILL
Write a 2-3 paragraph narrative. This is the pivot point of the book — where the
project commits to native compilation.

The emotional arc: the stack VM worked. It was a real upgrade. And then someone said
"what would it take to emit native code?" and the answer was: not a stack VM. Cranelift
wants register IR.

Reference the actual decision: rather than translate stack → register (an extra pass),
just emit register IR directly from the AST. The stack VM becomes an unnecessary middleman.

End with: "We made the call. The stack VM was removed in the same commit that added the
register IR. No overlap, no compatibility period. Just: here is the better thing."
-->

## The Cranelift impedance mismatch

Cranelift is a native code generator. It takes a **register-based intermediate
representation** (CLIF — Cranelift Intermediate Form) and emits x86, arm64, or other
native machine code.

CLIF is register-based. Stack-based code is not what it wants.

Translating from stack bytecode to Cranelift's register IR would require a conversion pass:
- Analyze the stack bytecode to figure out which stack slots correspond to which values
- Map those to named registers
- Emit CLIF register operations

This is exactly what the Python-to-LLVM and Ruby-to-LLVM projects had to build. It is a
significant amount of work and introduces a translation layer with its own bugs.

The alternative: skip the stack entirely. Compile the AST **directly to register IR**.
One pass, no translation layer, no stack VM needed.

## What register IR enables

With register IR:
- Each variable/value gets a named register (`v0`, `v1`, etc.)
- Each operation reads named registers and writes to a named register
- There is no implicit stack ordering to reason about

For `sum = sum + i`:
```
# Stack VM:
LoadLocal(0)    # stack: [sum]
LoadLocal(1)    # stack: [sum, i]
Add             # stack: [sum+i]
StoreLocal(0)   # locals[0] = sum+i; stack: []

# Register IR:
v2 = oxy_add(v0, v1)  # v0=sum, v1=i, v2=result
v0 = v2               # store back to sum's register
```

The register IR maps directly to what Cranelift wants: "take these two named values,
compute, store in this named result." No implicit ordering. No stack protocol to maintain.

## The decision and the commit

The decision was made in late May 2026:
- `2026-05-27` — `feat: add bytecode-to-Cranelift translator` (attempt to bridge stack → Cranelift)
- `2026-05-27` — `refactor: replace bytecode compiler with AST→Register IR generator` (new direction)
- `2026-05-28` — `refactor: remove bytecode compiler and VM` (stack VM gone)

The bytecode-to-Cranelift translator lasted one day. After seeing how complex bridging
was, the decision was: remove the stack VM and compile directly to register IR. The
stack VM's removal commit deleted ~2,700 lines. The register IR replacement added ~4,000.

The note in the commit message:
> *"Delete the retired bytecode compiler stub, OpCode enum, Chunk struct, Vm interpreter...
> Replace the compile_error test path with direct JitEngine::compile. The JIT pipeline
> (AST → IR → CLIF) is now the sole execution path."*

## What was kept

The stack VM's removal was clean because of the architectural separation established by
the bytecode era:

- The **stdlib** (HTTP, JSON, file I/O, math) was kept entirely — it was already
  independent of execution model
- The **`Value` type** was kept — both the stack VM and the register IR use `Value`
  as the runtime data representation
- The **`Environment`** type was kept — the wasm interpreter uses it
- The **test suite** (.ox feature tests) was kept — the tests are backend-agnostic

The stack VM that ran for 15 days contributed its architecture: the separation of compiler
(AST traversal) from executor (flat loop), and the stdlib independence. These ideas live
in the current codebase.
