# ir-cfg-jit — Path to Green & Merge

**Goal:** make every `.ox` file in `examples/` pass `cargo test -p oxy-core --test feature_examples`, with **proper root-cause fixes** (never edit a test to pass, never hack a single case). Then run the full pre-commit checklist and merge `ir-cfg-jit` → `main`.

**Baseline (2026-05-29):** `129 failed` in the `feature_examples` integration test.

**Progress:**
- ✅ **Cluster 1 done** (commit `26d33f5`): bool/unit register tagging. `feature_examples` 129 → **94**; `vm_tests` 113 → **107**; no regressions; clippy clean.

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

### Cluster 2 — Cross-function buffer / `local_count` corruption *(HIGH impact, MED-HIGH risk)*

**Symptom:** A function (e.g. a HashMap/HashSet test) **passes alone** but returns `Unit`/wrong values when compiled **alongside other functions** in the same file. Confirmed by extracting `test_hashmap_insert_get` to its own file (passes) vs running full `examples/features/stress/collections.ox` (fails).

**Suspected root cause:** The shared per-call execution buffer and the spill-slot layout are sized/offset inconsistently across functions. `codegen.rs` computes `capacity = ir_fn.local_count + STACK_CAP` (STACK_CAP=2048) and grows **spill slots downward from `capacity-1`** while the **operand stack grows upward from `local_count`**. CLAUDE.md documents the exact failure class: *"call_fn used engine.local_count (main's) for every function's buffer, but codegen computed spill offsets from each function's own local_count → silent heap corruption when a function had more locals than main."*

**Action before fixing — pin it precisely:**
1. Find where the runtime buffer is allocated per call (`jit/context.rs`, `jit/mod.rs`, `jit/ffi.rs`: `invoke_jit_fn`, `call_fn`, `CalleeFrame::new`, `with_capacity`, `local_count`).
2. Confirm whether each call's buffer is sized from **that function's own** `IrFunction.local_count` or from a single global (main's / engine-wide) value.
3. Confirm spill-slot offsets in codegen use the same `local_count` the runtime buffer was sized with.

**Fix direction (architectural, per CLAUDE.md guidance — no magic offsets):** ensure a *single source of truth* for each function's `local_count`, used consistently by (a) runtime buffer allocation and (b) codegen spill-slot computation. Likely: store per-function `local_count` in the JIT tables and have the runtime allocate each frame from *that* value (it already keeps `fn_local_counts` — verify every allocation path consults it rather than a cached main value).

**Verification:** `collections` (hashmap/hashset), `btreemap`, `hashmap`, `recursive_types` (tree), and any "passes alone / fails in-suite" Unit returns.

---

### Cluster 3 — Generic / trait method name resolution *(HIGH impact, diagnosed)*

**Symptom:** concrete inherent methods work (`Point::sum` → 7), but generic-impl / trait methods return `Unit`.

**Root cause (diagnosed):** `ir_gen/mod.rs` `gen_fn` (~L481–489) builds the method's registered name from the impl prefix, which **includes generic params** — e.g. `impl<T> Cell<T>` registers `"Cell<T>::make"`. At the call site (`ffi.rs` method lookup ~L1704; operator lookup ~L284–291), a `Value::Struct` only carries the **base name** `"Cell"`, so the lookup `format!("{base}::{method}")` = `"Cell::make"` misses → fallback leaves `ctx.result = Unit`.

**Fix:** strip generic params from the impl prefix when registering method IR names (`"Cell<T>" → "Cell"`), so registration and runtime lookup agree on the base name. Oxy is dynamic (values carry runtime type), so a single `Cell::make` correctly serves all `T` — no value-level monomorphization required. Verify the `monomorphization::*` tests' expectations (dedup, multi-type-args) are satisfied by base-name dispatch; if they assert distinct specializations, add dedup/registration logic accordingly.

**Verification:** `traits::test_generic_impl_*`, `traits::test_generic_struct_method`, `monomorphization::*`, `trait_def::test_multiple_trait_methods`, `trait_default::test_multiple_defaults`, `trait_bounds_static`, `impl_type_args::test_inherent_impl_with_type_args`.

---

### Cluster 4 — Operator overloading `div` / `rem` dispatch *(LOW count)*

**Symptom:** `operator_overloading::test_div_operator`, `test_rem_operator` return `Unit` (other operators were wired in commit `08686a1`).

**Action:** confirm whether this is the same base-name lookup miss as Cluster 3 (likely fixed by Cluster 3 if the struct is generic) **or** a genuine gap where `oxy_div`/`oxy_mod` don't dispatch to a user-defined `Div::div`/`Rem::rem` impl the way `oxy_add`/`oxy_mul` do. If the latter: wire `div`/`rem` through `lookup_op_method` in `ffi.rs` exactly like the already-working operators. Re-check after Cluster 3.

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
