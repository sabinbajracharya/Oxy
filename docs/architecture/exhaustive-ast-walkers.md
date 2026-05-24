# Exhaustive AST walkers in the compiler

**Status:** Active.
**Date:** 2026-05-24.
**Scope:** `crates/oxy-core/src/compiler/helpers.rs` — the closure-capture
analysis and generic-monomorphization walkers.

## Summary

Four walkers in `compiler/helpers.rs` used `match … { _ => {} }` to silently
drop every `Expr` or `Stmt` variant they hadn't been written for. Every time
a new variant was added to the AST, those walkers silently kept skipping it
— there was no compile error, no runtime warning, just incorrect behaviour
that only surfaced when someone wrote source code that exercised the gap.

The walkers are now exhaustive: every variant has an explicit arm. The
Rust compiler refuses to build whenever a new variant is added to `Expr`,
`Stmt`, or `Pattern` until each walker is given a recursion rule for it.
The class of bug — "a walker silently dropped a variant" — is closed at
the type level.

## The walkers

| Walker                                       | Used for                                                                           |
| -------------------------------------------- | ---------------------------------------------------------------------------------- |
| `substitute_type_params` (Expr + Stmt)       | Generic monomorphization — replace `T` with `int` etc. when specializing a generic |
| `collect_closure_free_vars` (Expr + Stmt)    | Mutable-capture pre-scan — find every closure body and record what it captures      |
| `collect_free_vars_in_stmt`                  | Per-statement free-variable analysis used by `collect_free_vars`                    |
| `pattern_bindings`                           | Record names a pattern introduces (so they don't get treated as free in the arm)    |

## What the silent wildcards hid

### Bug 1 — generic functions with closures weren't monomorphized

`substitute_type_params` mutates a generic function body by substituting
the type parameter into `PathCall`s, `Call`s, etc. Every variant the
walker didn't list got dropped:

```rust
match expr {
    Expr::PathCall { .. } => { /* substitute */ }
    Expr::Call { .. }     => { /* recurse */ }
    Expr::MethodCall { .. } => { /* recurse */ }
    // ... 13 more arms ...
    _ => {}  // ← Closure, Tuple, Range, IfLet, As, Try, Await, FString, Return all land here
}
```

So a generic function like:

```oxy
fn make_box<T>(v: T) -> Box<T> {
    let f = || { Box::<T>::new(v) };   // ← T inside closure
    f()
}
```

…silently kept the literal `T` token inside the closure body, because
the `Closure` arm fell into `_ => {}`. The monomorphized version had a
`T::new` call that doesn't exist.

The embedded `Stmt` walker inside the `Block` arm had the same shape:

```rust
match stmt {
    Stmt::Expr { .. } => ...
    Stmt::Let { .. } => ...
    Stmt::Return { .. } => ...
    _ => {}    // ← For, While, Loop, WhileLet, ForDestructure all silently skipped
}
```

So a `for` loop inside a generic function body never got its body
substituted either.

### Bug 2 — closures inside common AST nodes escaped the mutable-capture pre-scan

`collect_closure_free_vars` traverses a block and records every
variable each closure captures from outside. That set is used to
decide which captured locals must live in shared mutable storage.
The old walker handled only 6 of ~30 `Expr` variants:

```rust
match expr {
    Expr::Closure { .. }    => ...
    Expr::BinaryOp { .. }   => ...
    Expr::Call { .. }       => ...
    Expr::MethodCall { .. } => ...
    Expr::UnaryOp { .. }    => ...
    Expr::Assign { .. }     => ...
    _ => {}                  // ← If, Match, For-bodies, Tuples, StructInit, Block, ...
}
```

Concretely, this meant a closure tucked inside any of these constructs
escaped the pre-scan:

```oxy
let mut count = 0;
for n in nums {
    let f = || { count = count + 1; };   // ← inside Stmt::For — silently skipped
    f();
}
```

The closure's mutation of `count` wouldn't be classified as a mutable
capture. The compiler then emitted an immutable-capture path; sometimes
this worked by accident, sometimes it didn't.

### Bug 3 — free variables in several statement forms were lost

`collect_free_vars_in_stmt` had:

```rust
match stmt {
    Stmt::Expr { .. }  => ...
    Stmt::Let { .. }   => ...
    Stmt::While { .. } => ...
    Stmt::Loop { .. }  => ...
    _ => {}    // ← For, Return, WhileLet, ForDestructure, LetPattern, Break(value)
}
```

So a free variable referenced *only* via `for i in xs { … }` (the `xs`),
`return v;` (the `v`), `break v`, or `while let Some(x) = e { … }` (the
`e`) silently dropped out of analysis. The closure-capture pre-scan
ran on top of this, so the same source-code holes turned into "closure
needs a capture that the compiler didn't know about" downstream.

### Why the bugs hadn't been louder

Most of these bugs were latent. The compiler had several layers of
analysis; a missed capture often got picked up by a later pass that
re-resolved identifiers from scope, masking the gap. But the masking
was incomplete — once we added enough features
(`for-destructuring`, `while let`, closures-in-`match`-arms) the gaps
started showing up as "X works alone but not when wrapped in Y" bugs
that took a long time to localise because they didn't point at the
walker.

## The fix

Every match is now exhaustive. The wildcard `_ => {}` has been
replaced with a terminal arm that *lists* the leaves explicitly:

```rust
// Terminals — no subexpressions to recurse into.
Expr::Ident(..)
| Expr::IntLiteral(..)
| Expr::FloatLiteral(..)
| Expr::BoolLiteral(..)
| Expr::StringLiteral(..)
| Expr::CharLiteral(..)
| Expr::Path { .. }
| Expr::SelfRef(_) => {}
```

When someone adds a new `Expr::WhateverNext` variant to the AST, **every
walker will fail to compile** until they decide what its recursion rule
should be. They might still pick "drop it" — but they have to type the
arm, look at the others, and own the decision.

### Walker-by-walker outcome

| Walker                              | Before               | After                                | Recursion changes                                                                  |
| ----------------------------------- | -------------------- | ------------------------------------ | ---------------------------------------------------------------------------------- |
| `substitute_type_params` (Expr)     | 16 arms + `_ => {}`  | 30 explicit + terminal-leaves arm    | Closure body now substituted; Tuple/Range/IfLet/As/Try/Await/FString/Return covered |
| `substitute_type_params` (Stmt)     | 3 arms + `_ => {}`   | 13 arms (own fn, no wildcard)        | For/While/Loop/WhileLet/ForDestructure/LetPattern bodies now substituted            |
| `collect_closure_free_vars` (Expr)  | 6 arms + `_ => {}`   | 30 explicit + terminal-leaves arm    | Closures in If/Match/Block/Tuple/StructInit/Range/IfLet/etc. now reached            |
| `collect_closure_free_vars_in_block`/`_in_stmt` | 4 arms + `_ => {}` | 13 arms (no wildcard)        | Closures inside For/Return/WhileLet/ForDestructure/LetPattern/Break(value) found    |
| `collect_free_vars_in_stmt`         | 4 arms + `_ => {}`   | 13 arms (no wildcard)                | Free vars in For/Return/WhileLet/ForDestructure/LetPattern/Break(value) collected; loop-var bindings extend scope so they aren't reported free |
| `pattern_bindings`                  | 4 arms + `_ => {}`   | All 9 Pattern variants explicit       | No behaviour change — was already correct — but now compiler-checked                |

## Why "make matches exhaustive" is the right shape of fix

There are three ways to make a walker correct over a growing AST:

1. **Wildcard + tests.** Trust contributors to update walkers when
   they add variants, and trust the test suite to catch what gets
   missed.
2. **A real visitor trait.** Define a `Visitor` trait with one method
   per variant, default implementations that recurse, and have each
   walker override only the variants it cares about.
3. **Exhaustive `match`.** Don't write a wildcard; let the compiler
   refuse to build until every variant has an arm.

Option 1 is what we had. Tests caught some gaps; many didn't fire.

Option 2 is what a "proper" architecture would look like, but it
requires either inverting control (the walker calls `visitor.visit_*`),
which involves more plumbing for our `&mut` cases, or adopting a
crate like `syn::visit`. The Oxy AST isn't large enough to justify
that infrastructure yet.

Option 3 gets ~85% of option 2's safety for zero infrastructure. The
cost is a few extra lines (terminal arms now list literal variants
instead of `_`); the benefit is that the Rust compiler does the
checking forever, with no runtime overhead and no extra dependencies.

We picked option 3.

## What this didn't change

- `try_eval_const` still has `_ => None`. That wildcard is
  *semantically* meaningful: "not a foldable constant expression."
  An exhaustive list would be 25 arms all returning `None`. The cost
  of making it exhaustive outweighs the benefit — adding a new
  variant doesn't break constant-folding correctness because the
  default answer is already "no, not foldable."

- `check_literal_fits_type` still has `_ => {}`. Same reasoning: it
  only inspects literal expressions; non-literals fall through by
  design.

These two are the only walkers in `helpers.rs` whose wildcard is
load-bearing rather than an oversight.

## Benefits

- **Class of bugs closed at compile time.** "Walker silently skipped
  a variant" can no longer ship. The next time someone adds a new
  variant to `Expr`/`Stmt`/`Pattern`, they will see compile errors
  pointing at every walker that needs an explicit decision.

- **The decision is visible in diff form.** A PR that adds a variant
  *must* show its recursion rule for each walker. Reviewers can see
  whether "drop it" is correct or whether the new variant should
  recurse.

- **No runtime cost.** The exhaustive match compiles to the same code
  the wildcard would have, modulo the explicit arms we added for
  previously-dropped variants. Those arms exist because the previous
  behaviour was incorrect.

- **Generic monomorphization now reaches closures and loops.** This
  was the single biggest bug fix in the refactor — generic functions
  with closure-using bodies should now work in cases where they
  silently produced `T`-leaked code before.

- **Mutable-capture pre-scan now sees closures everywhere they can
  appear.** Closures inside `if`, `match`, `for`, `while let`,
  `StructInit`, etc. are now reached. This eliminates a category of
  "closure mutates a captured local, but the compiler emitted the
  immutable-capture path" bugs.

## Trade-offs

- **Adding a new Expr variant becomes a slightly bigger PR.** You
  have to update ~5 walker arms instead of 0. This is the entire
  point — but it does mean variant additions touch more files.

- **Terminal arms list every literal type by name.** When we add a
  new literal type (say `Expr::ByteString`), the terminal arm has
  to be updated. The compiler error will point you at it directly.

- **`+130 net lines` in `helpers.rs`.** Most of those lines are
  arms that previously hid behind `_ => {}` and had a real recursion
  rule waiting to be written. The lines were always going to be
  needed; we just hadn't been forced to write them.

## Related

- `pattern-stack-contract.md` — the same general principle (force the
  invariant into the type system / control flow so individual call
  sites don't have to remember) applied to the match-arm compilation
  protocol.
- `vm-locals-stack-separation.md` — another instance of pushing
  invariants into the structure so they're enforced by construction.
