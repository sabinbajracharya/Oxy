# War Stories: From 129 Failures to Zero

We had a JIT. It compiled. It ran. It was wrong in a hundred and twenty-nine ways.

That's the number that came back the first time the whole feature suite ran against the freshly
merged Cranelift backend: `129 FAILED`. Not one bug to chase down, not a dozen rough edges — a
hundred and twenty-nine red lines scrolling past, touching booleans, collections, closures,
generics, async, pattern matching, very nearly everything the language could do. There's a
particular feeling to that moment, and if you build compilers you'll meet it eventually: the thing
*works*, in the sense that it accepts a program and produces an answer, and the answer is simply
wrong, over and over, in ways that seem to have nothing to do with each other. It's easy to read
129 as "129 separate problems" and feel the floor drop out.

The instinct in that moment is whack-a-mole: open the first failing test, see that `true` printed
as `1`, patch that one case, rerun, move to the next. Resist it. That instinct is the single
slowest way to fix a compiler, and it would have meant 129 separate edits, each one a little
special-case band-aid, the whole codebase slowly turning into scar tissue. Because here is the
thing about a compiler test suite: it is not a list of bugs. It is a list of *symptoms*. The actual
bugs are far fewer, and each one tends to be a single convention that two parts of the compiler
disagree about — and that one disagreement shows up as twenty, thirty, thirty-five red lines at
once. Find the disagreement, fix it in one place, and watch an entire cluster of tests turn green
together. That is the only sane way through 129 failures, and it's the discipline this chapter is
really about.

So the work became archaeology, not whack-a-mole. Group the failures by what they had in common.
*Everything boolean is broken* — that's one cluster, and it points at one place where `true`
acquired the wrong tag. *Every collection constructor returns the wrong thing* — another cluster,
another single mismatch, this time between how a name was registered and how it was looked up.
*Every generic method can't be found* — a third. Each cluster had one root cause and one honest
fix, and the satisfaction of watching `129 → 94` after a single five-line change — thirty-five
tests cleared at once — is hard to overstate. It's the closest compiler work gets to a magic trick.

What follows is the real sequence, reconstructed from `docs/history/ir-cfg-jit-fix-log.md`, the log
we kept in real time as the number came down. It includes the hours we wasted on a wrong hypothesis,
because that was real too. Read it as a worked example of the only debugging method that scales:
treat the failures as symptoms, hunt the convention mismatch underneath, fix the cause, and let the
symptoms clear themselves.

## The document behind this chapter

Everything in this chapter comes from `docs/history/ir-cfg-jit-fix-log.md` — a debugging
log maintained in real time during the JIT stabilization work. Read it alongside this chapter.

The baseline from commit `cfb4e9a` (the JIT merge):

```
feature_examples integration test: 129 FAILED
```

What follows is how that number became 0.

---

## Cluster 1: Bool/Unit value tagging (129 → 94 failures)

**Symptom:**
```
test booleans::test_true_false ... FAILED
left: Bool(true), right: I64(1)
```

`true` was printing as `1`. `false` was printing as `0`. Comparisons that should produce
`Bool(true)` were producing `I64(1)`. Every test that touched boolean values was failing.

**Root cause investigation:**

In `codegen.rs`, the `compile_op` function handles `IrOp::ConstBool`. The original code:

```rust
IrOp::ConstBool(r, b) => {
    let v = builder.ins().iconst(types::I64, if *b { 1 } else { 0 });
    regs.insert(*r, v);  // ← THE BUG
}
```

This stored a raw `i64(1)` into `regs[r]`. When this register was later read for printing,
the codegen called `push_reg` which unconditionally called `oxy_push_int` — boxing the raw
integer as `Value::I64(1)` instead of `Value::Bool(true)`.

**The mirror case that worked:**

`ConstFloat` and `ConstString` were already correct — they called `oxy_push_float` /
`oxy_push_string` (FFI functions that produce properly-tagged `Value::F64` and `Value::String`
respectively). `ConstBool` had never gotten this treatment.

**Fix:**
```rust
IrOp::ConstBool(r, b) => {
    let b_val = builder.ins().iconst(types::I8, if *b { 1 } else { 0 });
    builder.ins().call(ffi_refs["oxy_push_bool"], &[ctx, b_val]);
    spill_result(&mut builder, ctx, &ffi_refs, *r, &mut reg_slot, &mut next_spill_slot);
}
```

Push through `oxy_push_bool` (which creates `Value::Bool`), spill into a local slot.
Same pattern as float and string.

The same bug affected the **inline comparison fast path** (`Eq`, `Neq`, `Lt`, etc. when
both operands were in `regs`). Each comparison result was stored as a raw `i64(1)/i64(0)`.
The fix: push comparison results through `oxy_push_bool` as well.

**Result:** 129 → 94 failures. 35 tests cleared.

---

## Cluster 2: stdlib path canonicalization (94 → 76 failures)

**Wrong hypothesis (3 hours wasted):**

The initial hypothesis for the collection failures was "cross-function buffer corruption" —
that one function's call was overwriting another function's local variables. This was
a documented pattern in the old stack VM (`docs/history/vm-locals-stack-separation.md`),
and the symptoms looked similar.

Spent hours verifying the `invoke_jit_fn` implementation, checking that each call allocated
from the callee's `local_count`... and found it was already correct. The fix had already
been made in a previous iteration.

**Actual root cause:**

Running `HashMap::new()` in isolation:
```bash
cargo run --bin oxy -- test examples/features/collections/hashmap.ox
# Output: hashmap::test_new ... FAILED: expected HashMap, got Unit
```

`HashMap::new()` was returning `Value::Unit`. Not crashing — returning the wrong value silently.

Traced through `oxy_path_call_builtin`: the function looked up `["HashMap", "new"]` in the
registry. The registry had `"HashMap::new"` registered. The path `use std::collections::HashMap`
rewrote the call to `["std::collections::HashMap", "new"]`. The lookup did an exact match
on the segment array — `["HashMap", "new"]` ≠ `["std::collections::HashMap", "new"]` → miss.

The fallback on a miss returned `Value::Unit`.

And the telltale sign: `Regex::new` was registered *twice* in the registry — once as
`["Regex", "new"]` and once as `["std", "regex", "Regex", "new"]`. Someone had already
hit this bug for Regex and added a workaround. The band-aid confirmed the diagnosis.

**Fix:** Flatten segments on `::` before matching, retry with just the trailing `Type::method` pair.
Remove the Regex double-registration.

**Result:** 94 → 76 failures. 18 more tests cleared.

---

## Cluster 3: generic impl name resolution (76 → ~60 failures)

Methods in `impl<T> Cell<T>` were registered under `"Cell<T>::make"` but looked up at
call time as `"Cell::make"` (the base name, without generics).

The type checker already had `base_type_name()` — a function that strips the generic params
off a type name. The IR gen was not using it when registering method names. Applied
`base_type_name()` to the impl type name during registration. Methods registered correctly.

**Result:** Another 14 failures cleared.

---

## The pattern that emerges

Looking across the clusters:

1. **Bool tagging** — one code path (ConstBool) used the wrong convention
2. **Path canonicalization** — the lookup key didn't match the registration key
3. **Generic name stripping** — registration and lookup agreed on the base name in one place but not another

Every bug was a **convention mismatch**: the code that emitted something used one name/format,
and the code that consumed it expected a different name/format. The convention was never
written down; it only became visible when the test failed.

The lesson from the debugging sessions:
> *"Do not debug a compiler by fixing individual test cases. Find the convention mismatch.
> Fix the convention. Watch 30 tests clear."*

Individual test fixes would have required 129 changes — one per failure. The cluster
approach required ~10 changes, each fixing one convention mismatch.

---

## Clusters 4-10 (condensed)

After the first three clusters, the remaining failures were:

| Cluster | Root cause | Tests cleared |
|---------|-----------|--------------|
| 4 | Tuple struct constructors not emitting struct-init IR | ~12 |
| 5 | Named functions as first-class values fell through to `ConstUnit` | ~8 |
| 6 | Closures not capturing mutable variables through cells | ~6 |
| 7 | Pattern matching on tuple struct variants accessing wrong fields | ~5 |
| 8 | Async task spawning not initializing child context correctly | ~4 |
| 9 | Operator overloading dispatch using wrong method lookup path | ~3 |
| 10 | Generic function monomorphization passing wrong type argument | ~5 |

Each cluster had a clean root cause. Each fix cleared multiple tests. Each fix was
a `let this_was_wrong = old_code; let this_is_right = new_code` change — not a hack.

Final state: 0 failures. All tests green.

---

## What the experience teaches

Building a JIT compiler is not debugging hundreds of individual edge cases. It is:
1. Run the test suite (get 129 failures)
2. Identify symptoms that cluster together (boolean values print as integers)
3. Find the shared root cause (ConstBool uses wrong tagging convention)
4. Fix the root cause (match the convention used everywhere else)
5. Repeat until zero

The test suite is not a list of bugs. It is a list of *symptoms*. The bugs are the
convention mismatches, the missing cases, the wrong assumptions. One bug manifests as
many failures. One fix clears many failures. This is the insight that makes compiler
debugging tractable.

Full details: `docs/history/ir-cfg-jit-fix-log.md`.
