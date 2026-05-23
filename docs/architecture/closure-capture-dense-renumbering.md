# Closure Capture Dense Renumbering

Architecture decision record. Refactor landed 2026-05-23.

## Background — closure capture layout after locals-separation

The locals/operand-stack separation refactor (see
`vm-locals-stack-separation.md`) gave every call frame its own `locals:
Vec<Value>` distinct from the operand stack. A natural follow-on
question: **at what indices should a closure's captured variables live
inside the callee frame's `locals` vec?**

Two answers exist:

- **Option A — preserve outer slot indices.** A closure that captures
  the parent's slot 7 reads its captured value from `locals[7]` of its
  own frame. The compiler defines each capture via
  `sym.define_at(name, outer_slot)`, so the body emits
  `LoadLocal(outer_slot)` and the VM places the value there at
  `OpCode::CallClosure` time. Simple, zero-bytecode-change refactor —
  what we shipped initially in commit `3a91fb9`.

- **Option B — dense renumbering from 0.** Captures live at
  `locals[0..N]` where `N = captures.len()`, regardless of which slots
  they came from in the parent. Params follow at `locals[N..N+P]`, then
  body-locals after.

Option A is a leaky abstraction: the parent's slot numbering bleeds
into the closure's runtime frame layout. The closure's `frame_size`
becomes proportional to `max(outer_slot) + 1`, not to the closure's
own needs.

```text
fn outer() {
    let a = 1;     // slot 0
    let b = 2;     // slot 1   (not captured)
    let c = 3;     // slot 2
    let d = 4;     // slot 3   (not captured)
    let f = |x| a + x + c;
    //          ^      ^
    //          captures: a (outer slot 0), c (outer slot 2)
}
```

| Layout | Closure frame |
|---|---|
| Option A | `[a, Unit, c, Unit, x]` — frame_size = 5 |
| Option B | `[a, c, x]` — frame_size = 3 |

Under Option A, every closure carries padding for skipped outer slots
forever, regardless of how few variables it actually uses.

## The issue

1. **Wasted memory per call.** A closure compiled inside a parent
   with 50 local variables that captures one of them still allocates
   a 50-slot frame on every invocation. Padding slots hold `Value::Unit`
   (cheap), but the vec allocation and zeroing both cost time and space.
2. **Leaky encapsulation.** A closure's runtime size depends on its
   parent's local count — moving an unrelated `let` into the parent
   inflates every call into the closure.
3. **Disassembler/debugger noise.** A bytecode dump shows the closure
   reading from `LoadLocal(7)` rather than `LoadLocal(0)`, leaking the
   parent's symbol layout into the child's instructions.

None of these were *bugs* — Option A is functionally correct. They are
architectural friction that compounds as closures nest or as parent
functions accumulate locals.

## Solutions considered

### a) Keep Option A indefinitely

**Pro:** Zero code change. Captures already work.
**Con:** Frame size grows with parent's local count, not closure's
needs. The leaky abstraction stays.

### b) Dense renumbering with new opcodes (`LoadCapture(i)` / `StoreCapture(i)`)

A separate opcode family for capture access, with the closure frame
carrying a distinct `captures: Vec<Value>` field alongside `locals`.

**Pro:** Clearest separation in the disassembler — `LoadCapture(0)` vs
`LoadLocal(0)` is immediately readable. Sets up future optimizations
(shared capture environments, lambda-lifted indirection).
**Con:** Doubles the surface area — VM dispatch, disassembler, and
compiler all need new arms. Touches every site that reads/writes
captures (BindIdent, MakeCell, Assign). Higher blast radius than the
problem demands.

### c) Dense renumbering reusing `LoadLocal`/`StoreLocal` *(chosen)*

Captures live at the front of the closure's `locals` vec at dense
indices `0..N`. The compiler registers them in the fresh
closure-scoped `SymTable` via plain `sym.define(name)` (sequential
slot allocation), then defines params, then body-locals. Body bytecode
uses `LoadLocal(i)` / `StoreLocal(i)` against the dense indices —
identical opcodes to the parent function, no new dispatch surface.

**Pro:** ~30 lines net change. No new opcodes. Frame size is exactly
`captures + params + body_locals`. Disassembler still readable because
the closure body addresses slots from 0 up, just like any function.
**Con:** Captures and body-locals share the same opcode family —
disassembler can't visually distinguish "this is a capture" vs "this
is a body-local". `FunctionData` still carries `captured_names:
Vec<String>` so the VM knows the boundary at frame-construction time.

## Decision

**Option (c).** Captures get dense indices via `sym.define`. Params
get the next dense slots. Body-locals follow. The `FunctionData`
struct carries `captured_names: Vec<String>` in dense order; at
`OpCode::CallClosure` (and `run_closure`) the VM places
`closure_env.get(captured_names[i])` at `locals[i]`, then args at
`locals[N..N+arg_count]`.

`Chunk.fn_frame_sizes[target_ip]` was already populated from the
compiler's `sym.next_slot` at end of closure compilation. With dense
numbering it equals `N + P + body_locals` and is exactly the size the
frame needs.

## Consequences

### Memory

A closure that captures K of its parent's L locals now allocates K +
P + body_locals slots per call, not max(parent slots used) + P +
body_locals. For closures compiled inside large functions this
shrinks frames substantially.

### Closure value layout

`FunctionData.captured_slots: Vec<(String, usize)>` (name, outer slot)
became `captured_names: Vec<String>` (dense order). Outer slot is
no longer needed at call time — `closure_env` is keyed by name and
already populated at `OpCode::Closure` creation time using the outer
slot from `closure_meta`. The third tuple field in
`Chunk.closure_meta` entries (`(name, outer_slot, is_mut)`) keeps
`outer_slot` purely for that creation-time fetch.

### Cell semantics unchanged

Mutable captures from `let mut x` in the parent are wrapped in
`Value::Cell(Rc<RefCell<Value>>)`. The parent holds the Cell at its
outer slot; the closure now holds the same Cell at dense slot `i`.
The shared `Rc` makes mutations visible across the boundary
regardless of where each side stores the Cell. No change to
`LoadLocal` / `StoreLocal` dereference logic.

### `SymTable::define_at` removed

The only caller (the closure compilation path) is gone. The method
itself is deleted to keep the symbol table API small.

### Forward path — Option B with new opcodes

Still on the table if and when:
- Profiling reveals capture access dominates a hot loop and a
  separate `Frame.captures` vec would let us skip locals-vec
  reallocation per call.
- Future shared-capture-environment work (multiple closures from the
  same parent sharing one environment) needs a level of indirection
  that `LoadCapture(i)` would express more naturally than `LoadLocal`.

No evidence today; deferring.

## References

- Prior ADR: `vm-locals-stack-separation.md` — the prerequisite refactor.
- Implementation: `crates/oxy-core/src/compiler/expr.rs` (Expr::Closure),
  `crates/oxy-core/src/vm/mod.rs` (OpCode::CallClosure, run_closure,
  OpCode::Closure), `crates/oxy-core/src/types/mod.rs` (FunctionData).
