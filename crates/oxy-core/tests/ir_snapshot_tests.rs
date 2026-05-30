//! IR snapshot tests — golden-file tests for the canonical Register IR serialization.
//!
//! Each test compiles a small Oxy snippet to IR (no codegen / JIT), serializes it
//! with the canonical pretty-printer from `IR_SNAPSHOT_FORMAT.md`, and compares
//! the result against a golden file stored in `tests/snapshots/ir/<category>/<name>.txt`.
//!
//! ## Workflow
//!
//! First run (no golden file):   auto-creates the file, then panics so you can
//!                                inspect the output before committing it.
//! Normal run:                    compares; panics with a line diff on mismatch.
//! Update mode:                   `UPDATE_SNAPSHOTS=1 cargo test -p oxy-core ir_snapshot`
//!                                overwrites all golden files silently.

use std::fs;
use std::path::{Path, PathBuf};

// ── Infrastructure ─────────────────────────────────────────────────────────

fn snapshot_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join("ir")
        .join(name)
        .with_extension("txt")
}

/// Assert that the IR generated for `source` matches the golden file `name`.
/// `source` is wrapped in `fn main() { … }` before compilation.
fn assert_ir_snapshot(name: &str, source: &str) {
    assert_ir_snapshot_raw(name, &format!("fn main() {{\n{source}\n}}"));
}

/// Like `assert_ir_snapshot` but does NOT wrap the source in `fn main()`.
/// Use this when the source already contains top-level function definitions.
fn assert_ir_snapshot_raw(name: &str, source: &str) {
    let actual =
        oxy_core::vm::gen_ir_snapshot(source).unwrap_or_else(|e| format!("<ir gen error: {e}>\n"));

    let path = snapshot_path(name);

    if std::env::var("UPDATE_SNAPSHOTS").is_ok() {
        fs::create_dir_all(path.parent().unwrap()).expect("failed to create snapshot directory");
        fs::write(&path, &actual).expect("failed to write snapshot");
        return;
    }

    if !path.exists() {
        fs::create_dir_all(path.parent().unwrap()).expect("failed to create snapshot directory");
        fs::write(&path, &actual).expect("failed to write snapshot");
        panic!(
            "New IR snapshot written: {}\nRun the test again to verify.",
            path.display()
        );
    }

    let expected = fs::read_to_string(&path).expect("failed to read snapshot file");

    // The IR pretty-printer only ever emits '\n'. A Windows checkout
    // (core.autocrlf) may store the golden file with CRLF, so normalize both
    // sides before comparing — a stray '\r' is never meaningful IR output, and
    // comparing raw bytes would fail every test on Windows with no visible diff.
    if normalize_newlines(&actual) != normalize_newlines(&expected) {
        panic!(
            "IR snapshot mismatch for '{name}':\n\n{diff}\nTo update: UPDATE_SNAPSHOTS=1 cargo test -p oxy-core ir_snapshot",
            diff = line_diff(&expected, &actual)
        );
    }
}

/// Normalize CRLF / lone CR to LF so snapshot comparison is independent of how
/// Git checked out the golden file (Windows `core.autocrlf` rewrites LF→CRLF).
fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Minimal unified-diff-style comparison: shows ` ` (same), `-` (expected only),
/// `+` (actual only) with 2-line context around each changed hunk.
// Index-based loops are clearer here: line 92 writes a ±2 sub-range into a
// parallel bool mask, and line 99 walks indices in lockstep across three slices.
#[allow(clippy::needless_range_loop)]
fn line_diff(expected: &str, actual: &str) -> String {
    let exp: Vec<&str> = expected.lines().collect();
    let act: Vec<&str> = actual.lines().collect();
    let max = exp.len().max(act.len());

    // Collect changed line indices
    let mut changed: Vec<usize> = Vec::new();
    for i in 0..max {
        let e = exp.get(i).copied().unwrap_or("");
        let a = act.get(i).copied().unwrap_or("");
        if e != a {
            changed.push(i);
        }
    }

    if changed.is_empty() {
        return "(no line differences — possible trailing whitespace or newline issue)".to_string();
    }

    // Expand changed set with ±2 context lines, then emit
    let mut in_context: Vec<bool> = vec![false; max];
    for &ci in &changed {
        for j in ci.saturating_sub(2)..=(ci + 2).min(max.saturating_sub(1)) {
            in_context[j] = true;
        }
    }

    let mut out = String::new();
    let mut last_printed: Option<usize> = None;
    for i in 0..max {
        if !in_context[i] {
            continue;
        }
        if let Some(lp) = last_printed {
            if i > lp + 1 {
                out.push_str("...\n");
            }
        }
        last_printed = Some(i);

        let e = exp.get(i).copied().unwrap_or("");
        let a = act.get(i).copied().unwrap_or("");
        if e == a {
            out.push_str(&format!("  {}\n", e));
        } else {
            if !e.is_empty() {
                out.push_str(&format!("- {}\n", e));
            }
            if !a.is_empty() {
                out.push_str(&format!("+ {}\n", a));
            }
        }
    }
    out
}

// ── Expressions ────────────────────────────────────────────────────────────

mod expressions {
    use super::assert_ir_snapshot;

    #[test]
    fn const_int() {
        assert_ir_snapshot("expressions/const_int", "let x = 42;");
    }

    #[test]
    fn const_float() {
        assert_ir_snapshot("expressions/const_float", "let x = 3.14;");
    }

    #[test]
    fn const_bool() {
        assert_ir_snapshot("expressions/const_bool", "let x = true;");
    }

    #[test]
    fn const_str() {
        assert_ir_snapshot("expressions/const_str", r#"let x = "hello";"#);
    }

    #[test]
    fn const_unit() {
        assert_ir_snapshot("expressions/const_unit", "let x = ();");
    }

    #[test]
    fn arithmetic_add() {
        assert_ir_snapshot("expressions/arithmetic_add", "let x = 1 + 2;");
    }

    #[test]
    fn arithmetic_precedence() {
        assert_ir_snapshot("expressions/arithmetic_precedence", "let x = 1 + 2 * 3;");
    }

    #[test]
    fn arithmetic_sub_div() {
        assert_ir_snapshot("expressions/arithmetic_sub_div", "let x = 10 - 3 / 2;");
    }

    #[test]
    fn comparison_lt() {
        assert_ir_snapshot("expressions/comparison_lt", "let x = 1 < 2;");
    }

    #[test]
    fn comparison_eq() {
        assert_ir_snapshot("expressions/comparison_eq", "let x = 1 == 1;");
    }

    #[test]
    fn comparison_neq() {
        assert_ir_snapshot("expressions/comparison_neq", "let x = 1 != 2;");
    }

    #[test]
    fn bitwise_and() {
        assert_ir_snapshot("expressions/bitwise_and", "let x = 6 & 3;");
    }

    #[test]
    fn bitwise_or() {
        assert_ir_snapshot("expressions/bitwise_or", "let x = 4 | 2;");
    }

    #[test]
    fn unary_neg() {
        assert_ir_snapshot("expressions/unary_neg", "let x = -5;");
    }

    #[test]
    fn unary_not() {
        assert_ir_snapshot("expressions/unary_not", "let x = !true;");
    }

    #[test]
    fn const_char() {
        assert_ir_snapshot("expressions/const_char", "let x = 'a';");
    }

    #[test]
    fn const_negative_int() {
        assert_ir_snapshot("expressions/const_negative_int", "let x = -100;");
    }

    #[test]
    fn const_string_escapes() {
        assert_ir_snapshot(
            "expressions/const_string_escapes",
            "let x = \"hi\\nthere\";",
        );
    }

    #[test]
    fn arithmetic_mul() {
        assert_ir_snapshot("expressions/arithmetic_mul", "let x = 3 * 4;");
    }

    #[test]
    fn arithmetic_rem() {
        assert_ir_snapshot("expressions/arithmetic_rem", "let x = 7 % 3;");
    }

    #[test]
    fn arithmetic_float() {
        assert_ir_snapshot("expressions/arithmetic_float", "let x = 1.5 + 2.5;");
    }

    #[test]
    fn precedence_paren_override() {
        assert_ir_snapshot(
            "expressions/precedence_paren_override",
            "let x = (1 + 2) * 3;",
        );
    }

    #[test]
    fn precedence_left_assoc() {
        assert_ir_snapshot("expressions/precedence_left_assoc", "let x = 1 - 2 - 3;");
    }

    #[test]
    fn precedence_mixed_cmp_arith() {
        assert_ir_snapshot(
            "expressions/precedence_mixed_cmp_arith",
            "let x = 1 + 2 < 3 * 4;",
        );
    }

    #[test]
    fn unary_bitnot() {
        assert_ir_snapshot("expressions/unary_bitnot", "let y = 5;\nlet x = ~y;");
    }

    #[test]
    fn unary_double_neg() {
        assert_ir_snapshot(
            "expressions/unary_double_neg",
            "let mut y = 5;\nlet x = -(-y);",
        );
    }

    #[test]
    fn comparison_gt_le_ge() {
        assert_ir_snapshot(
            "expressions/comparison_gt_le_ge",
            "let a = 5;\nlet b = 3;\nlet r1 = a > b;\nlet r2 = a <= b;\nlet r3 = a >= b;",
        );
    }

    #[test]
    fn bitwise_xor_shift() {
        assert_ir_snapshot(
            "expressions/bitwise_xor_shift",
            "let a = 5;\nlet b = 3;\nlet r1 = a ^ b;\nlet r2 = a << 1;\nlet r3 = a >> 1;",
        );
    }
}

// ── Variables ──────────────────────────────────────────────────────────────

mod variables {
    use super::assert_ir_snapshot;

    #[test]
    fn let_simple() {
        assert_ir_snapshot("variables/let_simple", "let x = 10;");
    }

    #[test]
    fn let_with_type_ann() {
        assert_ir_snapshot("variables/let_with_type_ann", "let x: int = 10;");
    }

    #[test]
    fn let_mut_reassign() {
        assert_ir_snapshot("variables/let_mut_reassign", "let mut x = 1;\nx = 2;");
    }

    #[test]
    fn let_shadow() {
        assert_ir_snapshot("variables/let_shadow", "let x = 1;\nlet x = 2;");
    }

    #[test]
    fn multiple_locals() {
        assert_ir_snapshot(
            "variables/multiple_locals",
            "let a = 1;\nlet b = 2;\nlet c = a + b;",
        );
    }

    #[test]
    fn let_bool() {
        assert_ir_snapshot("variables/let_bool", "let flag = false;");
    }

    #[test]
    fn let_uninit_then_assign() {
        assert_ir_snapshot("variables/let_uninit_then_assign", "let mut x;\nx = 1;");
    }

    #[test]
    fn load_local_raw() {
        // Method call on a local variable uses load.local.raw (no Cell unwrap)
        // to keep mutation through the method call visible.
        assert_ir_snapshot(
            "variables/load_local_raw",
            "let mut s = \"hello\";\nlet n = s.len();",
        );
    }
}

// ── Control flow ───────────────────────────────────────────────────────────

mod control_flow {
    use super::assert_ir_snapshot;

    #[test]
    fn if_else_basic() {
        assert_ir_snapshot(
            "control_flow/if_else_basic",
            "let x = if true { 1 } else { 2 };",
        );
    }

    #[test]
    fn if_no_else() {
        assert_ir_snapshot(
            "control_flow/if_no_else",
            "let mut x = 0;\nif true {\n    x = 1;\n}",
        );
    }

    #[test]
    fn if_nested() {
        assert_ir_snapshot(
            "control_flow/if_nested",
            "let x = if true { if false { 1 } else { 2 } } else { 3 };",
        );
    }

    #[test]
    fn while_basic() {
        assert_ir_snapshot(
            "control_flow/while_basic",
            "let mut i = 0;\nwhile i < 3 {\n    i = i + 1;\n}",
        );
    }

    #[test]
    fn for_range() {
        assert_ir_snapshot(
            "control_flow/for_range",
            "let mut sum = 0;\nfor i in 0..5 {\n    sum = sum + i;\n}",
        );
    }

    #[test]
    fn loop_break() {
        assert_ir_snapshot(
            "control_flow/loop_break",
            "let x = loop {\n    break 42;\n};",
        );
    }

    #[test]
    fn match_simple() {
        assert_ir_snapshot(
            "control_flow/match_simple",
            "let x = 2;\nlet y = match x {\n    1 => 10,\n    2 => 20,\n    _ => 0,\n};",
        );
    }

    #[test]
    fn if_chain() {
        assert_ir_snapshot(
            "control_flow/if_chain",
            "let x = 5;\nlet y = if x < 3 { 1 } else if x < 7 { 2 } else { 3 };",
        );
    }

    #[test]
    fn if_as_expression() {
        // Variable condition — shows the full Load → Branch → phi path.
        assert_ir_snapshot(
            "control_flow/if_as_expression",
            "let mut c = true;\nlet y = if c { 1 } else { 2 };",
        );
    }

    #[test]
    fn loop_continue() {
        // continue jumps back to loop header; break jumps to exit.
        // Both should produce Jump terminators to different targets.
        assert_ir_snapshot(
            "control_flow/loop_continue",
            "let mut i = 0;\nloop {\n    i = i + 1;\n    if i < 3 { continue; }\n    break;\n}",
        );
    }

    #[test]
    fn while_nested_break() {
        // Inner break resolves to the innermost loop's exit block, not the outer.
        assert_ir_snapshot(
            "control_flow/while_nested_break",
            "let mut i = 0;\nwhile i < 3 {\n    let mut j = 0;\n    while j < 3 {\n        if j == 1 { break; }\n        j = j + 1;\n    }\n    i = i + 1;\n}",
        );
    }

    #[test]
    fn for_over_vec() {
        // Array literal → oxy_make_iter (no oxy_make_range step, unlike for_range).
        assert_ir_snapshot(
            "control_flow/for_over_vec",
            "let v = [1, 2, 3];\nlet mut sum = 0;\nfor x in v {\n    sum = sum + x;\n}",
        );
    }
}

// ── Functions ──────────────────────────────────────────────────────────────

mod functions {
    use super::{assert_ir_snapshot, assert_ir_snapshot_raw};

    #[test]
    fn fn_no_params() {
        assert_ir_snapshot_raw(
            "functions/fn_no_params",
            "fn greet() -> int {\n    42\n}\nfn main() {}",
        );
    }

    #[test]
    fn fn_with_params() {
        assert_ir_snapshot_raw(
            "functions/fn_with_params",
            "fn add(a: int, b: int) -> int {\n    a + b\n}\nfn main() {}",
        );
    }

    #[test]
    fn fn_explicit_return() {
        assert_ir_snapshot_raw(
            "functions/fn_explicit_return",
            "fn abs(x: int) -> int {\n    if x < 0 { return -x; }\n    x\n}\nfn main() {}",
        );
    }

    #[test]
    fn fn_multiple_returns() {
        assert_ir_snapshot_raw(
            "functions/fn_multiple_returns",
            r#"fn classify(x: int) -> String {
    if x < 0 { return "negative"; }
    if x == 0 { return "zero"; }
    "positive"
}
fn main() {}"#,
        );
    }

    #[test]
    fn closure_basic() {
        assert_ir_snapshot(
            "functions/closure_basic",
            "let add = |a: int, b: int| a + b;\nlet r = add(1, 2);",
        );
    }

    #[test]
    fn fn_recursive() {
        assert_ir_snapshot_raw(
            "functions/fn_recursive",
            "fn fact(n: int) -> int {\n    if n <= 1 { return 1; }\n    n * fact(n - 1)\n}\nfn main() {}",
        );
    }

    #[test]
    fn closure_with_captures() {
        // The generated IrFunction must have a non-empty `captures:` header line.
        assert_ir_snapshot(
            "functions/closure_with_captures",
            "let y = 10;\nlet add_y = |x: int| x + y;\nlet r = add_y(5);",
        );
    }

    #[test]
    fn fn_async() {
        assert_ir_snapshot_raw(
            "functions/fn_async",
            "async fn delay() -> int { 42 }\nfn main() {}",
        );
    }

    #[test]
    fn fn_unit_return() {
        // Explicit `-> ()` return type annotation renders as `-> ()` in the header.
        assert_ir_snapshot_raw(
            "functions/fn_unit_return",
            "fn noop() -> () { }\nfn main() {}",
        );
    }
}

// ── Calls ──────────────────────────────────────────────────────────────────

mod calls {
    #[allow(unused_imports)]
    use super::{assert_ir_snapshot, assert_ir_snapshot_raw};

    #[test]
    fn call_user_fn() {
        assert_ir_snapshot_raw(
            "calls/call_user_fn",
            "fn double(x: int) -> int { x * 2 }\nfn main() {\n    let r = double(5);\n}",
        );
    }

    #[test]
    fn call_with_args() {
        assert_ir_snapshot_raw(
            "calls/call_with_args",
            "fn add(a: int, b: int) -> int { a + b }\nfn main() {\n    let r = add(3, 4);\n}",
        );
    }

    #[test]
    fn method_call_string_len() {
        assert_ir_snapshot(
            "calls/method_call_string_len",
            "let s = \"hello\";\nlet n = s.len();",
        );
    }

    #[test]
    fn method_call_vec_push() {
        assert_ir_snapshot(
            "calls/method_call_vec_push",
            "let mut v: Vec<int> = Vec::new();\nv.push(1);\nv.push(2);",
        );
    }

    #[test]
    fn call_println() {
        assert_ir_snapshot("calls/call_println", "println(\"hello\");");
    }

    #[test]
    fn call_nested_args() {
        // Arg registers: double(1) and double(2) are fully resolved before add() is called.
        assert_ir_snapshot_raw(
            "calls/call_nested_args",
            "fn double(x: int) -> int { x * 2 }\nfn add(a: int, b: int) -> int { a + b }\nfn main() {\n    let r = add(double(1), double(2));\n}",
        );
    }

    #[test]
    fn call_enum_variant_ctor() {
        // Variant constructor call routes to oxy_make_enum_variant, not oxy_call.
        assert_ir_snapshot_raw(
            "calls/call_enum_variant_ctor",
            "enum Direction { North, South }\nfn main() {\n    let d = Direction::North;\n}",
        );
    }

    #[test]
    fn method_chain() {
        // Receiver of the second call is the result register of the first, not a local slot.
        // So the second call's object goes through gen_expr, not LoadLocalRaw.
        assert_ir_snapshot(
            "calls/method_chain",
            "let s = \"  hello  \";\nlet n = s.trim().len();",
        );
    }
}

// ── Assignment ─────────────────────────────────────────────────────────────

mod assignment {
    use super::{assert_ir_snapshot, assert_ir_snapshot_raw};

    #[test]
    fn assign_multi_target() {
        // Two separate assignments from the same source value:
        // both emit StoreLocal, each to their own slot.
        // (Oxy `=` returns `()` like Rust, so `a = b = c` is not valid.)
        assert_ir_snapshot(
            "assignment/assign_multi_target",
            "let mut a = 0;\nlet mut b = 0;\nlet c = 5;\na = c;\nb = c;",
        );
    }

    #[test]
    fn assign_field() {
        // Field assignment lowers to oxy_field_store, not StoreLocal.
        assert_ir_snapshot_raw(
            "assignment/assign_field",
            "struct Point { x: int, y: int }\nfn main() {\n    let mut p = Point { x: 1, y: 2 };\n    p.x = 10;\n}",
        );
    }

    #[test]
    fn assign_index() {
        // Index assignment lowers to oxy_vec_index_store.
        assert_ir_snapshot(
            "assignment/assign_index",
            "let mut v = [1, 2, 3];\nv[0] = 99;",
        );
    }

    #[test]
    fn compound_assign_add() {
        // `x += n` lowers to Add(load(x), n) then StoreLocal into x's slot.
        assert_ir_snapshot("assignment/compound_assign_add", "let mut x = 5;\nx += 3;");
    }

    #[test]
    fn compound_assign_eval_order() {
        // For `x += f()`, ir_gen evaluates f() (the RHS) BEFORE loading x (the LHS).
        // This is ir_gen/mod.rs:1207-1208: val_reg = gen_expr(value) then target_reg = gen_expr(target).
        assert_ir_snapshot_raw(
            "assignment/compound_assign_eval_order",
            "fn get() -> int { 42 }\nfn main() {\n    let mut x = 0;\n    x += get();\n}",
        );
    }
}

// ── Returns ────────────────────────────────────────────────────────────────

mod returns {
    use super::assert_ir_snapshot_raw;

    #[test]
    fn return_implicit_tail() {
        // Last expression without semicolon: lowers to Return, same as explicit `return`.
        assert_ir_snapshot_raw(
            "returns/return_implicit_tail",
            "fn sum(a: int, b: int) -> int { a + b }\nfn main() {}",
        );
    }

    #[test]
    fn return_early_in_loop() {
        // `return` inside a while loop body: the block terminates with Return,
        // no back-edge Jump follows (the block is already terminated).
        assert_ir_snapshot_raw(
            "returns/return_early_in_loop",
            "fn find(n: int) -> int {\n    let mut i = 0;\n    while i < n {\n        if i == 3 { return i; }\n        i = i + 1;\n    }\n    -1\n}\nfn main() {}",
        );
    }

    #[test]
    fn return_unit_bare() {
        // Bare `return;` lowers to ConstUnit + Return(that reg).
        assert_ir_snapshot_raw(
            "returns/return_unit_bare",
            "fn noop() {\n    return;\n}\nfn main() {}",
        );
    }
}

// ── Edge cases ─────────────────────────────────────────────────────────────

mod edge_cases {
    use super::{assert_ir_snapshot, assert_ir_snapshot_raw};

    #[test]
    fn nested_arith_calls() {
        // Interleaved FFI call results and inline arithmetic:
        // inc(x) result reg + dbl(x) result reg * const.int 2.
        assert_ir_snapshot_raw(
            "edge_cases/nested_arith_calls",
            "fn inc(x: int) -> int { x + 1 }\nfn dbl(x: int) -> int { x * 2 }\nfn main() {\n    let x = 3;\n    let y = inc(x) + dbl(x) * 2;\n}",
        );
    }

    #[test]
    fn side_effect_order_in_args() {
        // f() and g() are both called before the add: left arg materialised first.
        assert_ir_snapshot_raw(
            "edge_cases/side_effect_order_in_args",
            "fn f() -> int { 1 }\nfn g() -> int { 2 }\nfn main() {\n    let x = f() + g();\n}",
        );
    }

    #[test]
    fn boolean_in_if_cond() {
        // `a && b` in an if condition: the And op feeds the Branch directly —
        // no short-circuit CFG splitting (both a and b are always evaluated).
        assert_ir_snapshot(
            "edge_cases/boolean_in_if_cond",
            "let a = true;\nlet b = false;\nlet mut x = 0;\nif a && b {\n    x = 1;\n}",
        );
    }

    #[test]
    fn block_expr_as_value() {
        // Block tail expression flows out as the block's value register.
        assert_ir_snapshot(
            "edge_cases/block_expr_as_value",
            "let x = { let t = 1; t + 1 };",
        );
    }

    #[test]
    fn nested_if_in_arith() {
        // `if` used inside an arithmetic expression: phi continuation block
        // sits between the branch arms and the add instruction.
        assert_ir_snapshot(
            "edge_cases/nested_if_in_arith",
            "let c = true;\nlet x = 1 + (if c { 2 } else { 3 });",
        );
    }
}

// ── Boolean logic (short-circuit) ──────────────────────────────────────────
//
// IMPORTANT: These snapshots document that `&&` and `||` are EAGER in Oxy —
// they lower to a single And/Or register op with no CFG short-circuit branch.
// Both operands always evaluate. If short-circuit semantics are ever added,
// these snapshots will fail and serve as the review gate for that change.

mod boolean {
    use super::{assert_ir_snapshot, assert_ir_snapshot_raw};

    #[test]
    fn bool_and_eager() {
        // `a && b` → single `and vA, vB`. No Branch. Both sides evaluated.
        assert_ir_snapshot(
            "boolean/bool_and_eager",
            "let a = true;\nlet b = false;\nlet x = a && b;",
        );
    }

    #[test]
    fn bool_or_eager() {
        // `a || b` → single `or vA, vB`. No Branch.
        assert_ir_snapshot(
            "boolean/bool_or_eager",
            "let a = true;\nlet b = false;\nlet x = a || b;",
        );
    }

    #[test]
    fn bool_and_with_call() {
        // `a && f()` — f() is called unconditionally even when a is false.
        // The surprising case: no lazy evaluation in Oxy's &&.
        assert_ir_snapshot_raw(
            "boolean/bool_and_with_call",
            "fn f() -> bool { true }\nfn main() {\n    let a = true;\n    let x = a && f();\n}",
        );
    }

    #[test]
    fn bool_chained() {
        // `a && b || c` — precedence: && binds tighter, so `(a && b) || c`.
        // Two flat eager ops, zero Branch instructions in the single block.
        assert_ir_snapshot(
            "boolean/bool_chained",
            "let a = true;\nlet b = false;\nlet c = true;\nlet x = a && b || c;",
        );
    }
}
