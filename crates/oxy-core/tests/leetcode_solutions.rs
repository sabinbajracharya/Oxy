//! Integration test: validates all LeetCode solutions in `examples/leetcode/`.
//!
//! Each `.ox` file contains `#[test]` functions. This test discovers them,
//! runs them through the interpreter's `run_tests()` harness, and asserts all pass.

use std::fs;
use std::path::Path;

use oxy_core::vm::run_tests;

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

#[test]
fn leetcode_solutions() {
    let leetcode_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates
        .unwrap()
        .parent() // repo root
        .unwrap()
        .join("examples")
        .join("leetcode");

    let mut ox_files = Vec::new();
    visit_ox_files(&leetcode_dir, &mut ox_files);

    let mut failures = Vec::new();
    let mut total = 0;
    let mut passed = 0;

    for path_str in &ox_files {
        let source = fs::read_to_string(path_str).expect("failed to read .ox file");
        let results = run_tests(path_str, &source)
            .unwrap_or_else(|e| panic!("failed to parse or type-check {path_str}: {e}"));
        let stem = Path::new(path_str)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();

        for result in &results {
            total += 1;
            if result.passed {
                passed += 1;
            } else {
                failures.push(format!(
                    "  FAIL {}::{} - {}",
                    stem,
                    result.name,
                    result.error.as_deref().unwrap_or("(no error)")
                ));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "\n{passed}/{total} leetcode tests passed\n\nFailures:\n{}",
            failures.join("\n")
        );
    }

    eprintln!("leetcode: {passed}/{total} tests passed across all solutions");
}
