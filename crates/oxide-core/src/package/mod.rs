//! Package manager for Oxide.
//!
//! Packages are directories of `.ox` files with a `package.ox` manifest.
//! They are installed to `~/.oxide/packages/<name>/`.

use std::path::{Path, PathBuf};

/// A parsed package manifest from `package.ox`.
#[derive(Debug, Clone)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub entry: Option<String>,
    pub dependencies: Vec<String>,
}

impl PackageManifest {
    /// Parse a `package.ox` manifest file.
    pub fn parse(source: &str) -> Result<Self, String> {
        let mut name = None;
        let mut version = None;
        let mut entry = None;
        let mut dependencies = Vec::new();

        for line in source.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
                continue;
            }

            // Simple key = "value" parsing
            if let Some((key, value)) = parse_kv(line) {
                match key {
                    "name" => name = Some(value.to_string()),
                    "version" => version = Some(value.to_string()),
                    "entry" => entry = Some(value.to_string()),
                    "dep" | "dependency" => dependencies.push(value.to_string()),
                    _ => {}
                }
            }
        }

        Ok(Self {
            name: name.ok_or_else(|| "package.ox must specify a name".to_string())?,
            version: version.unwrap_or_else(|| "0.1.0".to_string()),
            entry,
            dependencies,
        })
    }
}

/// Parse a key = "value" line. Returns None if not a valid KV pair.
fn parse_kv(line: &str) -> Option<(&str, &str)> {
    let (key, rest) = line.split_once('=')?;
    let key = key.trim();
    let rest = rest.trim().trim_end_matches(';');
    // Strip quotes
    let value = rest
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| rest.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(rest);
    Some((key, value))
}

/// Get the Oxide packages directory: `~/.oxide/packages/`
pub fn packages_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".oxide").join("packages")
}

/// Package metadata after installation.
#[derive(Debug, Clone)]
pub struct InstalledPackage {
    pub manifest: PackageManifest,
    pub path: PathBuf,
}

/// Install a package from a local directory path.
pub fn install_from_path(source_path: &Path) -> Result<InstalledPackage, String> {
    let manifest_path = source_path.join("package.ox");
    if !manifest_path.exists() {
        return Err(format!(
            "no package.ox found in {}",
            source_path.display()
        ));
    }

    let manifest_source = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read package.ox: {e}"))?;
    let manifest = PackageManifest::parse(&manifest_source)?;

    let dest_dir = packages_dir().join(&manifest.name);

    // Remove existing installation if present
    if dest_dir.exists() {
        std::fs::remove_dir_all(&dest_dir)
            .map_err(|e| format!("failed to remove existing package: {e}"))?;
    }

    // Copy all files
    copy_dir(source_path, &dest_dir)
        .map_err(|e| format!("failed to copy package: {e}"))?;

    Ok(InstalledPackage {
        manifest,
        path: dest_dir,
    })
}

/// Install a package from a git URL.
pub fn install_from_url(url: &str) -> Result<InstalledPackage, String> {
    let tmp = std::env::temp_dir().join(format!("oxide-pkg-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| format!("failed to create temp dir: {e}"))?;

    let status = std::process::Command::new("git")
        .args(["clone", "--depth", "1", url])
        .arg(&tmp)
        .status()
        .map_err(|e| format!("git not available: {e}"))?;

    if !status.success() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(format!("git clone failed with exit code: {}", status));
    }

    // Find the cloned directory (git creates a subdirectory)
    let entries: Vec<_> = std::fs::read_dir(&tmp)
        .map_err(|e| format!("failed to read temp dir: {e}"))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    let repo_dir = if entries.len() == 1 {
        entries[0].path()
    } else {
        tmp.clone()
    };

    let result = install_from_path(&repo_dir);
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

/// List all installed packages.
pub fn list_installed() -> Result<Vec<InstalledPackage>, String> {
    let dir = packages_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut packages = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| format!("failed to read packages dir: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read entry: {e}"))?;
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            let manifest_path = entry.path().join("package.ox");
            if let Ok(source) = std::fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = PackageManifest::parse(&source) {
                    packages.push(InstalledPackage {
                        manifest,
                        path: entry.path(),
                    });
                }
            }
        }
    }

    Ok(packages)
}

/// Normalize a name for matching: replace hyphens with underscores.
fn normalize_name(name: &str) -> String {
    name.replace('-', "_")
}

/// Find a module source file from installed packages.
/// Returns `(source, package_name)` if found.
pub fn find_module_in_packages(module_name: &str) -> Option<(String, String)> {
    let dir = packages_dir();
    if !dir.exists() {
        return None;
    }

    if let Ok(packages) = list_installed() {
        for pkg in &packages {
            // Try matching file names
            let path1 = pkg.path.join(format!("{module_name}.ox"));
            let path2 = pkg.path.join(module_name).join("mod.ox");

            if let Ok(source) = std::fs::read_to_string(&path1) {
                return Some((source, pkg.manifest.name.clone()));
            }
            if let Ok(source) = std::fs::read_to_string(&path2) {
                return Some((source, pkg.manifest.name.clone()));
            }

            // If module name matches the package name (normalized), try entry point
            if normalize_name(&pkg.manifest.name) == normalize_name(module_name) {
                if let Some(ref entry) = pkg.manifest.entry {
                    if let Ok(source) = std::fs::read_to_string(pkg.path.join(entry)) {
                        return Some((source, pkg.manifest.name.clone()));
                    }
                }
                // Also try lib.ox as default entry
                if let Ok(source) = std::fs::read_to_string(pkg.path.join("lib.ox")) {
                    return Some((source, pkg.manifest.name.clone()));
                }
            }

            // Search all .ox files in the package directory
            if let Ok(entries) = std::fs::read_dir(&pkg.path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.extension().map(|e| e == "ox").unwrap_or(false) {
                        if let Ok(source) = std::fs::read_to_string(&path) {
                            if source.contains(&format!("mod {module_name}"))
                                || source.contains(&format!("pub mod {module_name}"))
                                || source.contains(&format!("pub fn"))
                            {
                                return Some((source, pkg.manifest.name.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Recursively copy a directory.
fn copy_dir(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}
