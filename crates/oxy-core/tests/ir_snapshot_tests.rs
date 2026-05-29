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

    if actual != expected {
        panic!(
            "IR snapshot mismatch for '{name}':\n\n{diff}\nTo update: UPDATE_SNAPSHOTS=1 cargo test -p oxy-core ir_snapshot",
            diff = line_diff(&expected, &actual)
        );
    }
}

/// Minimal unified-diff-style comparison: shows ` ` (same), `-` (expected only),
/// `+` (actual only) with 2-line context around each changed hunk.
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
}
