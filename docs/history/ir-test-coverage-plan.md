# IR Test Coverage Plan — AST → Register IR Lowering

> **Status: RETIRED (plan fulfilled).** Every 🆕 row below was implemented; the 82 golden
> files now live under `crates/oxy-core/tests/snapshots/ir/**` and are asserted by
> `crates/oxy-core/tests/ir_snapshot_tests.rs`. This document is kept for historical
> context — it is **not** current guidance. The live coverage is the test suite itself;
> the IR/serialization sources of truth are `crates/oxy-core/src/vm/jit/IR_DESIGN.md`
> and `IR_SNAPSHOT_FORMAT.md`.

> **(Original plan, as written.)** This document specifies *what* snapshot tests should exist and
> *why*. It does not contain test code. An implementing model/engineer should turn each 🆕
> row into one `gen_ir_snapshot()` golden-file test in `crates/oxy-core/tests/ir_snapshot_tests.rs`.

## Context

The Register IR snapshot infrastructure already exists and is committed:
- `crates/oxy-core/src/vm/jit/ir_snapshot.rs` — canonical serializer (conforms to `IR_SNAPSHOT_FORMAT.md`)
- `gen_ir_snapshot(source: &str) -> Result<String, String>` — public API in `vm/api.rs`
  (parse → type-check → ir_gen, **no codegen**)
- `crates/oxy-core/tests/ir_snapshot_tests.rs` — golden-file harness
  (`UPDATE_SNAPSHOTS=1` regenerates; `line_diff` shows ±2-line context on mismatch)
- `crates/oxy-core/tests/snapshots/ir/<category>/<name>.txt` — 40 existing golden files

IR semantics: `crates/oxy-core/src/vm/jit/IR_DESIGN.md`.
Serialization format: `crates/oxy-core/src/vm/jit/IR_SNAPSHOT_FORMAT.md`.

This plan defines a **comprehensive, non-overlapping** coverage matrix for the AST→IR lowering
step implemented in `jit/ir_gen/mod.rs` (`gen_expr` / `gen_stmt`). Each test = one golden file.
A test "covers" a lowering decision when its snapshot is the unique place that decision is visible.

Legend:
- ✅ = golden file already exists (the listed name is the existing file under `tests/snapshots/ir/`)
- 🆕 = gap to add

### Two verified semantic facts the matrix is built to expose

1. **Boolean `&&` / `||` are EAGER.** They lower to a single `IrOp::And` / `IrOp::Or` register
   op (`ir_gen/mod.rs:776-777`), with **no short-circuit CFG branch** — both operands always
   evaluate. Category 13b locks and surfaces this.
2. **Assignment is statement-based and has type `()`.** Both `Expr::Assign` and
   `Expr::CompoundAssign` type-check to `TypeInfo::Unit` (`type_checker/check_expr.rs:806`
   and `:1550`) — they do **not** yield the assigned value. Consequently **chained
   assignment (`a = b = c`) is invalid**: the inner `b = c` produces `()`, so the outer
   assignment would try to store `()` into `a`. `CompoundAssign` still evaluates the
   **value before the target** (`ir_gen/mod.rs:1207-1208`) — an evaluation-order fact
   independent of the result type. Category 7 locks the unit-result targets and the
   eval-order, not a value-returning chain.

---

## How to implement each test (for the executing model)

1. Add a `#[test] fn` inside the matching `mod` in `tests/ir_snapshot_tests.rs`.
2. Call `assert_ir_snapshot("<category>/<name>", "<oxy source>")` for expression/statement-level
   snippets (the helper wraps the source in `fn main() { ... }`), or
   `assert_ir_snapshot_raw("<category>/<name>", "<full program>")` for tests that define their own
   top-level functions (categories 10, 12, and any test needing helper functions).
3. Generate the golden file:
   `docker compose run --rm dev bash -c "UPDATE_SNAPSHOTS=1 cargo test -p oxy-core --test ir_snapshot_tests"`
4. **Inspect every generated `.txt` by hand** before committing — the golden file is the
   assertion. Pay special attention to the 13b eager-boolean files.
5. Re-run without `UPDATE_SNAPSHOTS` to confirm they pass.

---

## Coverage Matrix (13 categories)

### 1. Literals — one op kind each, no arithmetic noise
Goal: every `Const*` IrOp variant has exactly one minimal snapshot.

| Test | Source idea | Locks |
|---|---|---|
| ✅ `expressions/const_int` | `let x = 5;` | `ConstInt` |
| ✅ `expressions/const_float` | `let x = 1.5;` | `ConstFloat` + float formatting |
| ✅ `expressions/const_bool` | `let x = true;` | `ConstBool` |
| ✅ `expressions/const_str` | `let x = "hi";` | `ConstString` |
| ✅ `expressions/const_unit` | `let x = ();` | `ConstUnit` |
| 🆕 `expressions/const_char` | `let x = 'a';` | `ConstChar` — **op uncovered today** |
| 🆕 `expressions/const_negative_int` | `let x = -5;` | `-5` is `Neg(ConstInt)`, not a negative literal |
| 🆕 `expressions/const_string_escapes` | `let x = "a\n\t\"b";` | §7 escape serialization |

No overlap: literals carry no operators except the deliberate `-` / escape probes.

### 2. Arithmetic expressions — one binary op each
Goal: each arithmetic `IrOp` (`Add/Sub/Mul/Div/Rem`) appears in isolation.

| Test | Source idea | Locks |
|---|---|---|
| ✅ `expressions/arithmetic_add` | `1 + 2` | `Add` |
| ✅ `expressions/arithmetic_sub_div` | `10 - 2`, `10 / 2` | `Sub`, `Div` |
| 🆕 `expressions/arithmetic_mul` | `3 * 4` | `Mul` |
| 🆕 `expressions/arithmetic_rem` | `7 % 3` | `Rem` (note: `BinOp::Mod` → `IrOp::Rem`) |
| 🆕 `expressions/arithmetic_float` | `1.5 + 2.5` | float operands → same IR shape (no lowering divergence) |

No overlap with precedence (single op only) or comparison (numeric-result only).

### 3. Precedence rules — operand register ordering only
Goal: lock the sub-expression-evaluation order / register numbering the parser hands to ir_gen.

| Test | Source idea | Locks |
|---|---|---|
| ✅ `expressions/arithmetic_precedence` | `1 + 2 * 3` | `*` binds tighter than `+` |
| 🆕 `expressions/precedence_paren_override` | `(1 + 2) * 3` | parens reshape the tree vs the above |
| 🆕 `expressions/precedence_left_assoc` | `1 - 2 - 3` | lowers as `(1-2)-3` (left-assoc order) |
| 🆕 `expressions/precedence_mixed_cmp_arith` | `1 + 2 < 3 * 4` | arith binds tighter than comparison |

No overlap: these are the *only* multi-operator-tree tests; categories 2/5 stay single-op.

### 4. Unary operators — one op each + composition
Goal: every unary `IrOp` (`Neg/Not/BitNot`) once, plus stacking.

| Test | Source idea | Locks |
|---|---|---|
| ✅ `expressions/unary_neg` | `-x` | `Neg` |
| ✅ `expressions/unary_not` | `!b` | `Not` |
| 🆕 `expressions/unary_bitnot` | bitwise-not of an int | `BitNot` — **op uncovered today** |
| 🆕 `expressions/unary_double_neg` | `--x` / `!!b` | two ops, distinct result regs (no folding at IR) |

No overlap: category 1's `const_negative_int` is unary-over-*literal*; here the operand is a *variable*.

### 5. Comparison & bitwise — remaining binary ops
Goal: cover the comparison and bitwise `IrOp`s not in category 2.

| Test | Source idea | Locks |
|---|---|---|
| ✅ `expressions/comparison_eq` | `a == b` | `Eq` |
| ✅ `expressions/comparison_lt` | `a < b` | `Lt` |
| ✅ `expressions/comparison_neq` | `a != b` | `Neq` |
| ✅ `expressions/bitwise_and` | `a & b` | `BitAnd` |
| ✅ `expressions/bitwise_or` | `a | b` | `BitOr` |
| 🆕 `expressions/comparison_gt_le_ge` | `a > b`, `a <= b`, `a >= b` | `Gt`, `Le`, `Ge` — **uncovered** |
| 🆕 `expressions/bitwise_xor_shift` | `a ^ b`, `a << 1`, `a >> 1` | `BitXor`, `Shl`, `Shr` — **uncovered** |

No overlap: split from arithmetic by result kind; each op appears in exactly one category.

### 6. Variables (load/store)
Goal: lock `LoadLocal` / `StoreLocal` / slot allocation / shadowing.

| Test | Source idea | Locks |
|---|---|---|
| ✅ `variables/let_simple` | `let x = 1;` | basic store |
| ✅ `variables/let_with_type_ann` | `let x: int = 1;` | typed binding |
| ✅ `variables/let_mut_reassign` | `let mut x = 1; x = 2;` | reassign to same slot |
| ✅ `variables/let_shadow` | two `let x` | shadowing → distinct slots |
| ✅ `variables/let_bool` | `let b = true;` | bool slot |
| ✅ `variables/multiple_locals` | several lets | slot counter |
| 🆕 `variables/let_uninit_then_assign` | `let x; x = 1;` | declare-before-define (if supported by parser) |
| 🆕 `variables/load_local_raw` | method-receiver context | `LoadLocalRaw` (no Cell unwrap) — isolate it |

No overlap: reassignment lives here (single target); *chains* are category 7.

### 7. Assignment statements — unit-result targets & eval order  ·  **entirely 🆕**
Goal: expose the three target lowering paths and the compound eval-order. Assignment is a
**statement-typed expression with type `()`** (see verified fact #2), so there is *no*
value-returning chain to test — `a = b = c` is invalid and is deliberately excluded.

| Test | Source idea | Locks |
|---|---|---|
| 🆕 `assignment/assign_local` | `x = 1` (already-bound local) | plain `=` lowers to `StoreLocal`; expression result is unit |
| 🆕 `assignment/assign_field` | `p.x = 1` | `oxy_field_store` (not `StoreLocal`) |
| 🆕 `assignment/assign_index` | `v[0] = 1` | `oxy_vec_index_store` |
| 🆕 `assignment/compound_assign_add` | `x += 1` | `Add` then `StoreLocal`; byte target adds `oxy_cast_byte` |
| 🆕 `assignment/compound_assign_eval_order` | `x += f()` | value reg materialized **before** target load (mod.rs:1207-1208) |

> **Note:** chained assignment (`a = b = c`) is **not** valid Oxy — `Expr::Assign` is typed
> `()` (`check_expr.rs:806`), so the inner assign yields `()` and the chain cannot thread a
> value. No `assign_chained` snapshot exists; do not add one.

No overlap: category 6 is the plain-`=`-to-a-local *binding/reassign* slot story; this category
is non-local targets / compound / eval-order. New snapshot subdir: `tests/snapshots/ir/assignment/`.

### 8. Control flow (if / else)
Goal: branch terminators, phi / `__phi_tmp` continuation, empty-else unit.

| Test | Source idea | Locks |
|---|---|---|
| ✅ `control_flow/if_else_basic` | `if c {…} else {…}` | `Branch` + two arms |
| ✅ `control_flow/if_no_else` | `if c {…}` | implicit unit else |
| ✅ `control_flow/if_nested` | nested ifs | nested branch blocks |
| ✅ `control_flow/if_chain` | `if … else if …` | chained branches |
| 🆕 `control_flow/if_as_expression` | `let y = if c { 1 } else { 2 };` | value-producing if → phi continuation block |

No overlap: category 13 covers if *nested inside other expressions*; here if is statement/binding-level.

### 9. Loops
Goal: every loop terminator / back-edge shape and break/continue targets.

| Test | Source idea | Locks |
|---|---|---|
| ✅ `control_flow/while_basic` | `while c {…}` | back-edge `Jump`, header `Branch` |
| ✅ `control_flow/for_range` | `for i in 0..3 {…}` | range desugaring |
| ✅ `control_flow/loop_break` | `loop { … break; }` | break → exit jump |
| 🆕 `control_flow/loop_continue` | `loop { … continue; }` | continue back-edge distinct from break exit |
| 🆕 `control_flow/while_nested_break` | nested loops with inner break/continue | resolves to innermost loop blocks |
| 🆕 `control_flow/for_over_vec` | `for x in v {…}` (v: Vec) | iterator-protocol FFI vs range fast path |

No overlap: `for_range` = range desugaring; `for_over_vec` = iterator FFI; distinct lowerings.

### 10. Function definitions  (use `assert_ir_snapshot_raw`)
Goal: `params` metadata, return-type header, captures, async flag, recursion.

| Test | Source idea | Locks |
|---|---|---|
| ✅ `functions/fn_no_params` | `fn f() {…}` | empty params header |
| ✅ `functions/fn_with_params` | `fn add(a: int, b: int) -> int` | `params` metadata in header |
| ✅ `functions/fn_explicit_return` | `fn abs(x) { return … }` | explicit return blocks |
| ✅ `functions/fn_multiple_returns` | two `return`s | multiple return terminators |
| ✅ `functions/fn_recursive` | recursive call | self-reference resolution |
| ✅ `functions/closure_basic` | `\|a, b\| a + b` | closure as separate IrFunction, empty captures |
| 🆕 `functions/closure_with_captures` | closure referencing an outer local | non-empty `captures:` header line |
| 🆕 `functions/fn_async` | `async fn …` | `is_async` → ` async` in header |
| 🆕 `functions/fn_unit_return` | `fn f() -> () {…}` | explicit unit return-type header rendering |

No overlap: closures-with-captures vs capture-free closure_basic; async isolated from sync.

### 11. Function calls
Goal: each call-resolution route in `gen_expr`'s `Expr::Call` / method dispatch.

| Test | Source idea | Locks |
|---|---|---|
| ✅ `calls/call_println` | `println("hi")` | builtin FFI route |
| ✅ `calls/call_user_fn` | call a user fn | `oxy_call` route |
| ✅ `calls/call_with_args` | `f(1, 2)` | arg push order |
| ✅ `calls/method_call_string_len` | `s.len()` | string method dispatch |
| ✅ `calls/method_call_vec_push` | `v.push(1)` | vec method dispatch + raw receiver |
| 🆕 `calls/call_nested_args` | `f(g(x), h(y))` | arg evaluation order + result-reg threading |
| 🆕 `calls/call_enum_variant_ctor` | variant constructor call | routes to `oxy_make_enum_variant` |
| 🆕 `calls/method_chain` | `s.trim().len()` | receiver of call-2 = result of call-1 |

No overlap: each test pins exactly one resolution branch (builtin / user / method / variant / nesting).

### 12. Returns  (use `assert_ir_snapshot_raw`)
Goal: distinguish implicit tail return, explicit `return`, early return, and `WriteResult`/`Return` shape.

| Test | Source idea | Locks |
|---|---|---|
| ✅ `functions/fn_explicit_return` | (shared w/ cat 10) | explicit return |
| ✅ `functions/fn_multiple_returns` | (shared w/ cat 10) | multiple returns |
| 🆕 `returns/return_implicit_tail` | `fn f() -> int { 1 + 2 }` | tail-expr return vs explicit `return` — same or different IR? |
| 🆕 `returns/return_early_in_loop` | `return` inside a loop body | terminates block mid-loop (no back-edge after) |
| 🆕 `returns/return_unit_bare` | `return;` | `Return` of a `ConstUnit` |

No overlap: category 10 is about the *signature*; this is about the *return-site terminator/IR*.
New snapshot subdir: `tests/snapshots/ir/returns/`.

### 13. Edge cases (nested expressions, side effects)  ·  **entirely 🆕**
Goal: cross-cutting interactions no single-feature test exposes.

| Test | Source idea | Locks |
|---|---|---|
| 🆕 `edge_cases/nested_arith_calls` | `f(x) + g(y) * 2` | interleaved FFI calls + inline arith, reg threading |
| 🆕 `edge_cases/side_effect_order_in_args` | `f() + g()` / `vec![f(), g()]` | left-to-right materialization order |
| 🆕 `edge_cases/boolean_in_if_cond` | `if a && b {…}` | eager `And` op feeds the branch cond (no short-circuit) |
| 🆕 `edge_cases/block_expr_as_value` | `let x = { let t = 1; t + 1 };` | block tail value flows out |
| 🆕 `edge_cases/nested_if_in_arith` | `1 + (if c { 2 } else { 3 })` | phi continuation embedded in arithmetic |

No overlap: deliberately multi-feature; single-feature shape is owned by categories 1-12.
New snapshot subdir: `tests/snapshots/ir/edge_cases/`.

### 13b. Boolean logic (short-circuit) — locks the EAGER reality  ·  **entirely 🆕**
Goal: make the no-short-circuit behavior explicit and reviewable.

| Test | Source idea | Locks |
|---|---|---|
| 🆕 `boolean/bool_and_eager` | `a && b` | single `And(r, a, b)`; **both operands evaluated, no branch** |
| 🆕 `boolean/bool_or_eager` | `a \|\| b` | single `Or(r, a, b)` |
| 🆕 `boolean/bool_and_with_call` | `a && f()` | `f()` called **unconditionally** (the surprising case) |
| 🆕 `boolean/bool_chained` | `a && b \|\| c` | precedence + flat eager ops, zero branches |

> **⚠ Flag for reviewer:** if Oxy is intended to have C-style short-circuit evaluation, these
> snapshots are *wrong-by-design* and become the TODO marker for a lowering fix. The snapshot's
> job is to surface the decision, not assume it. New subdir: `tests/snapshots/ir/boolean/`.

---

## Summary

| Category | Existing ✅ | New 🆕 | Notes |
|---|---|---|---|
| 1 Literals | 5 | 3 | `const_char` is a real op-coverage gap |
| 2 Arithmetic | 2 | 3 | fill `Mul`/`Rem`, add float |
| 3 Precedence | 1 | 3 | only multi-op-tree tests |
| 4 Unary | 2 | 2 | `BitNot` uncovered |
| 5 Comparison/bitwise | 5 | 2 | `Gt/Le/Ge`, `BitXor/Shl/Shr` uncovered |
| 6 Variables | 6 | 2 | `LoadLocalRaw` isolation |
| 7 Assignment statements | 0 | 5 | **entirely uncovered**; assignment is `()`-typed, no chaining |
| 8 If/else | 4 | 1 | if-as-expression |
| 9 Loops | 3 | 3 | continue, nested, iterator-for |
| 10 Fn defs | 6 | 3 | captures, async, unit-return |
| 11 Calls | 5 | 3 | variant ctor, chains, nesting |
| 12 Returns | 2 | 3 | tail vs explicit, early-in-loop, bare unit |
| 13 Edge cases | 0 | 5 | **entirely uncovered** |
| 13b Boolean short-circuit | 0 | 4 | **entirely uncovered**; locks eager eval |
| **Total** | **~41** | **42** | ~83 golden files at full coverage |

Highest-value gaps (whole categories missing): **assignment statements (7)**, **edge cases (13)**,
**boolean short-circuit (13b)**. Highest-value single ops uncovered: `ConstChar`, `BitNot`,
`BitXor/Shl/Shr`, `Gt/Le/Ge`, `LoadLocalRaw`, `is_async` header, non-empty `captures` header.

## Non-overlap principle

Each IrOp variant and each lowering branch is "owned" by exactly one category:
- single-op tests own **op shape** (categories 1, 2, 4, 5),
- multi-op trees own **evaluation / precedence order** (categories 3, 13),
- assignment statements own **target lowering paths & compound eval-order** (category 7),
- CFG-shape tests own **terminators / blocks** (categories 8, 9, 12).

When a test would need two features, it goes in category 13/13b and the constituent features keep
their isolated single-feature snapshot.

## Coverage-completeness checks (verify after implementation)

- Every `IrOp` variant in `ir.rs` appears in ≥1 golden file (cross-check against the §6 op table
  in `IR_SNAPSHOT_FORMAT.md`).
- Every `Terminator` variant (`Return / Jump / Branch / Halt / Panic`) appears in ≥1 golden file.
- Each `gen_expr` / `gen_stmt` match arm of interest is exercised by exactly one owning test.
- No golden file contains a `<malformed: codegen-only @...>` line (forbidden-FFI guard, spec §8).
