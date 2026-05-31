# Why We Outgrew It

This is the pivot of the whole book — the moment the project stops climbing the ladder of
interpreters and commits to native compilation. And the thing to understand is that nothing was
wrong with the stack VM. It worked. It was a real, measurable upgrade over the tree-walker, ten to
fifty times faster on the loops that mattered. If the goal had been "a faster interpreter," we'd
have been done. The goal was never that. The goal was native machine code, as fast as C, and at
some point the question on the table became: *what would it take to emit native code from here?*
The answer was not "improve the stack VM." The answer was that the stack VM was now standing in the
way.

The reason is a mismatch. Cranelift — the code generator we'd be handing things to — does not want
a stack. It wants a *register* IR: named values, explicit operations on those values, no implicit
push-and-pop discipline to reverse-engineer. So a stack VM in front of Cranelift means writing a
translation pass that reads stack bytecode, figures out which stack slots are really which values,
and rebuilds them as registers. That pass is real work, it's the kind of thing the Ruby-on-LLVM and
Python-on-LLVM projects had to grind through, and it's a whole new surface for bugs. We actually
started building it — the `bytecode-to-Cranelift translator` commit — and it lasted exactly one day
before the better idea won: don't translate the stack into registers, just *emit registers
directly from the AST in the first place.* The stack VM wasn't a foundation to build on; it was a
middleman to remove.

So we made the call, and we made it cleanly. The stack VM was deleted in the same span of commits
that introduced the register IR — no overlap, no dual-backend compatibility period, no slow
deprecation. The removal commit dropped about 2,700 lines; the register IR added about 4,000. One
day there was a stack VM and the next there wasn't, and in its place was the thing the rest of this
book is about. Here is the better thing.

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
