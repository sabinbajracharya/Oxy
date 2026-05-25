use assert_cmd::Command;
use predicates::prelude::*;

fn oxy_cmd() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("oxy").unwrap()
}

#[test]
fn test_version_flag() {
    oxy_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("Oxy v"));
}

#[test]
fn test_version_short_flag() {
    oxy_cmd()
        .arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("Oxy v"));
}

#[test]
fn test_help_flag() {
    oxy_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("run"))
        .stdout(predicate::str::contains("<file.ox>"))
        .stdout(predicate::str::contains("repl"))
        .stdout(predicate::str::contains("--extern"))
        // Package management commands should no longer appear in oxy's help.
        .stdout(predicate::str::contains("install").not())
        .stdout(predicate::str::contains("uninstall").not())
        .stdout(predicate::str::contains("List installed").not());
}

#[test]
fn test_install_is_not_a_subcommand() {
    // Per the Rust split: `oxy` is just the compiler. Package management
    // lives in `tug`. `oxy install` must NOT be a recognized command.
    oxy_cmd()
        .arg("install")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unknown command"));
}

#[test]
fn test_extern_flag_resolves_module() {
    use std::io::Write;
    let dir = std::env::temp_dir().join(format!("oxy_extern_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();

    // Extern module file (lives anywhere — extern lookup doesn't need siblings).
    let extern_dir = dir.join("ext");
    std::fs::create_dir_all(&extern_dir).ok();
    let extern_file = extern_dir.join("greet.ox");
    let mut f = std::fs::File::create(&extern_file).unwrap();
    writeln!(f, "pub fn hello() {{ println!(\"hello from extern\"); }}").unwrap();

    // Main file in a different directory — `mod greet;` would normally fail,
    // but the --extern flag must inject it.
    let main_file = dir.join("main.ox");
    let mut m = std::fs::File::create(&main_file).unwrap();
    writeln!(m, "mod greet;\nfn main() {{ greet::hello(); }}").unwrap();

    oxy_cmd()
        .args([
            "run",
            "--extern",
            &format!("greet={}", extern_file.display()),
            main_file.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello from extern"));
}

#[test]
fn test_use_without_extern_or_sibling_fails() {
    // No sibling foo.ox and no --extern: must fail with a clear message.
    let dir = std::env::temp_dir().join(format!("oxy_noextern_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    let main_file = dir.join("main.ox");
    std::fs::write(&main_file, "mod no_such_mod;\nfn main() {}").unwrap();

    oxy_cmd()
        .args(["run", main_file.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("could not find module"));
}

#[test]
fn test_extern_invalid_format() {
    oxy_cmd()
        .args(["run", "--extern", "missing-equals", "ignored.ox"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("--extern expects name=path"));
}

#[test]
fn test_extern_requires_argument() {
    oxy_cmd()
        .args(["run", "--extern"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("--extern requires a name=path"));
}

#[test]
fn test_no_args_shows_help() {
    oxy_cmd()
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_unknown_command() {
    oxy_cmd()
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

    oxy_cmd()
        .args(["run", hello.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello, Oxy!"));
}

#[test]
fn test_run_missing_file() {
    oxy_cmd()
        .args(["run", "nonexistent.ox"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("could not read file"));
}

#[test]
fn test_run_no_main() {
    // Create a temp file with no main function
    let dir = std::env::temp_dir().join("oxy_test");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("no_main.ox");
    std::fs::write(&path, "fn foo() {}").unwrap();

    oxy_cmd()
        .args(["run", path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no `main` function"));
}
