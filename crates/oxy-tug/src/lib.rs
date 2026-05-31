//! Tug — the Oxy package manager library.
//!
//! Tug is to Oxy what Cargo is to Rust: it owns the project manifest
//! (`tug.toml`), the resolved lockfile (`tug.lock`), dependency installation,
//! and orchestration of the `oxy` compiler. The `oxy` compiler itself knows
//! nothing about packages — tug feeds resolved dependencies into `oxy` via
//! its `--extern <name>=<path>` flag (mirroring `rustc --extern`).
//!
//! This library exposes the data model and parser/serializer for those files
//! so the CLI binary and external callers (tests, IDE integrations) can share
//! one implementation.

pub mod install;
pub mod lockfile;
pub mod manifest;
pub mod project;
pub mod runner;
pub mod scaffold;

use std::fmt;
use std::ops::Deref;

pub use install::{packages_dir, InstalledPackage};
pub use lockfile::{LockedPackage, TugLock};
pub use manifest::{Dependency, GitRef, TugManifest};
pub use project::Project;

/// Error type for tug operations. Wraps a message string; will be refined
/// into a proper enum with structured variants in a future change.
#[derive(Debug, Clone)]
pub struct TugError(pub String);

impl fmt::Display for TugError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for TugError {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<String> for TugError {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<std::io::Error> for TugError {
    fn from(e: std::io::Error) -> Self {
        Self(e.to_string())
    }
}

/// Convenience alias for `Result<T, TugError>`.
pub type TugResult<T> = Result<T, TugError>;

/// Construct a `TugError` via `format!`-style arguments.
///
/// ```ignore
/// return Err(tug_err!("no tug.toml found in {}", path.display()));
/// ```
#[macro_export]
macro_rules! tug_err {
    ($($arg:tt)*) => {
        $crate::TugError(format!($($arg)*))
    };
}
