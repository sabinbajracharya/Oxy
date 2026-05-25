//! `Project` — an on-disk Oxy project located by its `tug.toml`.
//!
//! Discovery walks upward from the starting directory looking for `tug.toml`
//! (mirroring Cargo's `Cargo.toml` discovery). The lockfile (`tug.lock`)
//! lives next to the manifest; it is created on demand when dependencies are
//! added.

use std::path::{Path, PathBuf};

use crate::lockfile::{LockedPackage, TugLock};
use crate::manifest::{Dependency, GitRef, Source, TugManifest};

const MANIFEST_NAME: &str = "tug.toml";
const LOCK_NAME: &str = "tug.lock";

#[derive(Debug)]
pub struct Project {
    root: PathBuf,
    manifest: TugManifest,
    lock: TugLock,
}

impl Project {
    /// Walk up from `start` until a `tug.toml` is found. Returns an error
    /// when no ancestor contains one.
    pub fn find(start: &Path) -> Result<Self, String> {
        let mut cur: Option<&Path> = Some(start);
        while let Some(dir) = cur {
            let candidate = dir.join(MANIFEST_NAME);
            if candidate.is_file() {
                return Self::load(dir);
            }
            cur = dir.parent();
        }
        Err(format!(
            "no {MANIFEST_NAME} found in '{}' or any parent directory",
            start.display()
        ))
    }

    /// Load a project whose manifest lives directly under `root`.
    pub fn load(root: &Path) -> Result<Self, String> {
        let manifest_path = root.join(MANIFEST_NAME);
        let manifest_src = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("failed to read {}: {e}", manifest_path.display()))?;
        let manifest = TugManifest::parse(&manifest_src)?;

        let lock_path = root.join(LOCK_NAME);
        let lock = if lock_path.is_file() {
            let src = std::fs::read_to_string(&lock_path)
                .map_err(|e| format!("failed to read {}: {e}", lock_path.display()))?;
            TugLock::parse(&src)?
        } else {
            TugLock::new()
        };

        Ok(Self {
            root: root.to_path_buf(),
            manifest,
            lock,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest(&self) -> &TugManifest {
        &self.manifest
    }

    pub fn lock(&self) -> &TugLock {
        &self.lock
    }

    /// Add or replace a dependency. Writes `tug.toml` and `tug.lock` to disk.
    pub fn add_dependency(&mut self, dep: Dependency) -> Result<(), String> {
        validate_dep_name(&dep.name)?;

        // Sync lockfile: record a pending entry that `tug install` will fill in.
        let locked = LockedPackage {
            name: dep.name.clone(),
            source: source_str(&dep.source),
            resolved: pending_resolved(&dep.source),
            checksum: None,
        };
        self.manifest.add_dependency(dep);
        self.lock.upsert(locked);
        self.save()
    }

    /// Remove a dependency by name. Writes manifest + lockfile.
    /// Returns `true` if something was removed.
    pub fn remove_dependency(&mut self, name: &str) -> Result<bool, String> {
        let removed_manifest = self.manifest.remove_dependency(name);
        let removed_lock = self.lock.remove(name);
        if removed_manifest || removed_lock {
            self.save()?;
        }
        Ok(removed_manifest || removed_lock)
    }

    /// Persist manifest + lockfile to disk.
    pub fn save(&self) -> Result<(), String> {
        std::fs::write(self.root.join(MANIFEST_NAME), self.manifest.to_string())
            .map_err(|e| format!("failed to write {MANIFEST_NAME}: {e}"))?;
        std::fs::write(self.root.join(LOCK_NAME), self.lock.to_string())
            .map_err(|e| format!("failed to write {LOCK_NAME}: {e}"))?;
        Ok(())
    }
}

/// Build a `Dependency` from a possibly-suffixed name spec and per-source
/// flags. Mutually-exclusive sources error.
///
/// `spec` can be `"name"` or `"name@version"`.
pub fn parse_dep_spec(
    spec: &str,
    git: Option<String>,
    tag: Option<String>,
    rev: Option<String>,
    path: Option<String>,
) -> Result<Dependency, String> {
    let (name, version_in_spec) = match spec.split_once('@') {
        Some((n, v)) => (n.to_string(), Some(v.to_string())),
        None => (spec.to_string(), None),
    };
    validate_dep_name(&name)?;

    let sources = [git.is_some(), path.is_some(), version_in_spec.is_some()]
        .iter()
        .filter(|b| **b)
        .count();
    if sources == 0 {
        return Err(format!(
            "no source for `{name}`: pass --git <url>, --path <p>, or `{name}@<version>`"
        ));
    }
    if sources > 1 {
        return Err(format!(
            "conflicting sources for `{name}`: pick one of --git, --path, or version"
        ));
    }

    if let Some(url) = git {
        let refs = [tag.is_some(), rev.is_some()]
            .iter()
            .filter(|b| **b)
            .count();
        if refs > 1 {
            return Err(format!("`{name}`: pick only one of --tag or --rev"));
        }
        let reference = if let Some(t) = tag {
            GitRef::Tag(t)
        } else if let Some(r) = rev {
            GitRef::Rev(r)
        } else {
            GitRef::Default
        };
        return Ok(Dependency {
            name,
            source: Source::Git { url, reference },
        });
    }
    if let Some(p) = path {
        return Ok(Dependency {
            name,
            source: Source::Path(p),
        });
    }
    Ok(Dependency {
        name,
        source: Source::Version(version_in_spec.unwrap()),
    })
}

fn validate_dep_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("dependency name must not be empty".to_string());
    }
    for c in name.chars() {
        if !(c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(format!(
                "invalid dependency name `{name}`: must be an identifier (letters, digits, `-`, `_`)"
            ));
        }
    }
    Ok(())
}

fn source_str(s: &Source) -> String {
    match s {
        Source::Version(v) => format!("version+{v}"),
        Source::Path(p) => format!("path+{p}"),
        Source::Git { url, reference } => match reference {
            GitRef::Tag(t) => format!("git+{url}#tag={t}"),
            GitRef::Rev(r) => format!("git+{url}#rev={r}"),
            GitRef::Branch(b) => format!("git+{url}#branch={b}"),
            GitRef::Default => format!("git+{url}"),
        },
    }
}

/// Placeholder `resolved` value used when a dep has been added but not yet
/// installed. `tug install` replaces this with a real SHA / digest.
fn pending_resolved(s: &Source) -> String {
    match s {
        Source::Path(_) => "pending".to_string(),
        Source::Version(_) => "pending".to_string(),
        Source::Git {
            reference: GitRef::Rev(r),
            ..
        } => r.clone(),
        _ => "pending".to_string(),
    }
}
