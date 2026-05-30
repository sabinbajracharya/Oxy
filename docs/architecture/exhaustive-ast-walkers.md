# Exhaustive matches as a correctness guard

**Status:** Active (principle). **Origin:** 2026-05-24, generalized after the
register-IR migration.

## The principle

When code walks a growing enum (`Expr`, `Stmt`, `Pattern`, `IrOp`, `Terminator`), a
`match … { _ => {} }` wildcard **silently drops** every variant nobody wrote an arm
for. Adding a new variant then produces no compile error, no warning — just wrong
behavior that surfaces only when source exercises the gap.

The fix is to **omit the wildcard** and let the Rust compiler refuse to build until
every variant has an explicit arm. The class of bug — "a walker silently skipped a
variant" — is closed at the type level, with no runtime cost and no extra
infrastructure (we evaluated a full `Visitor` trait; the AST isn't large enough to
justify it).

Contributors may still choose "drop it" for a new variant — but they must type the
arm, see the others, and own the decision in the diff.

## Where this lives today

| Site | What it walks | Why exhaustive matters |
|---|---|---|
| `vm/interp.rs` — the `IrOp` / `Terminator` match | the register IR | **The headline guard.** Compiled on *all* targets with no wildcard, so adding/removing an IR op breaks every native build until the interpreter handles it. This is what keeps the two execution backends from diverging (see [`../execution-model.md`](../execution-model.md)). |
| `vm/jit/ir_gen/mod.rs` — `collect_free_vars` | `Expr` / `Stmt` | Closure mutable-capture analysis: a free variable hidden inside `for` / `while let` / `match` / `StructInit` bodies must not be dropped, or the wrong capture path is emitted. |
| Type-checker / lowering walkers over `Expr`/`Stmt`/`Pattern` | the AST | Generic substitution and resolution must reach every sub-expression. |

## A concrete example of what the wildcard hid (historical)

In the (now-removed) bytecode `compiler/`, the closure free-variable walker handled
only ~6 of ~30 `Expr` variants behind a `_ => {}`. A closure tucked inside a `for`
body escaped the mutable-capture pre-scan:

```oxy
let mut count = 0;
for n in nums {
    let f = || { count = count + 1; };  // inside Stmt::For — silently skipped
    f();
}
```

`count`'s mutation wasn't classified as a mutable capture, so the compiler emitted an
immutable-capture path — working "by accident" sometimes and failing others. Making
the match exhaustive closed the whole family of "X works alone but not when wrapped in
Y" bugs. The same logic now lives in `ir_gen`'s `collect_free_vars`.

## When a wildcard is legitimately fine

A `_` arm is correct when the default answer is *semantically* "none of the rest
matter" and a new variant can't make that wrong — e.g. a constant-folding helper whose
default is "not a foldable constant," or a literal-only inspector. The test: *if a new
variant fell into `_`, would the result be wrong?* If yes, list the variants. If the
default is genuinely correct for anything new, the wildcard documents intent.

## Related

- [`../execution-model.md`](../execution-model.md) — the no-wildcard `IrOp` match as a
  backend-divergence guard, plus the FFI-consistency and `unsupported_on_wasm!` guards.
- [`../history/pattern-stack-contract.md`](../history/pattern-stack-contract.md),
  [`../history/vm-locals-stack-separation.md`](../history/vm-locals-stack-separation.md)
  — retired bytecode-era instances of the same "push the invariant into the structure"
  idea.
