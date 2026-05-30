//! Parity test: the Cranelift JIT (native backend) vs the portable IR
//! interpreter (the wasm/browser backend) must agree on every program.
//!
//! Both backends consume the identical register IR and the same shared `oxy_*`
//! runtime, so any feature that works on one should work on the other. This test
//! runs the whole `examples/features/**` corpus through *both* backends and
//! asserts their per-test pass/fail results match.
//!
//! It is the feature-parity command from CLAUDE.md "Two execution backends":
//!
//!     cargo test -p oxy-core --test jit_interp_parity
//!
//! ## Divergences
//!
//! There should be none. Both backends share the same register IR and the same
//! `oxy_*` runtime; the only structural difference is who runs a *callee*. Where
//! the JIT invokes a compiled function through its `fn_table` of native
//! pointers, the interpreter (whose `fn_table` is empty) interprets it — for
//! direct calls by intercepting at the IR level, and for callees reached from
//! inside the shared runtime (higher-order built-ins' closures, async task and
//! future bodies, user `Display::fmt`) via the thread-local closure-invoker hook
//! the interpreter installs (`ffi::set_interp_invoke`; see `vm/interp.rs`
//! `install_invoker`). So async (`spawn`/`await`/`sleep`/`select`), the
//! higher-order built-ins (`map`/`filter`/`fold`/`sort_by`/`for_each`/
//! Option·Result combinators), and `std::process::spawn`'s per-line callback all
//! run on both backends and must agree.
//!
//! Any divergence is therefore a real regression and fails the test.
//!
//! The `INTERP_UNSUPPORTED_MARKER` substring is still recognized below so that
//! if a future feature is *deliberately* marked unsupported on the interpreter
//! (via `unsupported_on_wasm!`), it is classified as an expected deferral rather
//! than a regression — but nothing is marked today.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use oxy_core::vm::{run_tests_interp_with_options, run_tests_jit_with_options, TestResult};

/// Stable substring of `INTERP_UNSUPPORTED_MARKER` (see `vm/jit/ffi.rs`). A
/// divergence whose interpreter error contains this is a deliberately
/// unsupported feature, not a regression. Nothing is marked unsupported today.
const UNSUPPORTED_MARKER: &str = "not supported by the Oxy IR interpreter";

/// `(file_stem, test_name)` divergences that are expected but do NOT carry the
/// marker. Empty: the FFI→interpreter closure-invoker hook closed the former
/// gaps (higher-order built-ins, async eager-runs, `std::process::spawn`
/// streaming). The staleness assertion below keeps this honest — re-add an entry
/// only alongside a documented, genuinely-diverging case.
const PENDING_CLOSURE_INVOKER: &[(&str, &str)] = &[];

fn visit_ox_files(dir: &Path, files: &mut Vec<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        paths.sort_by_key(|e| e.file_name());
        for entry in paths {
            let path = entry.path();
            if path.is_dir() {
                visit_ox_files(&path, files);
            } else if path.extension().is_some_and(|ext| ext == "ox") {
                files.push(path.to_string_lossy().to_string());
            }
        }
    }
}

fn pass_map(results: &[TestResult]) -> HashMap<&str, &TestResult> {
    results.iter().map(|r| (r.name.as_str(), r)).collect()
}

#[test]
fn jit_interp_parity() {
    let features_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join("features");

    let mut ox_files = Vec::new();
    visit_ox_files(&features_dir, &mut ox_files);
    assert!(
        !ox_files.is_empty(),
        "no .ox corpus found under {features_dir:?}"
    );

    let mut compared = 0; // (file, test) pairs whose results we compared
    let mut at_parity = 0;
    let mut deferred = 0; // expected, marker-tagged divergences
    let mut pending_hit = std::collections::HashSet::new(); // PENDING entries actually seen diverging
    let mut regressions: Vec<String> = Vec::new();

    for path_str in &ox_files {
        let source = fs::read_to_string(path_str).expect("read .ox");
        let stem = Path::new(path_str)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let jit = run_tests_jit_with_options(path_str, &source, HashMap::new());
        let interp = run_tests_interp_with_options(path_str, &source, HashMap::new());

        let (jr, ir) = match (jit, interp) {
            (Ok(jr), Ok(ir)) => (jr, ir),
            // A whole-program compile/typecheck outcome that differs between
            // backends is itself a regression (both share the same front-end).
            (j, i) => {
                regressions.push(format!(
                    "  {stem}: backend-level result mismatch — jit={:?} interp={:?}",
                    j.err().map(|e| e.to_string()),
                    i.err().map(|e| e.to_string())
                ));
                continue;
            }
        };

        let jmap = pass_map(&jr);
        let imap = pass_map(&ir);

        for (name, jres) in &jmap {
            compared += 1;
            let Some(ires) = imap.get(name) else {
                regressions.push(format!(
                    "  {stem}::{name}: present in JIT, absent in interpreter"
                ));
                continue;
            };
            if jres.passed == ires.passed {
                at_parity += 1;
                continue;
            }

            // Divergence. Classify it.
            let interp_err = ires.error.as_deref().unwrap_or("");
            let is_marked = interp_err.contains(UNSUPPORTED_MARKER);
            let is_pending = PENDING_CLOSURE_INVOKER.contains(&(stem.as_str(), name));

            // Only a JIT-pass / interpreter-fail divergence is "deferrable". If
            // the interpreter PASSES something the JIT fails, that's never
            // expected — flag it regardless of markers.
            let interp_only_failure = jres.passed && !ires.passed;

            if interp_only_failure && is_marked {
                deferred += 1;
            } else if interp_only_failure && is_pending {
                pending_hit.insert((stem.clone(), name.to_string()));
            } else {
                regressions.push(format!(
                    "  {stem}::{name}: jit_pass={} interp_pass={} interp_err={interp_err:?}",
                    jres.passed, ires.passed
                ));
            }
        }
    }

    // No stale entries: every documented pending divergence must still occur,
    // otherwise the entry should be deleted (the gap was closed).
    let stale: Vec<_> = PENDING_CLOSURE_INVOKER
        .iter()
        .filter(|(s, n)| !pending_hit.contains(&(s.to_string(), n.to_string())))
        .collect();

    eprintln!(
        "jit↔interp parity: {at_parity} at parity, {deferred} deferred (marked unsupported), \
         {} pending-closure-invoker, across {} corpus files ({compared} test comparisons)",
        pending_hit.len(),
        ox_files.len()
    );

    assert!(
        stale.is_empty(),
        "PENDING_CLOSURE_INVOKER entries no longer diverge (remove them): {stale:?}"
    );
    assert!(
        regressions.is_empty(),
        "\nJIT↔interpreter parity regressions ({} unexplained divergences):\n{}",
        regressions.len(),
        regressions.join("\n")
    );
}
