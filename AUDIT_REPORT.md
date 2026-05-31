# Oxy Codebase Audit Report — May 2026

Comprehensive audit covering bugs, simplifications, DRY violations, modularization, readability, and architecture across the entire Oxy compiler codebase.

---

## Summary

| Category | HIGH | MEDIUM | LOW |
|---|---|---|---|
| Bugs | 4 | 3 | 2 |
| DRY / Duplication | 6 | 5 | 4 |
| Modularization (files to split) | 6 | 4 | 3 |
| Architecture | 2 | 2 | 1 |
| Readability / Safety Docs | 2 | 3 | 2 |

---

## 1. BUGS

### HIGH — String builtin `push_str` silently eats errors
**File:** `crates/oxy-core/src/vm/builtins/string.rs:67`
`push_str` uses `eprintln!()` and returns `Ok(Value::Unit)` for unsupported operations instead of returning `Err(...)` like every other builtin. This silently swallows the error at runtime.
**Fix:** Return `Err(format!("String::push_str is unsupported (strings are immutable in Oxy)"))`.

### HIGH — Numeric dispatch silently coerces unknown types to 0.0
**File:** `crates/oxy-core/src/vm/builtins/numeric.rs:8-13`
The `to_f64` closure silently returns `0.0` for non-numeric types. If a non-numeric type somehow reaches numeric dispatch, it silently produces wrong results.
**Fix:** Return an error or use `unreachable!()` with a message.

### HIGH — Array repeat with non-const count silently falls back to 0
**File:** `docs/history/ir-cfg-jit-fix-log.md:114` (actual code location not specified)
When count is not an `IntLiteral`, it silently defaults to 0 elements. This is a latent bug — the user's array comes out empty with no error.
**Fix:** Emit a compile error when array repeat count is not a compile-time constant.

### HIGH — `unreachable!()` in parser `parse_struct_init`
**File:** `crates/oxy-core/src/parser/expr.rs:23`
A two-step check (`matches!` then `if let`) leaves a dead `unreachable!()` arm. A future refactor could accidentally hit it.
**Fix:** Replace with a single `if let TokenKind::IntLiteral(n, _) = self.peek_kind()`.

### MEDIUM — String builtin `method_names()` uses raw strings instead of symbol constants
**File:** `crates/oxy-core/src/vm/builtins/string.rs:139-162`
All other builtins reference `symbols::<type>_m::CONSTANT` for method names. String uses raw `"len"`, `"is_empty"`, etc. If symbol constants change, string will silently diverge.
**Fix:** Use `symbols::string_m::LEN`, `symbols::string_m::IS_EMPTY`, etc.

### MEDIUM — `Value::as_i64()` / `as_u64()` / `to_f64()` panic on wrong type
**File:** `crates/oxy-core/src/types/mod.rs:128-152`
These panicking accessors have no safe query alternative. Callers use `unwrap()` after checking the type variant, but if a new variant is added, these become runtime panics.
**Fix:** Return `Result<i64, PipelineError>` instead of panicking.

### MEDIUM — Comparison methods use unwrap that can crash
**File:** `crates/oxy-core/src/types/mod.rs:985, 988, 1046-1047, 1445`
`self.as_i128().unwrap()`, `self.as_f64().unwrap()`, `fa.get(k).unwrap()`, `a.partial_cmp(&b).unwrap()` — these all assume types are compatible. An unexpected value type crashes the interpreter.
**Fix:** Return `Result` or use proper error propagation.

### LOW — Potential panic in string slice bounds
**File:** `crates/oxy-core/src/vm/builtins/string.rs:95`
If `start > end` with an empty chars collection, `chars[start..end]` panics. Check `start <= end` before indexing.

### LOW — FFI `clone()` panic can leak buffer values
**File:** `crates/oxy-core/src/vm/jit/ffi/mod.rs:47,155,174,188,259,788,905`
`std::mem::forget(val)` after `ptr::read` is only safe if `clone()` doesn't panic. If `clone()` panics, the original buffer value is leaked.
**Fix:** Document the invariant or use a drop-guard.

---

## 2. DRY / DUPLICATION

### HIGH — HashMap/BTreeMap builtins ~91% identical
**Files:** `crates/oxy-core/src/vm/builtins/hashmap.rs` vs `btreemap.rs`
Every dispatch arm is identical except the `Value::HashMap` / `Value::BTreeMap` variant wrapper. ~80 lines of copy-paste.
**Fix:** Create a `builtin_collection!` macro or extract shared helpers.

### HIGH — HashSet/BTreeSet builtins ~90% identical
**Files:** `crates/oxy-core/src/vm/builtins/hashset.rs` vs `btreeset.rs`
Same pattern as HashMap/BTreeMap — only the type variant and symbol prefix differ.
**Fix:** Same `builtin_collection!` macro.

### HIGH — `Item::Impl` vs `Item::ImplTrait` in `collect.rs` — 95% identical
**File:** `crates/oxy-core/src/type_checker/collect.rs:171-226` vs `228-278`
~55 lines duplicated. Only the type name extraction differs (`i.base_type_name()` vs `i.type_name`).
**Fix:** Extract shared logic into a helper function.

### HIGH — `UseTree` handling duplicated between `check_item.rs` and `check_stmt.rs`
**Files:** `crates/oxy-core/src/type_checker/check_item.rs:42-67` and `check_stmt.rs:242-269`
Both parse `UseTree::Simple`, `Group`, `Glob` with identical logic.
**Fix:** Extract into a shared function.

### HIGH — `invoke_compiled_method` duplicates `CalleeFrame`
**File:** `crates/oxy-core/src/vm/jit/ffi/mod.rs:1239-1297` vs `681-731`
`invoke_compiled_method` manually performs alloc/swap/call/drop/dealloc/restore — exactly what `CalleeFrame::execute` abstracts. It was written before `CalleeFrame` and never retrofitted.
**Fix:** Refactor to use `CalleeFrame::execute`.

### HIGH — Comma-separated element parsing repeated 7+ times in parser
**Files:** `crates/oxy-core/src/parser/expr.rs` and `pattern.rs`
The pattern `if !check(&RParen) { loop { parse; if !match_token(&Comma) { break; } } }` appears 7+ times for tuples, arrays, macro args, struct fields, tuple patterns, slice patterns, and variant tuple patterns.
**Fix:** Extract `parse_comma_separated<T>(parser: fn) -> Vec<T>` helper.

### MEDIUM — `inner_of` function duplicated between `option.rs` and `result.rs`
**Files:** `crates/oxy-core/src/vm/builtins/option.rs:12-18` vs `result.rs:10-16`
Verbatim copy of the function that extracts the first data element from `Value::EnumVariant`.
**Fix:** Move to a shared location or `types::Value`.

### MEDIUM — `oxy_push_closure` and `oxy_push_async_block` duplicate capture-copying
**File:** `crates/oxy-core/src/vm/jit/ffi/mod.rs:781-792` and `898-909`
Identical loop iterating `captured`, reading outer slots, handling `Cell` sharing, `mem::forget`.
**Fix:** Extract into a helper function.

### MEDIUM — `invoke_binary_op_method` vs `invoke_unary_op_method` structurally identical
**File:** `crates/oxy-core/src/vm/jit/ffi/mod.rs:343-394` vs `396-438`
Only differ in `total_frame` (max(2) vs max(1)) and arg count (0 vs 1 args).
**Fix:** Unify into one function with an arity parameter.

### MEDIUM — Binary op compilation: 21 nearly-identical arms in codegen
**File:** `crates/oxy-core/src/vm/jit/codegen.rs:548-669`
Each arm (Add, Sub, Mul, ..., Shr, BitAnd, ...) does `call_ffi_binary(...)` then `spill_result(...)`.
**Fix:** A helper closure or macro would eliminate the repetition.

### MEDIUM — Arithmetic functions in `runtime.rs` have identical structure
**File:** `crates/oxy-core/src/vm/jit/runtime.rs:31-189`
`vm_add` through `vm_rem` (5 functions) and `vm_bitand`/`vm_bitor`/`vm_bitxor` (3 functions) all share the same structure with only operator symbols differing.
**Fix:** Use a macro or shared error helper.

### LOW — `make_array` / `make_fixed_array` / `make_tuple` identical except wrapper
**File:** `crates/oxy-core/src/vm/jit/ffi/collections.rs:6-43`
Three functions identical except the final `Value::*` wrapper.
**Fix:** One helper `fn collect_n(ctx, count, wrapper)`.

### LOW — `escape_string` / `escape_char` nearly identical in `ir_snapshot.rs`
**File:** `crates/oxy-core/src/vm/jit/ir_snapshot.rs:462-495`
Only `'"'` vs `'\''` and a trailing `.to_string()` differ.
**Fix:** Extract shared escape logic.

### LOW — Range-slicing duplicated across String/Vec/Array in `vec_index`
**File:** `crates/oxy-core/src/vm/jit/ffi/collections.rs:252-297`
Clamped start/end computation and bounds checking repeated 3x.
**Fix:** Extract into a closure or helper.

### LOW — Vec builtin index-extraction repeated 5-fold
**File:** `crates/oxy-core/src/vm/builtins/vec.rs:49-55,62-68,75-81,141-147,156-162`
`args.first().and_then(|v| match v { Value::I64(n) => Some(*n as usize), _ => None })` repeated 5 times.
**Fix:** Extract `fn extract_index(args: &[Value]) -> Option<usize>`.

---

## 3. MODULARIZATION (files to split)

### HIGH — `ffi/mod.rs` (2252 lines)
Split into: `ffi/arithmetic.rs`, `ffi/calls.rs`, `ffi/async.rs`, `ffi/method_dispatch.rs`. The README already acknowledges this need.

### HIGH — `ast/mod.rs` (1657 lines, 28+ types)
Split into: `ast/expr.rs` (Expr, BinOp, UnaryOp, MatchArm, Pattern), `ast/stmt.rs` (Stmt, Block), `ast/item.rs` (Item, FnDef, StructDef, EnumDef, ImplBlock, TraitDef, etc.).

### HIGH — `types/mod.rs` (1502 lines)
Split into: `types/value.rs` (enum + core impls), `types/display.rs` (Display impls), `types/comparison.rs` (PartialEq/Ord/Hash impls), `types/iter.rs` (IteratorState).

### HIGH — `oxy-lsp/src/main.rs` (1198 lines)
Split into: `handlers.rs`, `diagnostics.rs`, `completions.rs`, `hover.rs`, `symbols.rs`, `type_inference.rs`, `goto_definition.rs`, `util.rs`. Main file reduces to ~30 lines.

### HIGH — `parser/expr.rs` — `parse_prefix` (517 lines)
Split into sub-functions: `parse_literal_or_ident`, `parse_closure`, `parse_grouped_or_tuple`, `parse_array_or_vec`, `parse_if_match_loop`, `parse_unary`.

### HIGH — `ir_gen/expressions.rs` — `gen_expr` (900+ lines)
Split into: `ir_gen/literals.rs`, `ir_gen/operators.rs`, `ir_gen/calls.rs`, `ir_gen/structs.rs`.

### MEDIUM — `lexer/mod.rs` (1471 lines, 60+ helpers)
Split into: `lexer/char_reader.rs`, `lexer/numbers.rs`, `lexer/strings.rs`.

### MEDIUM — `json/mod.rs` (706 lines)
Split into: `json/serialize.rs`, `json/deserialize.rs`.

### MEDIUM — `stdlib/server.rs` (839 lines, mostly free functions)
Split into: `stdlib/server/router.rs`, `stdlib/server/response.rs`, `stdlib/server/handler.rs`.

### MEDIUM — `vm/interp.rs` (1139 lines)
Extract test functions (lines 962-1139, ~180 lines) to `vm/interp/tests.rs`. Extract `call_raw` / `call_collect` FFI infrastructure to `vm/interp/ffi.rs`.

### LOW — `vm/tests.rs` (996 lines, 131 tests)
Can be reduced ~30% via a test helper macro for the `run_compiled` boilerplate.

---

## 4. ARCHITECTURE

### HIGH — Duplicate name resolution in ir_gen
**Files:** `crates/oxy-core/src/type_checker/resolve.rs` and `crates/oxy-core/src/vm/jit/ir_gen/resolve.rs`
Both the type checker and ir_gen maintain their own `use_aliases`, `fn_aliases`, `glob_mods`, and `variant_to_enum` maps. Name resolution runs twice — once in the type checker, again in ir_gen. The type checker already walked all items and could annotate AST nodes with resolved paths.
**Fix:** Have the type checker annotate AST nodes with fully-resolved paths so ir_gen doesn't need its own resolve pass.

### HIGH — Two `TypeInfo` types with the same name
**Files:** `crates/oxy-core/src/type_checker/mod.rs:28` (semantic type) and `crates/oxy-core/src/symbols.rs:28` (LSP documentation metadata)
These are completely different concepts sharing the same name. The symbols.rs `TypeInfo` is really a `TypeDoc` or `TypeHelp`.
**Fix:** Rename `symbols.rs::TypeInfo` to `TypeDoc` or `SymbolTypeInfo`.

### MEDIUM — KEYWORDS duplicated between lexer/token.rs and symbols.rs
**Files:** `crates/oxy-core/src/lexer/token.rs` (via `TokenKind::from_keyword()`) and `crates/oxy-core/src/symbols.rs` (~lines 68-97)
The keywords list is independently maintained in two places. If keywords change, symbols.rs must be manually updated.
**Fix:** Derive `KEYWORDS` in symbols.rs from `TokenKind::from_keyword()`.

### MEDIUM — `oxy-tug` uses `Result<_, String>` everywhere instead of proper error types
**Files:** `crates/oxy-tug/src/project.rs`, `runner.rs`, `install.rs`, `lockfile.rs`, `manifest.rs`
String errors lose all structure and make programmatic error handling impossible.
**Fix:** Create a `TugError` enum using `thiserror`.

### LOW — `CallFrame` and utilities misplaced in `errors.rs`
**File:** `crates/oxy-core/src/errors.rs:109,133,149`
`CallFrame` is a VM/debugging concept. `edit_distance` and `suggest_name` are general utilities. None belong in the error types file.
**Fix:** Move `CallFrame` to `vm/`, utilities to a new `util.rs`.

### LOW — 11/15 stdlib modules have unused `_cb: ClosureInvoker<'_>` parameter
**Files:** `crates/oxy-core/src/stdlib/json.rs`, `http.rs`, `time.rs`, `rand.rs`, `io.rs`, `math.rs`, `net.rs`, `path.rs`, `args.rs`, `env.rs`, `fs.rs`
Only `server.rs` and `db.rs` use the callback. The `ModuleCall` signature forces all modules to accept it.
**Fix:** Consider splitting `ModuleCall` into two variants or using `Option<ClosureInvoker>`.

---

## 5. READABILITY / SAFETY

### HIGH — Zero `// Safety:` documentation on any unsafe block
**Files:** All unsafe blocks in `vm/interp.rs`, `vm/jit/ffi/casts.rs`, `vm/jit/ffi/enums.rs`, `vm/jit/ffi/mod.rs`
Not a single `unsafe` block or `unsafe fn` has a `// Safety:` comment documenting its preconditions. This is the standard Rust convention and is missing entirely.
**Fix:** Add `// Safety:` doc comments to every unsafe block.

### HIGH — 12 blanket `#[allow(clippy::*)]` in `lib.rs` suppress real issues
**File:** `crates/oxy-core/src/lib.rs:8-21`
Includes `mutable_key_type`, `type_complexity`, `useless_format`, `needless_borrow`, `single_match`, `map_clone`, `assigning_clones`. These hide real problems.
**Fix:** Move each allow to the specific location that needs it, with a justification comment.

### MEDIUM — `symbols.rs` is a fragile manual registry
**File:** `crates/oxy-core/src/symbols.rs` (819 lines)
Adding a type, method, or keyword requires updating this file or consistency tests fail. The symbols file is a parallel manual registry that must stay in-sync with actual implementations.
**Fix:** Consider code generation or derive macros to keep the registry in sync automatically.

### MEDIUM — `ir_snapshot_tests.rs` (807 lines) and `symbol_consistency.rs` (349 lines) — repetitive test boilerplate
Both files have dozens of near-identical test functions that differ only by input string or type name.
**Fix:** Use `rstest` or `test-case` crate for parameterized tests.

### MEDIUM — `JitContext::new()` relies on zeroed-buffer drop safety
**File:** `crates/oxy-core/src/vm/jit/context.rs:109`
Dropping an all-zero buffer works only because `Value::Unit` (discriminant 0) has no heap data. A future change to `Value::Unit` would cause UB.
**Fix:** Document this invariant explicitly.

### LOW — `#[test] fn main()` ambiguity in `vm/tests.rs`
**File:** `crates/oxy-core/src/vm/tests.rs:11,23`
Test functions named `main` containing source strings with inner `fn main()`. The naming creates confusion.
**Fix:** Rename test functions to describe what they test.

### LOW — `obi-cast_to_char` vs `obi-cast_int` naming inconsistency
**File:** `crates/oxy-core/src/vm/jit/ffi/casts.rs`
`oxy_cast_to_char` has `_to_` but `oxy_cast_int`, `oxy_cast_float`, `oxy_cast_byte` don't.
**Fix:** Rename to `oxy_cast_char` for consistency.

---

## 6. POSITIVE FINDINGS

Strengths of the codebase that should be preserved:

- **README coverage is excellent** — Every source directory has a README.md. All are accurate and current.
- **Two-backend architecture is clean** — JIT and interpreter share one IR and one runtime. The divergence guards (compile-time, test-time, runtime) are well-designed.
- **Error handling is unified** — Single `PipelineError` enum with `thiserror`. Clean `Result` propagation throughout the pipeline.
- **Type checker `check_expr/` sub-module split is well-done** — 6 sub-modules by expression category (calls, closures, control_flow, data, operators, primary) with a thin dispatcher. This is the model other modules should follow.
- **No circular dependencies** — Clean dependency flow: ast → parser → type_checker → ir_gen → codegen.
- **All clippy allows are justified with inline comments** (except the blanket ones in lib.rs).
- **No TODO/FIXME/HACK in production source** — Technical debt is tracked externally.
- **`ir_snapshot.rs` is the best-documented file** — Section references to the spec make it a model for documentation.
- **`std lib/registry.rs` has excellent module-level documentation** explaining the two-entry dispatch system.

---

## 7. PRIORITY ACTION PLAN

### Immediate (bug fixes)
1. Fix `string.rs` `push_str` — return `Err` instead of `eprintln!`
2. Fix `numeric.rs` `to_f64` — don't silently coerce to 0.0
3. Fix array repeat non-const count — emit compile error instead of silently returning 0
4. Fix `expr.rs:23` — remove `unreachable!()` with single `if let`

### Short-term (DRY + modularization)
5. Create `builtin_collection!` macro for HashMap/BTreeMap/HashSet/BTreeSet
6. Split `ffi/mod.rs` into submodules
7. Split `ast/mod.rs` into `expr.rs`, `stmt.rs`, `item.rs`
8. Split `types/mod.rs` into `value.rs`, `display.rs`, `comparison.rs`
9. Split `oxy-lsp/src/main.rs` into proper modules
10. Extract duplicated `UseTree` handling and `Item::Impl`/`ImplTrait` logic in type checker
11. Add `// Safety:` docs to all unsafe blocks

### Medium-term (architecture)
12. Rename `symbols.rs::TypeInfo` to `TypeDoc`
13. Have type checker annotate AST with resolved paths; eliminate ir_gen resolve pass
14. Replace `Result<_, String>` in tug with proper `TugError` enum
15. Remove blanket clippy allows from `lib.rs`
16. Parameterize repetitive tests with `rstest`/`test-case`

### Long-term (polish)
17. Split `lexer/mod.rs`, `json/mod.rs`, `stdlib/server.rs`
18. Derive KEYWORDS in symbols.rs from token.rs
19. Move `CallFrame` out of `errors.rs`
20. Consolidate `unwrap()` calls in types/mod.rs to return `Result`
