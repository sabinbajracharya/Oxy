//! Tests for the compiler's `mod <name>;` resolution and the externs map.
//!
//! Mirrors `rustc`'s design: dependency resolution is driven by the caller
//! (typically `tug`), not by the compiler probing a global package directory.

use std::collections::HashMap;
use std::path::PathBuf;

use oxy_core::vm::{run_compiled_with_options, run_tests_with_options};

fn unique_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "oxy-core-extern-{}-{label}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `mod foo;` finds `./foo.ox` next to the source.
#[test]
fn mod_resolves_sibling_file() {
    let dir = unique_dir("sibling-file");
    std::fs::write(dir.join("greet.ox"), "pub fn yo() -> Int { 42 }\n").unwrap();
    let main = dir.join("main.ox");
    let source = "mod greet;\nfn main() -> Int { greet::yo() }\n";
    std::fs::write(&main, source).unwrap();

    let v = run_compiled_with_options(source, Some(main.to_str().unwrap()), HashMap::new())
        .expect("should resolve sibling module");
    assert_eq!(v.to_string(), "42");
}

/// `mod foo;` finds `./foo/mod.ox` next to the source.
#[test]
fn mod_resolves_sibling_mod_directory() {
    let dir = unique_dir("sibling-dir");
    std::fs::create_dir_all(dir.join("greet")).unwrap();
    std::fs::write(dir.join("greet/mod.ox"), "pub fn yo() -> Int { 7 }\n").unwrap();
    let main = dir.join("main.ox");
    let source = "mod greet;\nfn main() -> Int { greet::yo() }\n";
    std::fs::write(&main, source).unwrap();

    let v = run_compiled_with_options(source, Some(main.to_str().unwrap()), HashMap::new())
        .expect("should resolve mod.ox directory");
    assert_eq!(v.to_string(), "7");
}

/// `mod foo;` finds an extern-injected file even when no sibling exists.
#[test]
fn mod_resolves_via_externs() {
    let dir = unique_dir("ext");
    let ext_dir = unique_dir("ext-source");
    let ext_file = ext_dir.join("lib.ox");
    std::fs::write(&ext_file, "pub fn yo() -> Int { 99 }\n").unwrap();

    let main = dir.join("main.ox");
    let source = "mod greet;\nfn main() -> Int { greet::yo() }\n";
    std::fs::write(&main, source).unwrap();

    let mut externs = HashMap::new();
    externs.insert("greet".to_string(), ext_file);

    let v = run_compiled_with_options(source, Some(main.to_str().unwrap()), externs)
        .expect("extern should resolve");
    assert_eq!(v.to_string(), "99");
}

/// Extern takes precedence over a sibling file with the same name
/// (matches rustc's `--extern` semantics).
#[test]
fn externs_take_precedence_over_siblings() {
    let dir = unique_dir("precedence");
    std::fs::write(dir.join("greet.ox"), "pub fn yo() -> Int { 1 }\n").unwrap();

    let ext_dir = unique_dir("precedence-ext");
    let ext_file = ext_dir.join("real.ox");
    std::fs::write(&ext_file, "pub fn yo() -> Int { 2 }\n").unwrap();

    let main = dir.join("main.ox");
    let source = "mod greet;\nfn main() -> Int { greet::yo() }\n";
    std::fs::write(&main, source).unwrap();

    let mut externs = HashMap::new();
    externs.insert("greet".to_string(), ext_file);

    let v = run_compiled_with_options(source, Some(main.to_str().unwrap()), externs).unwrap();
    assert_eq!(v.to_string(), "2", "extern must win over sibling");
}

/// `mod foo;` with neither sibling nor extern fails with a clear message.
/// No global package directory fallback exists — confirms the Rust-style
/// split between compiler and package manager.
#[test]
fn missing_module_errors_with_extern_hint() {
    let dir = unique_dir("missing");
    let main = dir.join("main.ox");
    let source = "mod missing;\nfn main() -> Int { 0 }\n";
    std::fs::write(&main, source).unwrap();

    let err = run_compiled_with_options(source, Some(main.to_str().unwrap()), HashMap::new())
        .expect_err("missing module must error");
    let msg = format!("{err}");
    assert!(
        msg.contains("could not find module"),
        "expected 'could not find module', got: {msg}"
    );
    assert!(
        msg.contains("--extern"),
        "error must suggest --extern flag, got: {msg}"
    );
}

/// Extern pointing at a nonexistent path surfaces a useful error.
#[test]
fn extern_with_bad_path_errors() {
    let dir = unique_dir("bad-extern");
    let main = dir.join("main.ox");
    let source = "mod missing;\nfn main() -> Int { 0 }\n";
    std::fs::write(&main, source).unwrap();

    let mut externs = HashMap::new();
    externs.insert(
        "missing".to_string(),
        PathBuf::from("/definitely/does/not/exist.ox"),
    );

    let err = run_compiled_with_options(source, Some(main.to_str().unwrap()), externs)
        .expect_err("bad extern path must error");
    let msg = format!("{err}");
    assert!(
        msg.contains("extern module") || msg.contains("could not read"),
        "expected extern read error, got: {msg}"
    );
}

/// `run_tests_with_options` threads externs through to #[test] compilation too.
#[test]
fn run_tests_with_externs() {
    let dir = unique_dir("test-externs");
    let ext_dir = unique_dir("test-externs-ext");
    let ext_file = ext_dir.join("helper.ox");
    std::fs::write(&ext_file, "pub fn answer() -> Int { 42 }\n").unwrap();

    let main = dir.join("main.ox");
    let source = "mod helper;\n#[test]\nfn t() { let x = helper::answer(); }\n";
    std::fs::write(&main, source).unwrap();

    let mut externs = HashMap::new();
    externs.insert("helper".to_string(), ext_file);

    let results =
        run_tests_with_options(main.to_str().unwrap(), source, externs).expect("tests run");
    assert_eq!(results.len(), 1);
    assert!(
        results[0].passed,
        "test should pass: {:?}",
        results[0].error
    );
}
