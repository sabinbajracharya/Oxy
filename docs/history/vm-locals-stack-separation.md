# VM Locals/Operand Stack Separation

> **RETIRED — historical.** Describes the removed stack-based **bytecode VM**
> (single `self.stack: Vec<Value>`, `frame.base + slot`, `OpCode`), which no longer
> exists. Oxy now lowers to a register IR run by the Cranelift JIT / IR interpreter.
> Kept for provenance. Current architecture: [`../execution-model.md`](../execution-model.md).

Architecture decision record. Refactor landed 2026-05-21.

## Background — the original architecture

Before this refactor, the VM in `crates/oxy-core/src/vm/mod.rs` used a single
`self.stack: Vec<Value>` for two distinct purposes:

1. **Locals**, addressed by `frame.base + slot` (random access).
2. **Operand scratch**, push/pop for expression evaluation.

The `Frame` struct held only `base: usize` (where this frame's locals start
in `self.stack`) and `max_slot: usize` (highest slot index touched + 1).
The boundary between "locals" and "operand scratch" was implicit and shifted
at runtime as more slots were touched.

```text
self.stack: [.................. caller scratch ..................][L0][L1][L2][.....callee scratch.....]
                                                                  ^                                    ^
                                                            frame.base                       self.stack.len()
                                                                  |<--- frame.max_slot --->|
                                                                                            ^
                                                                                     frame_protected()
```

`Pop` consulted `frame_protected()` (= `base + max_slot`) before popping
so that an over-eager pop couldn't clobber a local. Locals were addressed
via `self.stack[base + slot]` everywhere — `LoadLocal`, `StoreLocal`,
`BindIdent`, `MakeCell`.

## The issue — slot/stack invariant fragility

Three bugs in a six-month window were all variants of the same root cause:
"the slot index happened to land on live operand-stack data."

### `3c79e3d` — Range pattern slot collision

`Pattern::Range` with two bounds used `StoreLocal(0)` as a scratch slot
("the first local always exists, we don't need it after the test"). True
when slot 0 was unused, but **wrong** when slot 0 was the iterator variable
of an enclosing `for` loop:

```oxy
for n in 0..20 {
    match n {
        3..=9 => { ... },   // StoreLocal(0) clobbered the iterator
        _ => {},
    }
}
```

Fixed pointwise by allocating fresh temp slots (`__range_scrut`, `__range_bool`)
via `sym.define()` instead of reusing slot 0.

### `7bbc909` — Enum tuple variant destructure with 3+ fields

`EnumVariantEqual` for `Foo::Variant(a, b, c, d)` pushed the four field
values onto the operand stack in one go. The first `BindIdent` then bound
slot 0 — but because the binding's stack position landed _inside_ the bulk-
pushed data, the second `BindIdent` saw shifted data and bound the wrong
field to slot 1.

Fixed pointwise by mirroring `Pattern::Tuple`'s approach: save the variant
value to a temp slot, then for each field do `LoadLocal(tmp)`, `EnumDataGet(i)`,
`BindIdent(slot)` — one value on top per bind.

### `f81d3f8` — `StoreLocal` + `continue` corrupting closure captures

When a `continue` statement in a closure body skipped the trailing
`self.ip += 1`, `StoreLocal` re-executed on the next loop iteration,
consuming successive operand-stack values and overwriting the Cell-wrapped
captured variable with the wrong values.

Fixed pointwise by restructuring the dispatch loop so `continue`-style
opcodes return a `StepOutcome::Continue` and the bump logic lives in one
place.

### Meta-finding

Each fix was a workaround for the same conflation. The invariants — "slot
N lives at stack position base+N; do not push scratch values at base+N;
do not Pop below max_slot" — lived only in comments on individual handlers
(see e.g. the insert-vs-assign heuristic on `BindIdent`). New opcodes could
violate them silently.

## Solutions considered

### (a) Status quo + documentation

Write the invariants down in a module-level doc and add debug assertions
that check the operand-stack length matches expectations after each opcode.

- **Pros:** Minimal change. No perf risk. No allocator pressure.
- **Cons:** Future opcodes can still violate the invariants. Relies on
  contributors reading the docs and writing correct assertions. Doesn't
  remove the root cause — only makes violations louder.

### (b) Renumber slots to a dense capture index (for closures only)

Add `LoadCapture`/`StoreCapture` opcodes. The compiler's closure analysis
re-numbers captured variables to a dense index, so the closure's frame is
sized exactly `captures.len() + args.len() + body_locals`.

- **Pros:** Tighter memory in closure frames.
- **Cons:** Orthogonal to the root cause — doesn't help non-closure
  pattern bugs. Adds two opcodes and a compiler pass.

### (c) Separate `Frame.locals: Vec<Value>` from operand stack (chosen)

Each frame owns its locals as a typed Rust `Vec<Value>`, distinct from
`self.stack`. Slot indexing becomes ordinary Vec indexing — `frame.locals[slot]`.
The operand stack becomes pure LIFO.

- **Pros:** Eliminates the entire class of bugs by construction. Locals
  cannot be popped, scratch cannot land on slot positions, the `BindIdent`
  insert-vs-assign heuristic disappears, `StoreLocal`'s grow-with-Unit
  loop disappears. Each opcode's stack effect becomes localisable.
- **Cons:** One `Vec::with_capacity(frame_size)` per Call. Requires touching
  every slot-addressable opcode (small set: 4 opcodes + Call/Return/Closure/
  CallClosure/MethodCall/run_closure + 4 operator-overload sites).

### (d) Register VM

Replace the stack VM with a register-based VM.

- **Pros:** Cleaner still. Fewer instructions per program.
- **Cons:** Massive rewrite. Out of scope.

## Decision: option (c)

Sub-choices and their rationale:

### Frame sizing — pre-sized at Call time from compiler-known `frame_size`

The compiler already tracks the highest slot used per function as
`SymTable.next_slot`. We added `Chunk.fn_frame_sizes: HashMap<usize, usize>`
keyed by function entry IP. At Call time the VM does `vec![Value::Unit; frame_size]`.

Alternative: grow-on-demand inside opcodes (mirror today's `StoreLocal`
pad-with-Unit). Rejected because it preserves the realloc hot path and
the "what if slot doesn't exist yet" branches — a smaller-fragility version
of the bug class we're trying to delete. The pre-sized path also matches
how Lua / Python / V8 frames work.

### Closure capture layout — preserve outer slot indices

The closure body is compiled inside the parent's symbol table and addresses
captures by their parent-frame slot numbers (via `SymTable.define_at`).
We kept that — the closure's `locals` vec is sized to `captures_end +
arg_count + body_locals`, and captured values are placed at their original
outer-slot indices. Args go at `captures_end..captures_end + arg_count`.
The compiler is byte-identical.

Alternative: renumber captures to a dense index via `LoadCapture`/`StoreCapture`.
Rejected for this PR because it braids two failure surfaces together —
making it harder to attribute regressions. Can ship later as an independent
change if the slot padding ever proves wasteful.

### PR shape — single PR

Phased rollout (introduce `Frame.locals` alongside the unified stack,
migrate opcodes one at a time, then delete the old representation) was
rejected because every intermediate state would be more complex than
either endpoint.

## Consequences

- **Future opcode additions cannot collide with locals.** Locals are a
  typed `Vec` owned by the frame; the operand stack is a separate `Vec`
  with no protected region. The two cannot interleave by construction.
- **Pattern-emission contract is now operand-stack-only.** "Pattern leaves
  `[scrutinee, bool]` on the operand stack" is now an invariant about a
  pure LIFO, which is easier to reason about and assert.
- **`frame_size` underestimates are loud.** If the compiler computes a
  too-small `frame_size`, an `index out of bounds` panic fires immediately
  instead of silently overwriting an unrelated slot.
- **One `Vec` allocation per Call.** Pre-sizing replaces the old `StoreLocal`
  grow-on-demand path; the asymptotic cost is similar but allocation timing
  is more predictable. Benchmarked recursive `fib(30)` — within noise.
- **`Pop` is unconditional**, removing one branch from a hot opcode.
- **`BindIdent` lost its insert-vs-assign heuristic** — the one-line write
  `locals[slot] = val` replaces ~12 lines of stack-juggling.

## Reference

- Plan: `~/.claude/plans/when-i-try-to-dazzling-oasis.md`
- Module doc: top of `crates/oxy-core/src/vm/mod.rs`
- Pre-refactor regression tests: `test_for_loop_with_range_pattern`,
  `test_nested_match_in_closure`, `test_closure_mutating_captured_in_loop`,
  `test_deeply_nested_pattern_destructure`, `test_recursive_call_inside_closure`
  in `crates/oxy-core/tests/vm_tests.rs`.
