# Oxy Register IR — Design & Invariants (Source of Truth)

> This document is the **canonical reference** for Oxy's register IR. It records
> the *intended* semantics of the IR as it exists today and the inconsistencies /
> stack-VM-era assumptions still embedded in the lowering code. Treat it like a
> constitution: when fixing the JIT, conform code to this document, and when the
> design genuinely changes, update this document in the same commit.

Pipeline position:

```
AST → ir_gen (Register IR + CFG) → codegen (Cranelift CLIF) → native
```

Primary sources:
- `jit/ir.rs` — IR types (`IrOp`, `Terminator`, `IrFunction`, `BasicBlock`)
- `jit/ir_gen/mod.rs` — AST → Register IR lowering
- `jit/codegen.rs` — Register IR → CLIF emission
- `jit/context.rs` — `JitContext` runtime frame
- `jit/ffi.rs` + `jit/runtime.rs` — `oxy_*` FFI runtime and arithmetic helpers

---

## 1. Register / value model

- A register is `pub(crate) type Reg = usize` (`ir.rs:11`) — an index into an
  infinite virtual register space. Each value-producing `IrOp` **defines** a
  fresh register, drawn from a per-function monotonic counter (`alloc_reg`,
  `ir_gen/mod.rs:109`; `next_reg` reset to 0 at each function, `ir_gen/mod.rs:500`).
- Constants are stored inline in the op: `ConstInt(Reg, i64)`,
  `ConstString(Reg, String)`, etc.
- **A register has two possible physical residences at codegen time:**
  - `regs: HashMap<Reg, clif::Value>` (`codegen.rs:190`) — the value lives in a
    real Cranelift SSA value (an `i64`). Only `ConstInt` / `ConstBool` /
    `ConstUnit` and the comparison fast-path land here.
  - `reg_slot: HashMap<Reg, usize>` (`codegen.rs:191`) — the value was spilled
    into a `Value` slot in the runtime buffer. **Everything produced via FFI
    lands here** (all arithmetic, loads, calls, and the float/char/string
    constants).
  - `push_reg` (`codegen.rs:410`) is the single bridge: a CLIF value is pushed
    with `oxy_push_int`; a spilled slot is materialized with `oxy_load_local`.

## 2. Instruction categories (`IrOp`, `ir.rs:44-119`)

| Category | Variants |
|---|---|
| Constants | `ConstInt`, `ConstFloat`, `ConstBool`, `ConstChar`, `ConstUnit`, `ConstString` |
| Locals | `LoadLocal(Reg, slot)`, `LoadLocalRaw(Reg, slot)`, `StoreLocal(slot, Reg)` |
| Arithmetic | `Add`, `Sub`, `Mul`, `Div`, `Rem` |
| Comparison | `Eq`, `Neq`, `Lt`, `Gt`, `Le`, `Ge` |
| Logical | `And`, `Or` |
| Bitwise | `BitAnd`, `BitOr`, `BitXor`, `Shl`, `Shr` |
| Unary | `Neg`, `Not`, `BitNot` |
| FFI | `CallBuiltin { result, func: &'static str, args: Vec<Reg>, immediates: Vec<usize>, strings: Vec<String> }` |
| Result/error plumbing | `ReadResult(Reg)`, `WriteResult(Reg)`, `SetError(Reg)`, `CheckError(Reg)` |
| Merge | `Phi(Reg, Reg, Reg)` (result + two sources) |
| Misc | `Copy(Reg, Reg)` |

- `LoadLocalRaw` skips `Cell` unwrapping; it is used for method-call / closure-call
  receivers that must keep their `Cell` wrapper.
- `CallBuiltin` is the only escape hatch into the runtime: `args` are register
  operands passed on the operand stack, `immediates` are `usize` ABI args (arg
  counts, field counts, slot indices), `strings` are static metadata (function
  names, method names, field names, paths).

## 3. Temporary value rules

- Pure SSA-style allocation: one fresh `Reg` per op, never reused, never freed.
  There is **no operand-stack discipline at the IR level** — temporaries are just
  register numbers.
- At codegen, every FFI-backed op immediately spills its result into a buffer
  slot via `spill_result` (`codegen.rs:393`). Consequently most temporaries are
  buffer slots, and only a few (integer/bool/unit constants, comparison results)
  remain as CLIF SSA values.

## 4. Variable load/store semantics

- `ir_gen` tracks `locals: HashMap<String, usize>` (name → slot index) and a
  running `local_count`. `alloc_local` (`ir_gen/mod.rs:121`) assigns the next
  slot and bumps `local_count`.
- Function parameters get slots `0..n` first (`ir_gen/mod.rs:510`).
- `let x = e` → eval `e` to a register, optional width coercion (`coerce_reg` /
  `local_types`), then `StoreLocal(slot, reg)`.
- Reading an identifier → `LoadLocal(r, slot)`. Method/call receivers use
  `LoadLocalRaw` to preserve `Cell` wrapping.
- At runtime, slot `N` is `buffer[N]` (see §9 layout).

## 5. Block & CFG structure

- `IrFunction` (`ir.rs:17`): `fn_index`, `name`, `blocks: Vec<BasicBlock>`,
  `entry: BlockId`, `local_count`, `return_type`, `params`, `captures`,
  `is_async`.
- `BlockId = usize` (`ir.rs:14`). Blocks are stored in a `Vec` and indexed by
  position; each block's `id` mirrors its index. The `block_mut(id)` accessor is
  on `IrFunction` (lowering mutates blocks in place; there is no immutable
  `block(id)` getter).
- `BasicBlock` (`ir.rs:35`) = `{ id, ops: Vec<IrOp>, terminator, predecessors:
  Vec<BlockId> }` — a list of straight-line ops plus exactly one terminator.
- During lowering, `ir_gen` keeps a `current_block` cursor: `emit` appends to it,
  `start_block(id)` switches to it, `terminate(t)` sets its terminator
  (`ir_gen/mod.rs:182-231`).
- A newly created block's terminator defaults to `Halt`. `Terminator::is_default()`
  (true only for `Halt`) is the **"block not yet sealed"** signal — control-flow
  helpers only emit an auto-`Jump` to a merge/continuation block when the current
  terminator is still default (i.e. no explicit `return` / `break` happened).

## 6. Branch / terminator semantics (`ir.rs:169-184`)

| Terminator | Meaning |
|---|---|
| `Return(Reg)` | Return the register's value from the function |
| `Jump(BlockId)` | Unconditional branch |
| `Branch { cond, then_block, else_block }` | If `cond` truthy → `then_block`, else `else_block` |
| `Halt` | End of program; also the default terminator of a fresh block |
| `Panic(Reg)` | Set error from register, return error discriminant |

- `Branch` reads `cond` from either `regs` (CLIF `icmp_imm != 0`) or, if spilled,
  via `oxy_read_local_i64`, then lowers to `brif` (`codegen.rs:310-336`).
- Every non-entry CLIF block carries exactly **one** block param: `ctx: i64`
  (`codegen.rs:169-174`). There are **no value-carrying block params** — see §11.

## 7. Function-call semantics

- `ir_gen` funnels essentially all calls through `CallBuiltin`:
  - Regular call → callee resolved to a `Value::Function` via `oxy_push_named_fn`
    (or `LoadLocalRaw` if a local holds the closure), then `oxy_call_closure`
    with `immediates = [argc]`.
  - Method call → `oxy_method_call`, receiver as first arg, `strings = [method]`.
  - Enum constructor → `oxy_make_enum_variant`.
  - Async forms → `oxy_spawn_ffi` / `oxy_sleep_ffi` / `oxy_select_ffi`.
- Argument evaluation is strict **left-to-right**.
- At codegen (`codegen.rs:705`): the CLIF ABI args are
  `[ctx, (ptr,len) per string, i64 per immediate]`. **Register args are pushed
  onto the runtime operand stack via `push_reg`, not passed as ABI params**; the
  FFI function pops them.

## 8. Return semantics

- `ir_gen` emits `Terminator::Return(reg)` both for explicit `return` and for the
  implicit tail expression, applying return-type coercion
  (`coerce_reg_to_type_info`) first (`ir_gen/mod.rs:516-530, 595-608`).
- Codegen has **two return paths** (`codegen.rs:257-298`):
  - `i64` / `byte` (`TypeInfo::I64 | U8`) → `oxy_set_result_i64(ctx, clif_val)`
    directly from a CLIF value.
  - everything else → `push_return_value` (typed `oxy_push_*` by `return_type`)
    then `oxy_return`, which pops the operand stack into `ctx.result`.
- The CLIF function itself returns an `i64` **discriminant**: `0` = ok, `2` =
  error (`oxy_error_discriminant`). The real return value lives in `ctx.result`.

## 9. Runtime frame (`context.rs:70`, `#[repr(C)] JitContext`)

- `buffer: *mut Value` — one allocation laid out as:

  ```
  [ locals: 0 .. local_count ] [ operand stack: grows UP from local_count ]
                                                  ...
                               [ spill slots: grow DOWN from capacity-1 ]
  ```

- `sp` = operand-stack depth. `push_slot` computes
  `buffer.add(local_count + sp)` (`context.rs`).
- Spill slots for register values grow **downward** from `capacity-1`
  (`codegen.rs:176-188`); `capacity = local_count + STACK_CAP`, `STACK_CAP = 2048`.
- Other fields: result/error state (`result: Value`, `error_msg: [u8; 1024]`,
  `error_len`), the closure-call function-pointer table, and a `tables` pointer.
  (`async`/`spawn` run eagerly to completion — see `scheduler.rs` — so there is
  no yield/resume state on the context.)

## 10. Type / value expectations

- CLIF only ever sees `i64` (with transient `i8` / `i32` / `f64` at push sites).
  The function signature is `(ctx: i64) -> i64`. Slot indices, immediates, string
  `ptr`/`len`, and the return discriminant are all `i64`.
- Real Oxy values are the `Value` enum, living in the buffer. Type tags survive
  across blocks and calls **because values are materialized as `Value`** (through
  the operand stack), never as bare CLIF integers.
- Comparisons (`Eq` … `Ge`) have a CLIF fast path **only when both operands are
  already in `regs`** (native `icmp` + `uextend`, `codegen.rs:547-606`); otherwise
  they fall back to FFI (`oxy_eq`, …). `ConstInt/Bool/Unit` are the main producers
  of CLIF-resident registers.

## 11. Ownership / lifetime assumptions

- `Value` owns heap data (`Rc`, `String`, `Vec`). The load-bearing invariant is
  `move_value(src, dst)` (`ffi.rs:24`): `dst.write(src.read()); src.write(Unit)`.
  A bare `ptr::read` + `write` would create two owners and double-free on buffer
  `Drop`. **Every cross-slot transfer must clear the source slot to `Value::Unit`.**
- `oxy_load_local` reads a shallow bitwise copy, clones the live value, then
  `mem::forget`s the shallow copy to avoid double-free (`ffi.rs:134`).
- `pop` clears the popped slot to `Unit`; `invoke_jit_fn` uses `move_value` to
  hand arguments to the callee frame (`ffi.rs:691`).
- This invariant has historically been violated in `invoke_jit_fn`,
  `oxy_call_closure`, and `pop` — the canonical "fix it once" abstraction is
  `move_value`. Do not reintroduce raw `ptr::read`/`write` between slots.

## 12. SSA classification

**Pseudo-SSA at the IR level; register/slot machine fed by an operand stack at
the codegen level.**

- **IR:** each `Reg` is defined exactly once (monotonic counter, never
  reassigned), and merges use explicit `Phi` ops. This is SSA-with-phi.
- **Codegen:** values do **not** flow through CLIF block params (only `ctx`
  does). A register is either a CLIF temp or a buffer slot, and phi merge is
  realized by pushing the source `Value`s onto the operand stack at the jump and
  popping them into pre-allocated spill slots at the target block
  (`codegen.rs:169-239, 300-309`). So at runtime the model is a register/slot
  machine fed by a `Value` operand stack — not true SSA.

---

## Stack-VM-era vestiges & inconsistencies

These are documented so they are not mistaken for intended design. They are the
prime suspects when JIT tests fail and the first candidates for cleanup.

1. **Stack-based FFI calling convention under a register IR.** All op operands and
   call args are re-materialized onto the `JitContext` operand stack via
   `push_reg`, then popped by FFI (`call_ffi_binary` `codegen.rs:429`,
   `CallBuiltin` `codegen.rs:705`). The operand stack — not registers — is the
   real runtime value medium. This is the central holdover from the stack VM.

2. **Phi bypasses CLIF block params.** Only `ctx` is threaded as a block param;
   phi values travel via the operand stack + downward-growing spill slots
   (`phi_args` pushed on `Jump`, popped into `phi_slot` at block entry). Cranelift's
   native SSA/phi mechanism is unused.

3. **Two contradictory Phi/Copy lowerings (likely test-failure source).**
   `compile_op` has `IrOp::Phi(r, a, _) => regs.insert(r, regs[a])`
   (`codegen.rs:610`) — it **ignores the second source** and assumes source `a`
   is CLIF-resident. This both contradicts the operand-stack phi mechanism (#2)
   and panics on a missing `regs[a]` when `a` was spilled (the common case). The
   same fragile assumption applies to `Copy(r, a) => regs.insert(r, regs[a])`
   (`codegen.rs:607`).

4. **Phi-isolation hack in `ir_gen`.** `gen_if` stores the phi result into a
   synthetic `__phi_tmp` local, jumps to a continuation block, and reloads it,
   explicitly to "keep Phi stack ops isolated" (`ir_gen/mod.rs:~1442`). This is a
   workaround papering over stack-materialized phi rather than a clean SSA merge.

5. **Two stacks in one buffer + magic `STACK_CAP = 2048`.** The operand stack
   grows up from `local_count` while spill slots grow down from `capacity-1`; the
   code notes they "meet only if combined usage exceeds capacity" — a classic
   stack-VM collision guard, with no overflow handling beyond `grow()`.

6. **Hybrid return mechanism.** `i64` / `byte` use `oxy_set_result_i64` (register
   direct) while all other types use push + `oxy_return` (stack). Closures must
   **guess** their return type to pick the path, with `Unknown` falling back to a
   scalar-only `push_int` (`ir_gen/mod.rs:~2538`).

7. **Comparison fast path is load-bearing only by luck.** It fires only when both
   operands are still CLIF-resident; once a value has passed through any FFI op it
   is spilled, so the fast path is rarely taken and the two branches must stay
   behaviorally identical or bugs appear.

8. **Per-function vs engine `local_count`.** Each function carries its own
   `local_count` (`fn_local_counts`, `codegen.rs:19`); a prior bug used `main`'s
   count for every frame, causing silent heap corruption only when a function had
   more locals than `main`. Keep frame sizing per-function.

---

## How to use this document

- When a JIT feature test fails, follow the **JIT Debugging Protocol** in
  `CLAUDE.md` (read the `.ox` test → trace `ir_gen` → trace `codegen` → read the
  FFI helper), then check the failure against the vestige list above. Items #3,
  #6, and #7 are the highest-probability culprits.
- When you change the IR (new `IrOp`, new terminator, new residence rule, changed
  phi/return strategy), update the relevant section here in the **same commit**.
- Do not "fix" a symptom with a magic offset, guard, or special case. If a fix
  needs one, it is probably resolving one of the architectural vestiges above —
  fix the architecture and document it here.
