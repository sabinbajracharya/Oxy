use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_version_flag() {
    Command::cargo_bin("ferrite")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("Ferrite v"));
}

#[test]
fn test_version_short_flag() {
    Command::cargo_bin("ferrite")
        .unwrap()
        .arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("Ferrite v"));
}

#[test]
fn test_help_flag() {
    Command::cargo_bin("ferrite")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("run <file.fe>"))
        .stdout(predicate::str::contains("repl"));
}

#[test]
fn test_no_args_shows_help() {
    Command::cargo_bin("ferrite")
        .unwrap()
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_unknown_command() {
    Command::cargo_bin("ferrite")
        .unwrap()
        .arg("bogus")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown command"));
}
