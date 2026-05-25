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

pub use install::{packages_dir, InstalledPackage};
pub use lockfile::{LockedPackage, TugLock};
pub use manifest::{Dependency, GitRef, TugManifest};
pub use project::Project;
