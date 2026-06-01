//! End-to-end tests for `tug run`, `tug test`, and `tug build`. These shell
//! out to a real `oxy` binary (the one Cargo builds for `oxy-cli`), which
//! must already exist at `<workspace>/target/debug/oxy`. Running the full
//! workspace test suite ensures it does.

#![allow(deprecated)] // assert_cmd::Command::cargo_bin

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use assert_cmd::Command;
use predicates::prelude::*;

static HOME_LOCK: Mutex<()> = Mutex::new(());

struct HomeGuard {
    dir: PathBuf,
    prev: Option<String>,
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl HomeGuard {
    fn new(label: &str) -> Self {
        let lock = HOME_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let prev = std::env::var("HOME").ok();
        let dir = std::env::temp_dir().join(format!(
            "tug-runner-{}-{label}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::create_dir_all(&dir).unwrap();
        // Safety: tests are serialized via HOME_LOCK mutex; temp dirs are
        // per-test and cleaned up in Drop. No concurrent access to HOME.
        unsafe {
            std::env::set_var("HOME", &dir);
        }
        Self {
            dir,
            prev,
            _lock: lock,
        }
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
        // Safety: restoring the previous HOME value. Tests are serialized
        // via HOME_LOCK mutex, so no concurrent access to the environment.
        unsafe {
            match &self.prev {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
        }
    }
}

/// Absolute path to the workspace-built `oxy` binary. Tests pass this to
/// `tug` via `TUG_OXY_PATH` so tug doesn't need to find `oxy` on `PATH`.
fn oxy_binary() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let ws = Path::new(manifest_dir).parent().unwrap().parent().unwrap();
    let bin_name = if cfg!(windows) { "oxy.exe" } else { "oxy" };
    let candidate = ws.join("target").join("debug").join(bin_name);
    if !candidate.exists() {
        // Build it on demand. Slow but reliable when running this test alone.
        let _ = std::process::Command::new("cargo")
            .args(["build", "-p", "oxy-cli"])
            .current_dir(ws)
            .status();
    }
    candidate
}

fn tug_cmd(cwd: &Path) -> Command {
    let mut c = Command::cargo_bin("tug").unwrap();
    c.current_dir(cwd);
    c.env("TUG_OXY_PATH", oxy_binary());
    c
}

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

// ---- run: no dependencies ----

#[test]
fn run_executes_project_main() {
    let g = HomeGuard::new("run-main");
    let proj = g.dir.join("proj");
    write(
        &proj.join("tug.toml"),
        "[package]\nname = \"myproj\"\nversion = \"0.1.0\"\n",
    );
    write(
        &proj.join("src/main.ox"),
        "fn main() { println(\"hello from tug run\"); }\n",
    );

    tug_cmd(&proj)
        .arg("run")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello from tug run"));
}

#[test]
fn run_passes_script_args_through() {
    let g = HomeGuard::new("run-args");
    let proj = g.dir.join("proj");
    write(
        &proj.join("tug.toml"),
        "[package]\nname = \"argproj\"\nversion = \"0.1.0\"\n",
    );
    // Oxy's std::env::args() returns Vec<String>; the test script prints them.
    write(
        &proj.join("src/main.ox"),
        "fn main() {\n    val args = std::env::args();\n    for a in args { println(\"arg:{}\", a); }\n}\n",
    );

    tug_cmd(&proj)
        .args(["run", "hello", "world"])
        .assert()
        .success()
        .stdout(predicate::str::contains("arg:hello"))
        .stdout(predicate::str::contains("arg:world"));
}

#[test]
fn run_errors_when_not_in_a_project() {
    let g = HomeGuard::new("run-noproj");
    let dir = g.dir.join("empty");
    std::fs::create_dir_all(&dir).unwrap();
    tug_cmd(&dir)
        .arg("run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("tug.toml"));
}

// ---- run: with an installed dependency ----

#[test]
fn run_resolves_dependency_through_extern() {
    let g = HomeGuard::new("run-extern");

    // 1) Install a local package into ~/.oxy/packages/greet/.
    let pkg_src = g.dir.join("src-greet");
    write(
        &pkg_src.join("tug.toml"),
        "[package]\nname = \"greet\"\nversion = \"0.1.0\"\n",
    );
    write(
        &pkg_src.join("src/lib.ox"),
        "pub fn hi() { println(\"hi from greet pkg\"); }\n",
    );
    oxy_tug::install::install_from_path(&pkg_src).unwrap();

    // 2) Create a project that depends on it.
    let proj = g.dir.join("proj");
    write(
        &proj.join("tug.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ngreet = { path = \"../src-greet\" }\n",
    );
    write(
        &proj.join("src/main.ox"),
        "mod greet;\nfn main() { greet::hi(); }\n",
    );

    // 3) `tug run` should thread the extern through to oxy.
    tug_cmd(&proj)
        .arg("run")
        .assert()
        .success()
        .stdout(predicate::str::contains("hi from greet pkg"));
}

#[test]
fn run_reports_missing_dependency() {
    let g = HomeGuard::new("run-missing-dep");
    let proj = g.dir.join("proj");
    write(
        &proj.join("tug.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nnotinstalled = \"1.0.0\"\n",
    );
    write(&proj.join("src/main.ox"), "fn main() {}\n");

    tug_cmd(&proj)
        .arg("run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("notinstalled"));
}

// ---- test ----

#[test]
fn test_runs_test_functions() {
    let g = HomeGuard::new("test-runs");
    let proj = g.dir.join("proj");
    write(
        &proj.join("tug.toml"),
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\n",
    );
    // The CLI's `test` subcommand runs all #[test] fns.
    write(
        &proj.join("src/main.ox"),
        "fn main() {}\n#[test]\nfn one_plus_one() { assert_eq(1 + 1, 2); }\n",
    );

    tug_cmd(&proj)
        .arg("test")
        .assert()
        .success()
        .stdout(predicate::str::contains("one_plus_one"));
}

// ---- build ----

#[test]
fn build_succeeds_on_compilable_project() {
    let g = HomeGuard::new("build-ok");
    let proj = g.dir.join("proj");
    write(
        &proj.join("tug.toml"),
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\n",
    );
    write(&proj.join("src/main.ox"), "fn main() {}\n");

    tug_cmd(&proj).arg("build").assert().success();
}

#[test]
fn build_fails_on_invalid_oxy_source() {
    let g = HomeGuard::new("build-fail");
    let proj = g.dir.join("proj");
    write(
        &proj.join("tug.toml"),
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\n",
    );
    // Garbage that should fail to parse.
    write(&proj.join("src/main.ox"), "this is not valid oxy ;;\n");

    tug_cmd(&proj).arg("build").assert().failure();
}
