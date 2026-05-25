//! Package-store operations: install, uninstall, list, resolve entry points.
//!
//! Packages are installed under `~/.oxy/packages/<name>/` and each must
//! contain a `tug.toml` manifest. When a project depends on a package,
//! `tug` resolves its entry point via [`find_installed_entry`] and passes
//! that path to `oxy` through `--extern <name>=<path>`. The `oxy` compiler
//! itself never reads from `~/.oxy/packages/` — that wiring belongs to tug.

use std::path::{Path, PathBuf};

use crate::manifest::TugManifest;

/// Metadata returned after a successful install.
#[derive(Debug, Clone)]
pub struct InstalledPackage {
    pub manifest: TugManifest,
    pub path: PathBuf,
}

/// The directory tug installs packages into: `$HOME/.oxy/packages/`.
pub fn packages_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".oxy").join("packages")
}

/// Install a package from a local directory. The directory must contain a
/// valid `tug.toml`. Any existing installation with the same package name is
/// replaced.
pub fn install_from_path(source: &Path) -> Result<InstalledPackage, String> {
    let manifest_path = source.join("tug.toml");
    if !manifest_path.exists() {
        return Err(format!("no tug.toml found in {}", source.display()));
    }
    let manifest_src = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read tug.toml: {e}"))?;
    let manifest = TugManifest::parse(&manifest_src)?;

    let dest = packages_dir().join(&manifest.name);
    if dest.exists() {
        std::fs::remove_dir_all(&dest)
            .map_err(|e| format!("failed to remove existing install: {e}"))?;
    }
    copy_dir(source, &dest).map_err(|e| format!("failed to copy package: {e}"))?;

    Ok(InstalledPackage {
        manifest,
        path: dest,
    })
}

/// Install a package from a git URL by shelling out to `git clone` and then
/// running [`install_from_path`] against the clone.
pub fn install_from_url(url: &str) -> Result<InstalledPackage, String> {
    let tmp = std::env::temp_dir().join(format!(
        "tug-clone-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&tmp).map_err(|e| format!("failed to create temp dir: {e}"))?;

    let status = std::process::Command::new("git")
        .args(["clone", "--depth", "1", url])
        .arg(&tmp)
        .status()
        .map_err(|e| format!("git not available: {e}"))?;
    if !status.success() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(format!("git clone failed with exit code: {status}"));
    }

    let result = install_from_path(&tmp);
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

/// Remove an installed package by name. Returns the removed path on success.
pub fn uninstall(name: &str) -> Result<PathBuf, String> {
    let dir = packages_dir().join(name);
    if !dir.exists() {
        return Err(format!("package not installed: '{name}'"));
    }
    std::fs::remove_dir_all(&dir).map_err(|e| format!("failed to remove package: {e}"))?;
    Ok(dir)
}

/// List every installed package (those with a parseable `tug.toml`).
pub fn list_installed() -> Result<Vec<InstalledPackage>, String> {
    let dir = packages_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let entries =
        std::fs::read_dir(&dir).map_err(|e| format!("failed to read packages dir: {e}"))?;
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read entry: {e}"))?;
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let manifest_path = entry.path().join("tug.toml");
        let Ok(src) = std::fs::read_to_string(&manifest_path) else {
            continue;
        };
        let Ok(manifest) = TugManifest::parse(&src) else {
            continue;
        };
        out.push(InstalledPackage {
            manifest,
            path: entry.path(),
        });
    }
    Ok(out)
}

/// Resolve the on-disk entry point for an installed package.
///
/// Preference order:
/// 1. `<pkg>/src/lib.ox`
/// 2. `<pkg>/src/main.ox`
/// 3. the first `*.ox` file under `<pkg>/src/`
///
/// Returns `None` if the package isn't installed or has no `.ox` files.
/// Used by tug to build the `--extern <name>=<path>` map it hands to oxy.
pub fn find_installed_entry(name: &str) -> Option<PathBuf> {
    let pkg_root = packages_dir().join(name);
    if !pkg_root.is_dir() {
        return None;
    }
    let src = pkg_root.join("src");
    let lib = src.join("lib.ox");
    if lib.is_file() {
        return Some(lib);
    }
    let main = src.join("main.ox");
    if main.is_file() {
        return Some(main);
    }
    if let Ok(entries) = std::fs::read_dir(&src) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "ox").unwrap_or(false) && p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
