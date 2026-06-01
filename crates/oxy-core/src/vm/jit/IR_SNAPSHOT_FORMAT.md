# Oxy Register IR — Canonical Snapshot Serialization Format (Spec)

> **Status:** implemented by `jit/ir_snapshot.rs` (`gen_ir_snapshot` in `vm/api.rs`).
> This document is the spec that serializer conforms to.
> **Scope:** defines the *canonical textual serialization* of the Register IR
> (`IrFunction` / `BasicBlock` / `IrOp` / `Terminator` from `jit/ir.rs`) for use in
> golden / snapshot tests and stable diffs.
> **Authority:** semantics are governed by `IR_DESIGN.md` (the IR source of truth).
> This document governs only how that IR is *printed*.

## 0. Goals & non-goals

The canonical snapshot MUST be:

- **Deterministic** — the same `IrFunction` produces byte-identical output on every
  run, every platform, every process. No HashMap iteration order, no addresses, no
  timestamps, no floating-point platform variance.
- **Semantic / IR-level** — it serializes the register IR *as it exists before
  codegen*. It MUST NOT leak codegen residence (CLIF values vs spill slots), the
  operand stack, `STACK_CAP`, `regs`/`reg_slot`, or any `JitContext` runtime state.
- **Minimal-noise** — a small change to the IR produces a small textual diff.
- **Unambiguous** — every `IrOp` and `Terminator` has exactly one printed form.

Non-goals: this is not a parseable on-disk format (no round-trip parser is
required), and it is **not** the existing `IrFunction::dump()` / `Display`
(`ir.rs:249-341`). That dump uses `{:?}`, raw ids, and arbitrary block order — it
is for live `OXY_VM_TRACE` tracing only and is explicitly **non-canonical**. The
canonical serializer is a separate routine.

---

## 1. Grammar (EBNF)

```
snapshot      = function , { blank_line , function } , "\n" ;
blank_line    = "\n" ;

function      = header , "\n" ,
                "  locals: " , uint , "\n" ,
                [ "  captures: " , capture_list , "\n" ] ,
                block , { "\n" , block } , "\n" ,
                "}" ;
header        = "fn " , fq_name , "(" , [ param_list ] , ")" ,
                " -> " , type , [ " async" ] , " {" ;
param_list    = param , { ", " , param } ;
param         = ident , ": " , type ;
capture_list  = capture , { ", " , capture } ;
capture       = ident , "@" , slot ;

block         = "  " , block_label , ":" , "\n" ,
                { "    " , instruction , "\n" } ,
                "    " , terminator ;
block_label   = "bb" , uint , [ "(" , block_tag , ")" ] ;
block_tag     = "entry" | ( "preds: " , bbref , { ", " , bbref } ) ;
bbref         = "bb" , uint ;

instruction   = ( reg , " = " , rhs ) | effect_op ;   (* see §6 table *)
terminator    = "ret " , reg
              | "jump " , bbref
              | "branch " , reg , " -> " , bbref , ", " , bbref
              | "halt"
              | "panic " , reg ;

reg           = "v" , uint ;
slot          = "$" , uint ;
uint          = digit , { digit } ;
```

`rhs` and `effect_op` forms are enumerated exhaustively in §6.

---

## 2. Deterministic ordering rules

Ordering is fixed at every level. No collection is ever printed in hash order.

### 2.1 Functions (program-level)

Sort all functions by the pair `(fq_name, fn_index)`, ascending, where `fq_name`
is compared by Unicode scalar (byte) order. Name is primary; `fn_index` is the
tie-break only. This decouples the snapshot from source declaration order, so
reordering items in a `.ox` file produces no diff.

Functions are separated by exactly one blank line. The file ends with exactly one
trailing newline.

### 2.2 Blocks (within a function) — canonical order = RPO

Blocks are **renumbered** and printed in **reverse postorder (RPO)** from `entry`,
computed from the CFG (not from stored block ids). Successor order is fixed:

```
successors(terminator):
  Jump(t)                      -> [t]
  Branch{then_block,else_block}-> [then_block, else_block]   (* then first *)
  Return | Halt | Panic        -> []
```

Algorithm (deterministic given the fixed successor order):

```
postorder = []
visited   = {}
dfs(b):                       # iterative or recursive; order is identical
  visited.add(b)
  for s in successors(block(b).terminator):   # in the fixed order above
    if s not in visited: dfs(s)
  postorder.append(b)
dfs(entry)
rpo = reverse(postorder)
unreachable = [ id for id in 0..n if id not in visited ]   # ascending raw id
canonical_block_order = rpo ++ unreachable
```

Reachable blocks come first in RPO; any **unreachable** blocks (present in the
`blocks` Vec but with no path from entry) are appended in ascending raw-id order so
the snapshot still faithfully represents stored IR. The entry block is always the
first line of the block list and is tagged `(entry)`.

### 2.3 Instructions (within a block)

Printed in stored `block.ops` order — this is execution order and is already
deterministic. Never reorder, sort, or dedupe. The terminator is always the last
line of the block.

### 2.4 Registers

Renumbered densely to `v0, v1, …` per function (see §3). Within an op, operands are
printed left-to-right in the field order defined in §6 — never sorted (operand
order is semantic, e.g. `sub`, `div`, `shl`).

### 2.5 CallBuiltin sub-lists

`args`, `immediates`, and `strings` are printed in their stored order. They are
**never** sorted — their order is part of the call's meaning.

### 2.6 Predecessors

A block's predecessor list is **recomputed** from CFG edges during serialization
(every block whose terminator targets this block), then printed in
`canonical_block_order`. The stored `BasicBlock.predecessors` field is **ignored**
for snapshots — it may be stale, and trusting it would inject nondeterministic
noise.

---

## 3. Register naming (stable & reproducible)

- A register prints as `v<N>`, `N` a 0-based dense integer.
- Numbers are assigned by **first definition** in canonical traversal order
  (canonical block order from §2.2, then op order within each block). Use a
  **two-pass** scheme so loop/phi back-edges resolve:
  1. Pass 1: walk all blocks in canonical order; for each op, for the register it
     *defines* (per the def/use table in §6), assign the next free `vN` if unseen.
  2. Pass 2: print, resolving every operand to its assigned name.
- Raw `Reg` ids from `alloc_reg` are **never printed**. They encode allocator state
  and would tie the snapshot to unrelated counter drift.
- A register used but never defined anywhere in the function is **malformed IR**.
  Print it as `<undef:r{raw}>` (preserving the raw id) so the bug is visible rather
  than silently renamed. A correct serializer never emits this for well-formed IR.
- **Known shift property:** because numbering follows definition order, inserting an
  instruction renumbers every later register. This is inherent to SSA value
  numbering and is accepted; the numbering is still a pure, reproducible function of
  IR content. Snapshot consumers should treat register *identity* as the `vN` token,
  not assume a register keeps its number across edits.
- **Local slots** print as `$<index>` using the raw slot index, which is a stable,
  deterministic identifier from `ir_gen`. Slot operands never carry the variable
  name (names appear once, in the function header). This keeps `store.local` /
  `load.local` lines stable when a variable is renamed.

---

## 4. Control-flow representation

- **Block header:** `bb<N>(<tag>):` where `<tag>` is `entry` for the entry block,
  otherwise `preds: bbA, bbB, …` (recomputed predecessors, canonical order). A
  non-entry block with no predecessors prints `bb<N>(preds: ):` — the empty list is
  shown so unreachable/dead blocks are obvious.
- **Branches and jumps** reference blocks by their canonical renumbered names only.
- **Terminators** are the final line of each block, indented like instructions
  (4 spaces), one of:
  - `ret <reg>`
  - `jump bb<t>`
  - `branch <cond> -> bb<then>, bb<else>`
  - `halt`
  - `panic <reg>`
- **Phi:** printed `v = phi <s0>, <s1>` (exactly two sources — the IR `Phi(Reg,
  Reg, Reg)` is binary). The IR does **not** bind each source to a predecessor; the
  binding is positional and is read from the block's predecessor list in header
  order. This positional, predecessor-less phi is a known limitation (see
  `IR_DESIGN.md` §12 and vestige #2/#4); the snapshot represents the IR as-is and
  does **not** invent predecessor labels it cannot derive.

---

## 5. Function boundaries

Each function prints as:

```
fn <fq_name>(<params>) -> <ret_type>[ async] {
  locals: <local_count>
  captures: <name>@$<slot>, ...        (line omitted when there are no captures)
  <blocks, blank-line-separated>
}
```

- `<fq_name>`: the function's fully-qualified `name`, verbatim (e.g.
  `calc::triple`, `main::{closure#0}`). Deterministic from `ir_gen`.
- `<params>`: `name: type` in declared order (from `IrFunction.params`). Param i
  occupies local slot i; that mapping is documented here, not repeated per line.
- `<ret_type>`: canonical type rendering (§7.4).
- ` async`: appended to the header iff `is_async`; omitted otherwise.
- `locals:` always present (`local_count`). `captures:` present only when
  `captures` is non-empty, listed in stored order.
- The closing `}` is on its own line at column 0. Functions are separated by one
  blank line.

---

## 6. Op encoding — included information (authoritative table)

Every `IrOp` maps to exactly one printed form. `vR` = canonical name of the defined
register; `vA`/`vB` = canonical names of operand registers; `$k` = slot index.

| `IrOp` | Printed form | Defines |
|---|---|---|
| `ConstInt(R, n)` | `vR = const.int <n>` | vR |
| `ConstFloat(R, n)` | `vR = const.float <float>` (§7.3) | vR |
| `ConstBool(R, b)` | `vR = const.bool <true\|false>` | vR |
| `ConstChar(R, c)` | `vR = const.char '<esc>'` (§7.2) | vR |
| `ConstUnit(R)` | `vR = const.unit` | vR |
| `ConstString(R, s)` | `vR = const.str "<esc>"` (§7.1) | vR |
| `LoadLocal(R, k)` | `vR = load.local $k` | vR |
| `LoadLocalRaw(R, k)` | `vR = load.local.raw $k` | vR |
| `StoreLocal(k, S)` | `store.local $k, vS` | — |
| `Add/Sub/Mul/Div/Rem(R,A,B)` | `vR = add\|sub\|mul\|div\|rem vA, vB` | vR |
| `Eq/Neq/Lt/Gt/Le/Ge(R,A,B)` | `vR = eq\|ne\|lt\|gt\|le\|ge vA, vB` | vR |
| `And/Or(R,A,B)` | `vR = and\|or vA, vB` | vR |
| `BitAnd/BitOr/BitXor(R,A,B)` | `vR = bitand\|bitor\|bitxor vA, vB` | vR |
| `Shl/Shr(R,A,B)` | `vR = shl\|shr vA, vB` | vR |
| `Neg/Not/BitNot(R,A)` | `vR = neg\|not\|bitnot vA` | vR |
| `Copy(R,A)` | `vR = copy vA` | vR |
| `Phi(R,A,B)` | `vR = phi vA, vB` | vR |
| `ReadResult(R)` | `vR = read.result` | vR |
| `CheckError(R)` | `vR = check.error` | vR |
| `WriteResult(R)` | `write.result vR` | **— (use)** |
| `CallBuiltin{result,func,args,immediates,strings}` | see below | result |

**`CallBuiltin` form:**

```
vR = call @<func>(<arg0>, <arg1>, ...)[ imm[<i0>, <i1>, ...]][ str["<s0>", "<s1>", ...]]
```

- `@<func>` is the stored `func: &'static str` verbatim.
- The `(...)` arg list is **always** printed (empty as `()`); args are canonical
  register names in order.
- `imm[...]` is printed **only when** `immediates` is non-empty; values are decimal
  `usize` in order.
- `str[...]` is printed **only when** `strings` is non-empty; each is a quoted,
  escaped string literal (§7.1) in order.
- Omitting empty `imm`/`str` groups (rather than printing `imm[] str[]`) is required
  to minimize noise.

### 6.1 Def/use classification (overrides `result_reg()`)

The serializer's register-numbering pass uses **this** table, not
`IrOp::result_reg()` (`ir.rs:124`). `result_reg()` reports `WriteResult` as
*defining* its register, but semantically it **consumes** it (writes the value
to `ctx.result`). For snapshots it is a **use, not def** — it introduces no new
`vN`. `StoreLocal` likewise defines no register. This discrepancy with
`result_reg()` is a known IR inconsistency (track it; do not "fix" it by trusting
`result_reg` here).

### 6.2 Excluded information (and why)

- **`fn_index`** — a codegen-assigned table index that shifts with unrelated edits.
  Not part of this function's IR shape. Excluded.
- **Raw `Reg` / `BlockId` ids** — replaced by canonical `vN` / `bbN`.
- **Stored `predecessors`** — recomputed from the CFG (§2.6).
- **All codegen / runtime state** — CLIF values, `regs`, `reg_slot`, spill slots,
  `STACK_CAP`, operand-stack `sp`/`capacity`, `JitContext`, FFI function-pointer
  table. The snapshot is pre-codegen IR.
- **Source spans / line numbers** — not IR semantics; high churn. Excluded.
- **Comments and debug annotations.**

---

## 7. Diff-stability & formatting rules

### 7.1 Strings

Printed as a double-quoted literal. Escape, in this order of precedence:
`\` → `\\`, `"` → `\"`, U+000A → `\n`, U+000D → `\r`, U+0009 → `\t`,
U+0000 → `\0`. Any other control character (`< U+0020` or `U+007F`) → `\u{<hex>}`
with lowercase, minimal-digit hex. All other characters (including printable
non-ASCII) are emitted as UTF-8 verbatim. This is platform-independent.

### 7.2 Chars

Same escape set as §7.1, single-quoted, plus `'` → `\'`.

### 7.3 Floats

Goal: stable across platforms and visually distinct from ints.

- Finite: shortest round-trip decimal (Ryū-style, as Rust's `f64` `Display`
  produces). If the result contains no `.`, `e`, or `E`, append `.0` (so `2` prints
  `2.0`).
- `-0.0` prints `-0.0` (sign preserved).
- `+∞` → `inf`; `-∞` → `-inf`; NaN → `NaN` (a single canonical NaN spelling
  regardless of payload/sign).

### 7.4 Types

Rendered via the type checker's single-source-of-truth name mapping (the inverse of
`TypeInfo::from_name`). Required canonical spellings: `int`, `byte`, `float`,
`bool`, `char`, `String`; unit type → `()`. Composite types use their canonical
generic spelling (e.g. `Vec<int>`, `Option<String>`, `Result<int, String>`).
`TypeInfo::Unknown` → `?` and is a smell in post-typecheck IR — keep it visible.

### 7.5 Whitespace & layout

- Indentation is spaces only: function body keys (`locals:`, `captures:`) and block
  labels at 2 spaces; instructions and terminators at 4 spaces; closing `}` at
  column 0.
- Exactly one space around `=`, after commas, and around `->`. No double spaces.
- No trailing whitespace on any line. Unix `\n` line endings only. Exactly one
  blank line between functions and exactly one trailing newline at EOF.

### 7.6 General determinism

No HashMap/HashSet iteration in output, no pointers/addresses, no timestamps, no
absolute paths, no environment-dependent data, no RNG. Given an `IrFunction`, the
output is a pure function of its fields (minus the §6.2 exclusions).

---

## 8. Forbidden representations

A conforming snapshot MUST NOT contain any of the following. Their presence
indicates either a serializer bug or codegen leaking into the IR — surface it
(panic in the serializer or emit a visible `<malformed: …>` marker), never print it
silently.

1. **Operand-stack artifacts** — no `push`, `pop`, `sp`, "top of stack", or stack
   depth. The register IR has no operand stack (`IR_DESIGN.md` §3, §12); that lives
   only in codegen/runtime.
2. **Codegen-synthesized FFI helpers in `@func`** — the following are emitted by
   `codegen.rs`, never by `ir_gen`, and must **not** appear as a `CallBuiltin`
   `func` in valid IR: `oxy_push_int`, `oxy_push_bool`, `oxy_push_float`,
   `oxy_push_char`, `oxy_push_string`, `oxy_push_unit`, `oxy_load_local`,
   `oxy_load_local_raw`, `oxy_store_local`, `oxy_read_local_i64`,
   `oxy_set_result_i64`, `oxy_return`, `oxy_error_discriminant`, `oxy_panic`.
   (Semantic IR-level calls such as `oxy_call`, `oxy_call_closure`,
   `oxy_method_call`, `oxy_make_enum_variant`, `oxy_struct_init`,
   `oxy_struct_update`, `oxy_make_iter`, `oxy_iter_next`, `oxy_make_array`,
   `oxy_push_named_fn`, `oxy_spawn_ffi`/`oxy_sleep_ffi`/`oxy_select_ffi` are
   legitimate and printed normally.) Maintain this forbidden set as a named constant
   so it cannot drift.
3. **Implicit / anonymous temporaries** — every produced value has an explicit `vN`
   name. No positional, unnamed, or "result of previous op" references.
4. **Codegen residence info** — no CLIF values, no `regs`/`reg_slot` distinction, no
   spill-slot indices, no `STACK_CAP`. A register is a single abstract `vN`.
5. **Raw `{:?}` debug formatting** of any value, Vec, or struct (the cause of
   instability in the existing `dump()`).
6. **Raw allocator ids** — printed `Reg`/`BlockId` integers from `alloc_reg`/
   `add_block`. Always renumber.
7. **Nondeterministic ordering** — anything sourced from hash iteration, including
   the stored `predecessors` field.

---

## 9. Worked examples

### 9.1 Straight-line

```
fn add_pair(a: int, b: int) -> int {
  locals: 2
  bb0(entry):
    v0 = load.local $0
    v1 = load.local $1
    v2 = add v0, v1
    ret v2
}
```

### 9.2 Branch + phi

```
fn max(a: int, b: int) -> int {
  locals: 2
  bb0(entry):
    v0 = load.local $0
    v1 = load.local $1
    v2 = gt v0, v1
    branch v2 -> bb1, bb2
  bb1(preds: bb0):
    v3 = load.local $0
    jump bb3
  bb2(preds: bb0):
    v4 = load.local $1
    jump bb3
  bb3(preds: bb1, bb2):
    v5 = phi v3, v4
    ret v5
}
```

### 9.3 Call with immediates and strings

```
fn greet() -> () {
  locals: 1
  bb0(entry):
    v0 = const.str "world"
    v1 = load.local $0
    v2 = call @oxy_method_call(v1, v0) imm[1] str["push"]
    ret v2
}
```

---

## 10. Implementation notes (for the engineer)

The serializer is a pure function `serialize(&IrFunction) -> String` (and a
program-level wrapper that sorts per §2.1 and joins with blank lines). Suggested
pipeline, all deterministic:

1. Compute `canonical_block_order` (§2.2) and the recomputed predecessor map (§2.6).
2. Two-pass register naming (§3) using the §6.1 def/use table.
3. Emit header (§5), then blocks in canonical order, each op via the §6 table and
   the §7 formatting rules.
4. Assert the §8 forbidden set is absent (debug assertion / visible marker).

Keep the canonical serializer entirely separate from `IrFunction::dump()` /
`Display` in `ir.rs`; those remain the non-canonical trace path. When the IR changes
(new `IrOp`, new terminator, changed phi/return shape), update §6 and the relevant
sections here in the same commit.
