# Pattern compilation: unified stack contract

**Status:** Active.
**Date:** 2026-05-24.
**Scope:** `crates/oxy-core/src/compiler/expr.rs` — `compile_pattern`,
`bind_pattern_data`, and the three callers (`Expr::Match`,
`Expr::IfLet`, `Expr::WhileLet`).

## Summary

Pattern compilation used to leave the operand stack in two different
shapes depending on which pattern variant was compiled. The match-arm
dispatcher tracked which shape it got via a `consumes_scrutinee` flag,
emitted a prelude `Pop` between arms to drain leftover scrutinees, and
needed a sentinel `ConstUnit` before guards to keep the prelude from
underflowing into the caller's frame.

The contract is now uniform:

```text
compile_pattern    :  [scrutinee] -> [bool]     (scrutinee always consumed)
bind_pattern_data  :  [value]     -> []         (value always consumed)
```

This eliminated the dispatcher's bookkeeping entirely and removed the
class of stack-discipline bugs that bit us four times in one
development session.

## Why we had two shapes

The old `compile_pattern` had grown variant-by-variant. Each pattern
emitted whatever opcodes were locally convenient:

| Pattern         | Old output shape       | Why                                          |
| --------------- | ---------------------- | -------------------------------------------- |
| `Wildcard`      | `[scrut, true]`        | just `ConstBool(true)` — scrutinee passes through |
| `Ident`         | `[scrut, scrut, true]` | `ConstBool(true); Dup` — the dup was the binding value |
| `Literal`       | `[scrut, bool]`        | `Dup; <lit>; Eq` — kept scrut for downstream |
| `EnumVariant`   | `[bool]`               | `EnumVariantEqual` semantics consume the scrutinee |
| `Tuple`         | `[bool]`               | stashed scrut in a temp to index its elements |
| `Struct`        | `[bool]`               | `Pop` + `true` — type checker guarantees the match |
| `Range (s, e)`  | `[scrut, bool]`        | a `bool_tmp` re-pushed shape after the test  |
| `Or`            | `[scrut, bool]`        | an `alt_tmp` dance restored shape after recursion |

After `JumpIfFalse` consumed the bool, the stack was either `[scrut]`
or `[]` depending on the variant. The dispatcher had to know which.

## The bookkeeping we used to need

The old match-arm dispatcher carried three coordinated pieces of
state to compensate for the shape variance:

1. **`prev_consumed_scrutinee`** — set during the previous iteration
   so the next iteration's prelude could decide whether to `Pop`.
2. **`consumes_scrutinee`** — a fresh per-arm flag that ran the same
   `matches!(arm.pattern, EnumVariant | Tuple | Struct)` check twice:
   once to update `prev_consumed_scrutinee`, once to decide whether
   to `LoadLocal(scrut_slot)` or `Pop` before `bind_pattern_data`.
3. **`ConstUnit` sentinel before guards** — guards were originally
   broken because the guard-fail jump went to the next arm's prelude,
   which `Pop`ped a value that didn't exist (underflow into the
   caller's frame). The fix pushed a dummy unit before the guard so
   the prelude `Pop` consumed *that* on guard-fail and the
   post-guard cleanup `Pop`ped it on guard-success.

Each of these was added as a fix for a specific underflow bug. They
all worked, but the design centre — "different patterns produce
different stack shapes" — kept generating new instances of the same
class of bug.

## Bugs the old design produced

Same class, four different symptoms, all in this session:

| Symptom                                                                                          | Root cause                                                                  |
| ------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------- |
| `match describe(n: int)` with a guard underflowed and corrupted the caller's `FStringConcat`     | Guard-fail jumped to a prelude `Pop` that dipped into the caller frame      |
| Adding `Pattern::Struct` worked in isolation but broke whichever arm followed it                 | `Struct` consumes the scrut; the `consumes_scrutinee` whitelist didn't know |
| `EnumVariantEqual` consumed the scrutinee but `bind_pattern_data` needed it back                 | Required a `LoadLocal(scrut_slot)` branch only for "consuming" patterns     |
| `Or([Literal, Literal])` worked at the top level but broke inside a nested context               | `Or` had to manually restore `[scrut, bool]` shape via `alt_tmp`            |

Each fix touched a different file or function. None of them addressed
the design issue: the contract itself was non-uniform.

## The new contract

```text
                    INPUT          OUTPUT
compile_pattern :   [scrutinee] -> [bool]
bind_pattern_data : [value]     -> []
```

The match-arm dispatcher reduces to a straight pipeline:

```rust
for arm in arms {
    self.emit(OpCode::LoadLocal(scrutinee_slot));   // [scrut]
    self.compile_pattern(&arm.pattern, ...)?;       // [bool]
    let jump_to_next = self.emit(OpCode::JumpIfFalse(0));
                                                    // []  on fail
                                                    // []  on pass
    self.emit(OpCode::LoadLocal(scrutinee_slot));   // [scrut] for binding
    self.bind_pattern_data(&arm.pattern)?;          // []

    let guard_fail_jump = if let Some(guard) = &arm.guard {
        self.compile_expr(guard)?;                  // [bool]
        Some(self.emit(OpCode::JumpIfFalse(0)))     // []
    } else {
        None
    };

    self.compile_expr(&arm.body)?;                  // [result]
    arm_jumps.push(self.emit(OpCode::Jump(0)));

    // Pattern-fail and guard-fail both land at the next arm with [].
    let next_arm = self.code.len();
    self.patch(jump_to_next, OpCode::JumpIfFalse(next_arm));
    if let Some(gj) = guard_fail_jump {
        self.patch(gj, OpCode::JumpIfFalse(next_arm));
    }
}
```

No `prev_consumed_scrutinee`. No `consumes_scrutinee`. No prelude
`Pop`. No `ConstUnit` sentinel. `if-let` and `while-let` collapse to
the same shape.

## What each variant changed

| Pattern         | Before                              | After                                                                      |
| --------------- | ----------------------------------- | -------------------------------------------------------------------------- |
| `Wildcard`      | `ConstBool(true)`                   | `Pop; ConstBool(true)`                                                     |
| `Ident`         | `ConstBool(true); Dup`              | `Pop; ConstBool(true)` (binding now reloads in `bind_pattern_data`)         |
| `Literal`       | `Dup; <lit>; Eq`                    | `<lit>; Eq`                                                                |
| `EnumVariant`   | (unchanged — already consumed)      | (unchanged)                                                                |
| `Range (s,e)`   | scrut_tmp + bool_tmp swap to rebuild `[scrut, bool]` | scrut_tmp only; emit `Ge`/`Le`/`And` to leave `[bool]`              |
| `Range (s,_)`   | `Dup; <s>; Ge`                      | `<s>; Ge`                                                                  |
| `Range (_,e)`   | `Dup; <e>; Le/Lt`                   | `<e>; Le/Lt`                                                               |
| `Tuple`         | (unchanged — already consumed)      | (unchanged)                                                                |
| `Or`            | scrut_tmp + alt_tmp dance to restore shape after each alt | scrut_tmp only; `LoadLocal; compile_pattern; Or` loop          |
| `Struct`        | (unchanged — already consumed)      | (unchanged)                                                                |

`bind_pattern_data` got Pops on the no-binding cases (`Wildcard`,
`Literal`, the `_` arm) so the value always gets consumed.

## Why this works — and why "make it uniform" wasn't obvious in hindsight

The temptation when each variant emits "locally convenient" opcodes
is to leave whatever's already on the stack. `Wildcard` doesn't need
the scrutinee, but it's already there, so why not leave it. `Range`'s
`Ge` test needs the scrutinee on top, so `Dup; <s>; Ge` was the
shortest path. Each individual decision was defensible.

The hidden cost was that the *dispatcher* — the part that doesn't
know which variant it's compiling — had to handle every possible
output shape. That dispatcher then leaked complexity outward (`bind_pattern_data`
also had to know which patterns consumed and which didn't; the
caller had to either `Pop` or `LoadLocal`).

The fix isn't to make any individual variant smarter. It's to push
the burden of consuming the scrutinee onto every variant, so the
dispatcher doesn't have to care. Six of the eight variants pay one
extra `Pop` to follow the contract. The dispatcher loses three
pieces of state. The class of bug disappears.

## Benefits

- **Class of bugs eliminated.** All four bugs above had the same
  root cause; none of them can recur. New patterns added to the
  language can't accidentally introduce a fifth instance.

- **−90 net lines.** Mostly in `compiler/expr.rs`. The
  dispatcher dropped from ~95 lines to ~50; `Range` and `Or` got
  shorter; the cumulative comment burden explaining "why this Pop
  is here" went away.

- **Adding a new pattern is now mechanical.** Any future
  `Pattern::Foo` only needs to satisfy `[scrut] → [bool]`. The
  match/if-let/while-let dispatchers don't need updating, and
  `bind_pattern_data` only needs an entry if the pattern introduces
  bindings.

- **`if-let` and `while-let` collapse to the same shape as `match`.**
  Previously each carried its own copy of the consumes-or-not
  branching. Now all three follow the same pipeline.

- **The class is closed at the type level**: there's exactly one
  contract, documented in `compile_pattern`'s doc comment and the
  `vm/mod.rs` module header. Future contributors don't need to read
  the implementation to know the invariant.

## Trade-offs and caveats

- **Wildcards and Idents pay one extra opcode.** Old: just
  `ConstBool(true)`. New: `Pop; ConstBool(true)`. The `Pop` is
  trivial, but it does exist. In tight `match x { _ => ... }`
  benchmarks this is a one-opcode regression. We have no data
  suggesting it matters.

- **`Range` with both bounds saves a `StoreLocal`/`LoadLocal` pair**
  (the `bool_tmp` is gone), so net opcode count there is *down*.

- **`Or`-with-bindings still isn't supported** (it was never
  supported before either — `bind_pattern_data`'s catch-all `_`
  arm didn't bind, and `Pattern::Or` inside `bind_pattern_data`
  still falls to the no-binding `Pop` arm). The refactor neither
  fixes nor breaks this. A follow-up could plumb bindings through.

## Related

- `vm-locals-stack-separation.md` — the locals/operand split that
  made this kind of stack-contract reasoning possible at all.
- `closure-capture-dense-renumbering.md` — another instance of
  pushing invariants into the data model so callers don't have to
  track them.
