//! Project scaffolding: `tug new <name>` and `tug init`.
//!
//! Mirrors Cargo's behavior:
//! - `new`  creates a new directory with the standard layout.
//! - `init` initializes a project inside an existing directory, preserving
//!   any files already there as long as no `tug.toml` is present.
//!
//! The layout created:
//!
//! ```text
//! <root>/
//!   tug.toml      # project manifest
//!   .gitignore    # ignores /target by default
//!   src/
//!     main.ox     # `fn main()` stub
//! ```

use std::path::Path;

use crate::manifest::TugManifest;
use crate::tug_err;
use crate::TugResult;

/// Create a brand-new project at `target`. The directory must either not exist
/// or be empty.
pub fn new_project(target: &Path, name: &str) -> TugResult<()> {
    validate_package_name(name)?;

    if target.exists() {
        let nonempty = std::fs::read_dir(target)
            .map_err(|e| format!("failed to read '{}': {e}", target.display()))?
            .next()
            .is_some();
        if nonempty {
            return Err(tug_err!(
                "target '{}' already exists and is not empty",
                target.display()
            ));
        }
    } else {
        std::fs::create_dir_all(target)
            .map_err(|e| format!("failed to create '{}': {e}", target.display()))?;
    }

    write_layout(target, name)
}

/// Initialize a project in-place inside `target` (which must already exist).
/// Preserves existing files except that `tug.toml` must not already exist.
///
/// If `name` is empty, the project name defaults to the basename of `target`.
pub fn init_project(target: &Path, name: &str) -> TugResult<()> {
    if !target.exists() {
        return Err(tug_err!("target '{}' does not exist", target.display()));
    }
    if !target.is_dir() {
        return Err(tug_err!("target '{}' is not a directory", target.display()));
    }

    let resolved_name = if name.is_empty() {
        target
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "could not derive a name from the target directory".to_string())?
            .to_string()
    } else {
        name.to_string()
    };
    validate_package_name(&resolved_name)?;

    if target.join("tug.toml").exists() {
        return Err(tug_err!(
            "'{}/tug.toml' already exists \u{2014} project already initialized",
            target.display()
        ));
    }

    write_layout(target, &resolved_name)
}

/// Write the three files that make up a fresh project.
///
/// Idempotent only at the level of writing — caller must have checked
/// pre-existing state.
fn write_layout(root: &Path, name: &str) -> TugResult<()> {
    let manifest = TugManifest::new(name, "0.1.0");
    std::fs::write(root.join("tug.toml"), manifest.to_string())
        .map_err(|e| format!("failed to write tug.toml: {e}"))?;

    let src = root.join("src");
    std::fs::create_dir_all(&src).map_err(|e| format!("failed to create src/: {e}"))?;
    std::fs::write(src.join("main.ox"), DEFAULT_MAIN)
        .map_err(|e| format!("failed to write src/main.ox: {e}"))?;

    std::fs::write(root.join(".gitignore"), DEFAULT_GITIGNORE)
        .map_err(|e| format!("failed to write .gitignore: {e}"))?;

    Ok(())
}

/// A dependency-style name validation, applied to project names as well so
/// that they round-trip cleanly through `tug.toml`.
fn validate_package_name(name: &str) -> TugResult<()> {
    if name.is_empty() {
        return Err(tug_err!("package name must not be empty"));
    }
    for c in name.chars() {
        if !(c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(tug_err!(
                "invalid package name `{name}`: must be an identifier (letters, digits, `-`, `_`)"
            ));
        }
    }
    Ok(())
}

const DEFAULT_MAIN: &str = "fn main() {\n    println(\"Hello, Oxy!\");\n}\n";

const DEFAULT_GITIGNORE: &str = "/target\n";
