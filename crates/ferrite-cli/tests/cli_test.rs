use assert_cmd::Command;
use predicates::prelude::*;

fn ferrite_cmd() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("ferrite").unwrap()
}

#[test]
fn test_version_flag() {
    ferrite_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("Ferrite v"));
}

#[test]
fn test_version_short_flag() {
    ferrite_cmd()
        .arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("Ferrite v"));
}

#[test]
fn test_help_flag() {
    ferrite_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("run <file.fe>"))
        .stdout(predicate::str::contains("repl"));
}

#[test]
fn test_no_args_shows_help() {
    ferrite_cmd()
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_unknown_command() {
    ferrite_cmd()
        .arg("bogus")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown command"));
}
