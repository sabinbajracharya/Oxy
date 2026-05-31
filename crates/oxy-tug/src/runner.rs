//! Run / build / test orchestration: resolves dependencies and shells out to
//! the `oxy` compiler with the appropriate `--extern <name>=<path>` flags.
//!
//! The `oxy` compiler is located via, in order:
//! 1. `$TUG_OXY_PATH` — explicit override (used in tests and packaged dists)
//! 2. the `oxy` binary in the same directory as the running `tug` binary
//!    (the canonical install layout)
//! 3. `oxy` on `PATH`

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::install;
use crate::project::Project;
use crate::tug_err;
use crate::TugResult;

/// Resolve the entry-point path for every dependency listed in the project's
/// manifest. Returns the `name → path` map suitable for `oxy --extern`.
///
/// Missing installations are returned as an error listing the offending names
/// so users get one actionable message.
pub fn resolve_externs(project: &Project) -> TugResult<HashMap<String, PathBuf>> {
    let mut externs = HashMap::new();
    let mut missing = Vec::new();
    for dep in &project.manifest().dependencies {
        match install::find_installed_entry(&dep.name) {
            Some(p) => {
                externs.insert(dep.name.clone(), p);
            }
            None => missing.push(dep.name.clone()),
        }
    }
    if !missing.is_empty() {
        return Err(tug_err!(
            "missing installed package(s): {} — run `tug install` first",
            missing.join(", ")
        ));
    }
    Ok(externs)
}

/// Find the `oxy` binary path. See module docs for the lookup order.
pub fn locate_oxy() -> TugResult<PathBuf> {
    if let Ok(p) = std::env::var("TUG_OXY_PATH") {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Ok(path);
        }
        return Err(tug_err!(
            "TUG_OXY_PATH points to '{}' which is not a file",
            path.display()
        ));
    }
    if let Ok(self_exe) = std::env::current_exe() {
        if let Some(dir) = self_exe.parent() {
            let candidate = dir.join(if cfg!(windows) { "oxy.exe" } else { "oxy" });
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    // Last resort: rely on PATH lookup at exec time.
    Ok(PathBuf::from("oxy"))
}

/// Default entry-point file for a project, in priority order:
///   `src/main.ox`, then `src/lib.ox`.
pub fn project_entry(project: &Project) -> TugResult<PathBuf> {
    let src = project.root().join("src");
    let main = src.join("main.ox");
    if main.is_file() {
        return Ok(main);
    }
    let lib = src.join("lib.ox");
    if lib.is_file() {
        return Ok(lib);
    }
    Err(tug_err!(
        "no src/main.ox or src/lib.ox in '{}'",
        project.root().display()
    ))
}

/// Run the project (or a specific entry file) via `oxy run`, threading
/// resolved deps as `--extern` flags. Returns the child process exit code.
pub fn run_project(project: &Project, script_args: &[String]) -> TugResult<i32> {
    invoke_oxy(project, "run", script_args)
}

/// Run `oxy test` over the project's entry file (or `src/lib.ox` if present),
/// with the same extern map as `run`.
pub fn test_project(project: &Project) -> TugResult<i32> {
    invoke_oxy(project, "test", &[])
}

/// Type-check the entry file by compiling it via `oxy --dump-ir` and
/// discarding the output. This is the closest equivalent to `cargo build`
/// for a script-style language with no separate "object file" output.
pub fn build_project(project: &Project) -> TugResult<i32> {
    let externs = resolve_externs(project)?;
    let entry = project_entry(project)?;
    let oxy = locate_oxy()?;

    let mut cmd = Command::new(&oxy);
    cmd.arg("--dump-ir");
    cmd.arg(&entry);
    // --dump-ir does not accept --extern yet; this is acceptable for now
    // because dependency-using projects will use `tug run` / `tug test`.
    // Surface the externs anyway for diagnostic clarity if oxy adds support.
    let _ = externs; // intentionally unused for now

    let status = cmd
        .status()
        .map_err(|e| format!("failed to invoke '{}': {e}", oxy.display()))?;
    // We don't care about the dump output; just bubble the exit code up.
    Ok(status.code().unwrap_or(1))
}

fn invoke_oxy(project: &Project, subcmd: &str, extra_args: &[String]) -> TugResult<i32> {
    let externs = resolve_externs(project)?;
    let entry = project_entry(project)?;
    let oxy = locate_oxy()?;

    let mut cmd = Command::new(&oxy);
    cmd.arg(subcmd);
    for (name, path) in &externs {
        cmd.arg("--extern")
            .arg(format!("{name}={}", path.display()));
    }
    cmd.arg(&entry);
    for a in extra_args {
        cmd.arg(a);
    }

    let status = cmd
        .status()
        .map_err(|e| format!("failed to invoke '{}': {e}", oxy.display()))?;
    Ok(status.code().unwrap_or(1))
}

/// Helper used by tests and the CLI to invoke an arbitrary oxy command line
/// with a known extern map (e.g. for one-off scripts that don't live in a
/// project). Not exposed in the CLI yet.
#[doc(hidden)]
pub fn raw_oxy(args: &[String], oxy_path: Option<&Path>) -> TugResult<i32> {
    let oxy = match oxy_path {
        Some(p) => p.to_path_buf(),
        None => locate_oxy()?,
    };
    let status = Command::new(&oxy)
        .args(args)
        .status()
        .map_err(|e| format!("failed to invoke '{}': {e}", oxy.display()))?;
    Ok(status.code().unwrap_or(1))
}
