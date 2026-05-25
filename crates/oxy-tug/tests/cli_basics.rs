#![allow(deprecated)] // assert_cmd::Command::cargo_bin still works fine for our setup.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn version_flag_prints_version() {
    Command::cargo_bin("tug")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("tug v"));
}

#[test]
fn short_version_flag_prints_version() {
    Command::cargo_bin("tug")
        .unwrap()
        .arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("tug v"));
}

#[test]
fn help_flag_prints_usage() {
    Command::cargo_bin("tug")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("new <name>"))
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("remove"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("install"))
        .stdout(predicate::str::contains("uninstall"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("build"))
        .stdout(predicate::str::contains("run"))
        .stdout(predicate::str::contains("test"));
}

#[test]
fn no_args_prints_help() {
    Command::cargo_bin("tug")
        .unwrap()
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn unknown_command_errors() {
    Command::cargo_bin("tug")
        .unwrap()
        .arg("frobnicate")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unknown command"))
        .stderr(predicate::str::contains("frobnicate"));
}
