# ir-cfg-jit — Path to Green & Merge

**Goal:** make every `.ox` file in `examples/` pass `cargo test -p oxy-core --test feature_examples`, with **proper root-cause fixes** (never edit a test to pass, never hack a single case). Then run the full pre-commit checklist and merge `ir-cfg-jit` → `main`.

**Baseline (2026-05-29):** `129 failed` in the `feature_examples` integration test.

**Progress:**
- ✅ **Cluster 1 done** (commit `26d33f5`): bool/unit register tagging. `feature_examples` 129 → **94**; `vm_tests` 113 → **107**; no regressions; clippy clean.
- ✅ **Cluster 2 done** (commit `60881ae`): **NOT** cross-function corruption — that hypothesis was stale (the `CalleeFrame` architecture already sizes every frame from the callee's own `local_count`, and the collections tests fail *standalone*, not only in-suite). Real root cause: stdlib **item path canonicalization**. Type-associated fns are registered by short path (`["HashMap","new"]`), but `use std::collections::HashMap` rewrites `HashMap::new` → `std::collections::HashMap::new`, which `lookup_item`'s exact match missed → constructors silently returned `Unit`, poisoning every downstream method call. Fixed by flattening segments on `::` and retrying the trailing `Type::method` pair; removed the per-type `Regex` band-aid that subsumed. `feature_examples` 87 → **76**; vm_tests unchanged (107); no regressions. Cleared: `collections`, `btreemap`, `hashmap`, `recursive_types` (ListNode/TreeNode), and others.
- ✅ **Cluster 3 done** (commit `a1f8832`): generic-impl method **name resolution** — methods in `impl<T> Cell<T>` / `impl Pair<int>` were registered under `Cell<T>::make` but resolved by base name `Cell::make`. Fixed via `base_type_name()` in IR + type checker. `feature_examples` 94 → **87**; no regressions.
  - Investigation revealed the rest of the original "Cluster 3" list is **two distinct root causes**, now split out below as Clusters 9 & 10:
    - **Tuple structs broken** (Cluster 9): `Num(int)` constructor calls aren't lowered to struct construction, so `.0` access returns `Unit`. Blocks `trait_def::test_multiple_trait_methods`, `operator_overloading::test_div_operator`/`test_rem_operator`, `traits::test_generic_struct_method` (partly), etc.
    - **Generic-fn monomorphization** (Cluster 10): `T::zero()` inside `fn make_zero<T>()` is mis-lowered as an enum-variant constructor `T::zero` instead of resolving `T` to the turbofish concrete type. Blocks all `monomorphization::*` and `trait_bounds_static::test_trait_static_method_bound`.

> Note: there are also pre-existing failures outside `feature_examples` to drive to green for full end-to-end: `vm_tests` (107) and `leetcode_solutions` (1). These share the same root-cause clusters below.

**Pipeline reminder:** `parse → type_check → ir_gen (AST → Register IR + CFG) → codegen (IR → Cranelift CLIF) → native`.

---

## How to measure

```bash
# Full integration suite (source of truth)
docker compose run --rm dev bash -c "cargo test -p oxy-core --test feature_examples 2>&1 | tail -40"

# Run a single .ox file's tests directly (fast iteration)
docker compose run --rm dev bash -c "cargo run -q --bin oxy -- test examples/features/<cat>/<file>.ox"

# Run a program's main()
docker compose run --rm dev bash -c "cargo run -q --bin oxy -- run <file>.ox"

# Dump IR for a failing test
OXY_VM_TRACE=1 docker compose run --rm dev bash -c "cargo test -p oxy-core --test feature_examples" 2> ir_dump.txt
```

> **Important:** the integration harness compiles **every function in a file together**. A test that passes when run alone can fail in-suite (see Cluster 2). Always reproduce against the *whole file* / full suite, not an isolated snippet.

---

## Root-cause clusters, in execution order

Order is by *impact × confidence × foundational-ness*. Re-run the full suite after **each** cluster — fixing a foundational bug flips many tests at once and re-triages the "misc" bucket. Failure counts per cluster are approximate; the suite re-measures truth.

---

### ✅ Cluster 1 — Bool / Unit value tagging — DONE (commit `26d33f5`)

**Symptom:** `true`→`1`, `false`→`0`, `false.to_string()`→`"0"`; asserts fail with `left: Bool(true), right: I64(1)` or `I64(0) is not truthy`.

**Root cause (confirmed):** In `codegen.rs`, registers in the `regs` map hold *raw, untyped i64*. `ConstBool`, `ConstUnit`, and the **inline comparison fast-path** (`Eq/Neq/Lt/Gt/Le/Ge` when both operands are in `regs`) all stash a raw i64 in `regs`. When such a register is later materialized into a `Value` via `push_reg`, it is *unconditionally* tagged `oxy_push_int` → `Value::I64`. (By contrast `ConstFloat`/`ConstChar`/`ConstString` correctly **push a tagged value onto the operand stack and `spill_result`** into a slot, so they round-trip with the right tag. FFI comparisons like `oxy_eq` also produce a properly tagged `Bool`.) That's why `x > 3` (FFI path) prints `true` but `let t = true` (ConstBool) prints `1`.

**Fix (mirror the existing `ConstFloat` pattern — no new plumbing):**
- `codegen.rs` `IrOp::ConstBool` (~L521): push via `oxy_push_bool` (I8) + `spill_result`; stop inserting raw i64 into `regs`.
- `codegen.rs` `IrOp::ConstUnit` (~L524): push via `oxy_push_unit` + `spill_result`.
- `codegen.rs` `IrOp::Eq/Neq/Lt/Gt/Le/Ge` inline branches (~L547–606): keep the `icmp`, but push the I8 result via `oxy_push_bool` + `spill_result` instead of `regs.insert(uextend …)`.
- `codegen.rs` `IrOp::Copy` (~L607) and any `regs[a]` indexing: make robust to a source that now lives in `reg_slot` (copy the slot mapping instead of panicking on `regs[a]`).
- Branch terminator (~L310) already reads spilled conditions via `oxy_read_local_i64`, so control flow is unaffected.

**Verification:** `strings/literals.ox`, `control_flow`, `error_handling` (is_some/is_none/is_ok/is_err), `numeric_ops` (float_eq/float_lt/precedence), `operator_types`, `iterators` (all/any), `rand_regex_stdlib::test_rand_bool_returns_bool`, `short_circuit`, plus the `Bool(true) vs I64(1)` failures inside `select`/`spawn`.

---

### ✅ Cluster 2 — stdlib item path canonicalization — DONE (commit `60881ae`)

**Original (wrong) hypothesis:** cross-function buffer / `local_count` corruption — "passes alone, fails in-suite." **Disproven:** the collections tests fail *standalone* too (`cargo run -- test examples/features/stress/collections.ox` → 10 failed), and the runtime already allocates each `CalleeFrame` from the **callee's own** `local_count` via `tables.local_count(fn_index)` (see `ffi.rs` `invoke_jit_fn`, `invoke_binary_op_method`, `oxy_call_closure`). The CLAUDE.md corruption lesson was already-fixed history, not a live bug.

**Actual root cause:** `oxy_path_call_builtin` (`ffi.rs`) resolves a path call by, first, `registry::lookup_item`. The registry indexes type-associated fns by **short path** (`["HashMap","new"]`), but `use std::collections::HashMap; HashMap::new()` is rewritten by use-alias resolution to the canonical `std::collections::HashMap::new` (one segment `"std::collections::HashMap"` + `"new"`). `lookup_item` did an **exact** match only → miss → fell through to the enum-variant / "unknown built-in path" branch and pushed `Value::Unit`. Every collection constructor (`HashMap`/`HashSet`/`BTreeMap`/`BTreeSet`/`BinaryHeap`/`VecDeque`/`ListNode`/`TreeNode`) returned `Unit`, so all downstream `.insert`/`.len`/etc. operated on `Unit`. The smell that confirmed it: `Regex::new` was registered **twice** — `["Regex","new"]` *and* `["std","regex","Regex","new"]` — a per-type band-aid for this exact mismatch.

**Fix:** `lookup_item` now flattens each segment on `::` and, on exact-match miss, retries against the trailing `Type::method` pair — canonicalizing every collection constructor at once. Removed the redundant `Regex` 4-segment registration (subsumed). Safe because registered 2-segment items are all reserved CamelCase builtin type names a user cannot redefine.

**Verification (done):** `collections` 18→28 pass; `btreemap`, `hashmap`, `recursive_types` cleared; regex construction still works (regex_oop's remaining failures are separate method-dispatch bugs).

---

### ✅ Cluster 3 — Generic / trait method name resolution — DONE (commit `a1f8832`)

**Symptom:** concrete inherent methods work (`Point::sum` → 7), but generic-impl / trait methods return `Unit`.

**Root cause (diagnosed):** `ir_gen/mod.rs` `gen_fn` (~L481–489) builds the method's registered name from the impl prefix, which **includes generic params** — e.g. `impl<T> Cell<T>` registers `"Cell<T>::make"`. At the call site (`ffi.rs` method lookup ~L1704; operator lookup ~L284–291), a `Value::Struct` only carries the **base name** `"Cell"`, so the lookup `format!("{base}::{method}")` = `"Cell::make"` misses → fallback leaves `ctx.result = Unit`.

**Fix:** strip generic params from the impl prefix when registering method IR names (`"Cell<T>" → "Cell"`), so registration and runtime lookup agree on the base name. Oxy is dynamic (values carry runtime type), so a single `Cell::make` correctly serves all `T` — no value-level monomorphization required. Verify the `monomorphization::*` tests' expectations (dedup, multi-type-args) are satisfied by base-name dispatch; if they assert distinct specializations, add dedup/registration logic accordingly.

**Verification:** `traits::test_generic_impl_*`, `traits::test_generic_struct_method`, `monomorphization::*`, `trait_def::test_multiple_trait_methods`, `trait_default::test_multiple_defaults`, `trait_bounds_static`, `impl_type_args::test_inherent_impl_with_type_args`.

---

### ~~Cluster 4 — Operator overloading `div` / `rem`~~ — RESOLVED AS Cluster 9

**Finding:** not an operator-wiring gap. `operator_overloading` uses tuple structs (`WrappedInt(int)`); the `Unit` comes from broken tuple-struct construction / `.0` access (Cluster 9), not from `div`/`rem` dispatch. The `binary_op!` macro already routes `div`→`"div"`/`rem`→`"rem"` to `lookup_op_method`. Will clear once Cluster 9 lands.

---

### Cluster 5 — Named functions as first-class values *(diagnosed)*

**Symptom:** `let f = dbl; f(10)` prints `()` then "value is not a callable closure". Cluster: `fn_as_value::test_named_fn_as_arg`, `test_two_arg_fn_pointer`, `test_fns_in_vec`, `test_pass_different_named_fns`.

**Root cause (diagnosed):** `ir_gen/mod.rs` `Expr::Ident` (~L729–752) handles locals, consts, and enum variants, then **falls through to `ConstUnit`** for a bare function name. The machinery already exists — `Expr::Call` (~L871–896) emits `oxy_push_named_fn` (FFI `ffi.rs` ~L798) to build a `Value::Function`.

**Fix:** in the `Expr::Ident` fallthrough, emit `oxy_push_named_fn` (resolving `use_aliases`/fn aliases first) instead of `ConstUnit`, so a bare function name evaluates to a callable `Value::Function`. Guard so genuine unresolved identifiers (which the type checker should already reject) don't silently become bogus function values.

**Verification:** `fn_as_value::*`.

---

### Cluster 6 — Missing compile-error enforcement (type checker) *(diagnosed)*

**Symptom:** `#[compile_error]` tests "compiled successfully" instead of being rejected.

**Root causes & fixes (all in `type_checker/check_expr.rs`):**
- **`sleep` arity** (`async_await::sleep_zero_args`, `sleep_two_args`): no arity check. Add a `"sleep"` case requiring exactly 1 arg.
- **`spawn` arity + non-closure** (`spawn_zero_args/two_args/non_closure`): the `"spawn"` case (~L554) silently tolerates wrong arity / non-closure. Require exactly 1 arg **and** that it is a closure.
- **`select` arity** (`select::select_zero_args`, `select_one_arg`): the `"select"` case (~L565) accepts any count. Require ≥ 2 JoinHandle args.
- **array repeat non-const count** (`arrays::test_array_repeat_non_constant_count`): `Expr::Repeat` (~L680) silently falls back to `0` when `count` isn't an `IntLiteral`. Make a non-constant count a type error.

> Per CLAUDE.md: a `#[compile_error]` test passes if **either** the type checker **or** ir_gen/codegen rejects it. Prefer the type checker.

**Verification:** `async_await::*` (the `compile_error` ones), `select` arity ones, `arrays::test_array_repeat_non_constant_count`.

---

### Cluster 7 — Modules / visibility *(NEEDS INVESTIGATION)*

**Tests:** `modules::*` (incl. `test_module_pub_fn` showing a **sign bug** `left: I64(-6), right: I64(6)`, `test_pub_crate`, `test_enum_in_module`, `test_enum_via_use`, `test_field_visibility_pub`, `test_module_pub_fn`), `visibility::*`, `file_modules::*`, `field_visibility::*`, `private_use_call::test_private_fn_via_glob`, `pub_modifiers::test_pub_crate_from_sibling_module`, `basic::test_enum_in_module`, `basic::test_struct_via_qualified_path`.

**Action:** investigate after Clusters 1–3 (some may already flip). Likely sub-causes: qualified-path call resolution in ir_gen (the `-6` vs `6` sign bug suggests a wrong-function or wrong-arg dispatch), `use`-alias resolution, and `pub(crate)`/field-visibility enforcement paths. Trace `Expr::PathCall`/`Expr::Path` lowering and `check_path_visible`/`is_visible`.

---

### Cluster 8 — Misc / re-triage *(after 1–7)*

Re-run the suite; the remaining set will be much smaller. Expected residual candidates and where to look:
- `if_else::test_if_expression_no_else` (`I64(0)` vs `42`) — value of an `if` without `else` used as expression.
- `math_stdlib::test_pi_constant` — PI constant value / comparison.
- `struct_basics::test_struct_field_mutation`, `struct_field_types::test_struct_field_mut_assign_ok` — field assignment / mutation path.
- `closures`/`capture` (`test_mutable_capture`, `test_closure_modifies_capture`, `test_multiple_closures_same_capture`, `test_capture_with_param_and_mut`) — mutable capture semantics.
- `complex_patterns::test_deeply_nested_match`, `match_exhaustive::test_match_nested`.
- `btreemap::test_bracket_get`, `hashmap::test_bracket_get` — `m[k]` indexing.
- `generics::test_turbofish_on_path_call`, `test_option_of_vec_int`, `test_hashmap_string_int`.
- `regex_oop::*`, `rand_regex_stdlib::regex_*` — regex method dispatch (may be Cluster 2/3).
- `select::*` async-logic ones not covered by Cluster 1.
- `type_checking::test_closure_empty_body`, `consumers::test_for_each_side_effect`, `vec_iterators::test_for_each`, `operators::test_builder_pattern`, `operators::test_mut_self_in_method`, `error_handling::test_question_double_chain_short_circuits`.

Each gets a proper root-cause trace (read failing `.ox` → trace ir_gen → trace codegen → check FFI), no per-test hacks.

---

### Cluster 9 — Tuple structs (constructor + field access) *(NEW — diagnosed)*

**Symptom:** `Num(int)` / `WrappedInt(int)` tuple structs: constructing `Num(10)` then accessing `a.0` returns `Unit` — reproduces at top level, in one minimal function (not cross-function).

**Root cause (diagnosed):** ir_gen `Expr::Call` (`ir_gen/mod.rs` ~L799–915) recognizes enum-variant constructors and `spawn`/`sleep`/`select`, then falls through to the named-function call path (`oxy_push_named_fn` → `oxy_call_closure`). A tuple-struct constructor call `Num(10)` is **not** recognized, so no `Value::Struct` is built; `a` ends up `Unit` and `a.0` (via `oxy_field_access`, which reads `fields.get("0")`) returns `Unit`. Named-field structs work because they use `Expr::StructInit` → `oxy_struct_init`.

**Fix direction:** teach ir_gen to recognize a call whose callee is a known tuple-struct name and lower it to a struct construction (build `Value::Struct{ name, fields: {"0":…, "1":…} }`, matching what `oxy_field_access`/`oxy_field_store` expect). ir_gen needs the set of tuple-struct names+arity (collected from struct defs, like `variant_to_enum` is for enum variants). Verify the type checker already accepts `Name(args)` for tuple structs (it must, since these reach runtime).

**Verification:** `trait_def::test_multiple_trait_methods`, `operator_overloading::test_div_operator`/`test_rem_operator`, `traits::test_generic_struct_method`, and any other tuple-struct (`Name(T)`) usage.

### Cluster 10 — Generic-function monomorphization (`T::method()`) *(NEW — diagnosed)*

**Symptom:** `monomorphization::*` and `trait_bounds_static::test_trait_static_method_bound` return `EnumVariant { enum_name: "T", variant: "zero" }` where a value (e.g. `I64(0)`) is expected.

**Root cause (diagnosed):** inside `fn make_zero<T: Zero>() { T::zero() }`, the path call `T::zero()` is lowered as an **enum-variant constructor** `T::zero` rather than resolving the type parameter `T` to the turbofish-supplied concrete type (`make_zero::<int>()` → `int::zero()`). Needs turbofish-driven type substitution at generic-function call sites so `T::method` dispatches to `Concrete::method`.

**Fix direction:** propagate turbofish/inferred type args into the generic function body's `T::…` path resolution (monomorphize, or pass a type-binding so `T::zero` resolves to `int::zero` at the call). Larger than Clusters 1/3 — likely its own focused effort.

**Verification:** `monomorphization::*`, `trait_bounds_static::test_trait_static_method_bound`.

## Closeout

1. After all clusters green, run the **full pre-commit checklist** from CLAUDE.md:
   ```bash
   docker compose run --rm dev bash -c "cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p oxy-core"
   docker compose run --rm dev bash -c "cargo clippy -p oxy-lsp --all-targets -- -D warnings && cargo test -p oxy-lsp"
   docker compose run --rm dev bash -c "cargo clippy -p oxy-tug --all-targets -- -D warnings && cargo test -p oxy-tug"
   docker compose run --rm dev bash -c "rustup target add wasm32-unknown-unknown 2>/dev/null; cargo check --target wasm32-unknown-unknown -p oxy-core --no-default-features"
   ```
2. Update LSP / REPL / VS Code extension if any new symbols or constructs were added (per the TDD process step 6).
3. Commit per cluster with conventional-commit messages (`fix:`/`feat:`), no co-author trailers.
4. Open PR `ir-cfg-jit` → `main` once the full suite + checklist are green.

## Guardrails (non-negotiable)
- Never modify a `.ox` test to make it pass when the compiler should handle it.
- No magic offsets / special-case guards papering over architecture (esp. Cluster 2). If a fix needs one, flag it.
- If the same bug pattern appears in >1 place, build a shared abstraction.
- Update both `symbols.rs` and the dispatch site when touching built-in methods.
