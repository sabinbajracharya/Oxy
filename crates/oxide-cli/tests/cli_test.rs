use assert_cmd::Command;
use predicates::prelude::*;

fn oxide_cmd() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("oxide").unwrap()
}

#[test]
fn test_version_flag() {
    oxide_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("Oxide v"));
}

#[test]
fn test_version_short_flag() {
    oxide_cmd()
        .arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("Oxide v"));
}

#[test]
fn test_help_flag() {
    oxide_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("run <file.ox>"))
        .stdout(predicate::str::contains("repl"));
}

#[test]
fn test_no_args_shows_help() {
    oxide_cmd()
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_unknown_command() {
    oxide_cmd()
        .arg("bogus")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown command"));
}

#[test]
fn test_run_hello() {
    // CWD for cargo tests is the crate root, so use CARGO_MANIFEST_DIR to find workspace
    let manifest = env!("CARGO_MANIFEST_DIR");
    let workspace = std::path::Path::new(manifest)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let hello = workspace.join("examples/hello.ox");

    oxide_cmd()
        .args(["run", hello.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello, Oxide!"));
}

#[test]
fn test_run_missing_file() {
    oxide_cmd()
        .args(["run", "nonexistent.ox"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("could not read file"));
}

#[test]
fn test_run_no_main() {
    // Create a temp file with no main function
    let dir = std::env::temp_dir().join("oxide_test");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("no_main.ox");
    std::fs::write(&path, "fn foo() {}").unwrap();

    oxide_cmd()
        .args(["run", path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no `main` function"));
}
