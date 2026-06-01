//! WASM-execution smoke test for the playground entry points.
//!
//! Why this exists: the IR interpreter is the *only* backend that runs in the
//! browser (Cranelift can't emit wasm), yet every other test runs the
//! interpreter compiled for the **native host**. On 64-bit native `usize == i64`,
//! so the interpreter's FFI dispatch — which calls the shared `oxy_*` functions
//! through transmuted pointers — happens to line up. On `wasm32` `usize == i32`,
//! and a width mismatch traps at runtime with "indirect call signature
//! mismatch". That class of bug is invisible to native tests (including the
//! jit↔interp parity test, which runs both backends on the host) and only
//! surfaces when the wasm module actually executes.
//!
//! These tests run under `wasm-pack test --node`, i.e. the real wasm32 module
//! executing in a wasm runtime. Each program exercises a distinct slice of the
//! `oxy_*` FFI surface (the args of which were the source of the mismatch), so a
//! signature/ABI regression on any of them fails here instead of in production.
//!
//!     wasm-pack test --node playground/wasm

#![cfg(target_arch = "wasm32")]

use oxy_wasm::{run_oxy, run_tests_oxy};
use wasm_bindgen_test::*;

/// Run `main` and assert the captured output, surfacing any trap/error verbatim.
fn check(src: &str, expected: &str) {
    let got = run_oxy(src);
    assert_eq!(got, expected, "program:\n{src}\n-- got: {got:?}");
}

// oxy_print/println_val (usize count), oxy_add — the most basic FFI path.
#[wasm_bindgen_test]
fn wasm_println_arithmetic() {
    check("fn main() { println(\"{}\", 1 + 2 * 3); }", "7\n");
}

// io::println path dispatch should use the same captured output sink as println.
#[wasm_bindgen_test]
fn wasm_io_println_captures_output() {
    check("fn main() { io::println(\"{}\", 42); }", "42\n");
}

// oxy_load_local / oxy_store_local / oxy_make_cell (usize slot indices).
#[wasm_bindgen_test]
fn wasm_locals_and_while() {
    check(
        "fn main() { var s = 0; var i = 0; while i < 5 { s = s + i; i = i + 1; } println(\"{}\", s); }",
        "10\n",
    );
}

// oxy_make_array + oxy_vec_index (usize count/index).
#[wasm_bindgen_test]
fn wasm_vec_index() {
    check(
        "fn main() { val v = [10, 20, 30]; println(\"{}\", v[1]); }",
        "20\n",
    );
}

// oxy_make_range (the `inclusive` flag) + for-loop iteration.
#[wasm_bindgen_test]
fn wasm_range_for_loop() {
    check(
        "fn main() { var s = 0; for i in 0..5 { s = s + i; } println(\"{}\", s); }",
        "10\n",
    );
}

// oxy_push_closure + oxy_call_closure + oxy_method_call driving a higher-order
// builtin through the interpreter closure-invoker hook — the path that the
// `i64` name-pointer params used to trap on.
#[wasm_bindgen_test]
fn wasm_higher_order_closure() {
    check(
        "fn main() { val v = [1, 2, 3]; val t: Int = v.map(|x| x * 2).sum(); println(\"{}\", t); }",
        "12\n",
    );
}

// oxy_struct_init + oxy_field_access + a user method via oxy_method_call.
#[wasm_bindgen_test]
fn wasm_struct_method() {
    check(
        "struct P { x: Int, y: Int }\nimpl P { fn sum(self) -> Int { self.x + self.y } }\nfn main() { val p = P { x: 3, y: 4 }; println(\"{}\", p.sum()); }",
        "7\n",
    );
}

// oxy_make_enum_variant + match (Option/Result combinators).
#[wasm_bindgen_test]
fn wasm_enum_and_match() {
    check(
        "fn half(n: Int) -> Result<Int, String> { if n % 2 == 0 { Ok(n / 2) } else { Err(\"odd\") } }\nfn main() { match half(10) { Ok(v) => println(\"ok {}\", v), Err(e) => println(\"err {}\", e) } }",
        "ok 5\n",
    );
}

// User-function calls (oxy_path_call_builtin) + recursion.
#[wasm_bindgen_test]
fn wasm_recursion() {
    check(
        "fn fib(n: Int) -> Int { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }\nfn main() { println(\"{}\", fib(10)); }",
        "55\n",
    );
}

// oxy_push_closure(is_async) → Future, oxy_spawn_ffi / oxy_await_ffi driven by
// the interpreter hook (eager run).
#[wasm_bindgen_test]
fn wasm_async_spawn_await() {
    check(
        "fn main() { val h = spawn(|| 21 * 2); println(\"{}\", h.await); }",
        "42\n",
    );
}

// String methods + f-string formatting (oxy_method_call on a String receiver,
// oxy_fstring_concat).
#[wasm_bindgen_test]
fn wasm_string_methods() {
    check(
        "fn main() { val s = \"hello\"; println(\"{} {}\", s.to_uppercase(), s.len()); }",
        "HELLO 5\n",
    );
}

// The #[test]-runner entry point (run_tests_oxy) must also execute on wasm.
#[wasm_bindgen_test]
fn wasm_run_tests_entry() {
    let json = run_tests_oxy("#[test]\nfn t_ok() { assert_eq(1 + 1, 2); }");
    assert!(
        json.contains("\"name\":\"t_ok\"") && json.contains("\"passed\":true"),
        "unexpected test JSON: {json}"
    );
}
