//! Package manager for Oxy.
//!
//! Packages are directories of `.ox` files with a `package.ox` manifest.
//! They are installed to `~/.oxy/packages/<name>/`.

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

/// Get the Oxy packages directory: `~/.oxy/packages/`
pub fn packages_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".oxy").join("packages")
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
        return Err(format!("no package.ox found in {}", source_path.display()));
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
    copy_dir(source_path, &dest_dir).map_err(|e| format!("failed to copy package: {e}"))?;

    Ok(InstalledPackage {
        manifest,
        path: dest_dir,
    })
}

/// Install a package from a git URL.
pub fn install_from_url(url: &str) -> Result<InstalledPackage, String> {
    let tmp = std::env::temp_dir().join(format!("oxy-pkg-{}", std::process::id()));
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

/// Uninstall a package by name. Returns the removed path on success.
pub fn uninstall(name: &str) -> Result<PathBuf, String> {
    let dir = packages_dir().join(name);
    if !dir.exists() {
        return Err(format!("package not installed: '{name}'"));
    }
    std::fs::remove_dir_all(&dir).map_err(|e| format!("failed to remove package: {e}"))?;
    Ok(dir)
}

/// List all installed packages.
pub fn list_installed() -> Result<Vec<InstalledPackage>, String> {
    let dir = packages_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut packages = Vec::new();
    let entries =
        std::fs::read_dir(&dir).map_err(|e| format!("failed to read packages dir: {e}"))?;

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
                                || source.contains("pub fn")
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // packages_dir() reads $HOME, and tests within a process share env.
    // Serialize tests that mutate HOME.
    static HOME_LOCK: Mutex<()> = Mutex::new(());

    struct HomeGuard {
        prev: Option<String>,
        dir: PathBuf,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl HomeGuard {
        fn new(label: &str) -> Self {
            let lock = HOME_LOCK.lock().unwrap_or_else(|p| p.into_inner());
            let prev = std::env::var("HOME").ok();
            let dir = std::env::temp_dir().join(format!(
                "oxy-pkg-test-{}-{}-{}",
                std::process::id(),
                label,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0),
            ));
            std::fs::create_dir_all(&dir).unwrap();
            unsafe {
                std::env::set_var("HOME", &dir);
            }
            Self {
                prev,
                dir,
                _lock: lock,
            }
        }
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
            unsafe {
                match &self.prev {
                    Some(v) => std::env::set_var("HOME", v),
                    None => std::env::remove_var("HOME"),
                }
            }
        }
    }

    fn make_pkg(parent: &Path, name: &str, version: &str) -> PathBuf {
        let src = parent.join(format!("src-{name}"));
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("package.ox"),
            format!("name = \"{name}\"\nversion = \"{version}\"\n"),
        )
        .unwrap();
        std::fs::write(src.join("lib.ox"), "pub fn hi() {}\n").unwrap();
        src
    }

    #[test]
    fn manifest_parse_basic() {
        let m = PackageManifest::parse(
            "name = \"foo\"\nversion = \"1.2.3\"\nentry = \"lib.ox\"\ndep = \"bar\"\n",
        )
        .unwrap();
        assert_eq!(m.name, "foo");
        assert_eq!(m.version, "1.2.3");
        assert_eq!(m.entry.as_deref(), Some("lib.ox"));
        assert_eq!(m.dependencies, vec!["bar".to_string()]);
    }

    #[test]
    fn manifest_parse_requires_name() {
        let err = PackageManifest::parse("version = \"1.0.0\"\n").unwrap_err();
        assert!(err.contains("name"));
    }

    #[test]
    fn manifest_parse_default_version() {
        let m = PackageManifest::parse("name = \"x\"\n").unwrap();
        assert_eq!(m.version, "0.1.0");
    }

    #[test]
    fn uninstall_missing_returns_err() {
        let _g = HomeGuard::new("uninstall-missing");
        let err = uninstall("nonexistent-xyz").unwrap_err();
        assert!(err.contains("not installed"), "unexpected: {err}");
    }

    #[test]
    fn install_list_uninstall_cycle() {
        let g = HomeGuard::new("cycle");

        assert!(list_installed().unwrap().is_empty());

        let src = make_pkg(&g.dir, "demo", "1.2.3");
        let installed = install_from_path(&src).unwrap();
        assert_eq!(installed.manifest.name, "demo");
        assert_eq!(installed.manifest.version, "1.2.3");
        assert_eq!(installed.path, packages_dir().join("demo"));
        assert!(installed.path.exists());

        let listed = list_installed().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].manifest.name, "demo");

        let (source, pkg_name) = find_module_in_packages("demo").expect("module found");
        assert_eq!(pkg_name, "demo");
        assert!(source.contains("pub fn hi"));

        let removed = uninstall("demo").unwrap();
        assert_eq!(removed, packages_dir().join("demo"));
        assert!(!removed.exists());
        assert!(list_installed().unwrap().is_empty());
    }

    #[test]
    fn install_overwrites_existing() {
        let g = HomeGuard::new("overwrite");
        let src1 = make_pkg(&g.dir, "samename", "1.0.0");
        install_from_path(&src1).unwrap();
        let src2 = make_pkg(&g.dir, "samename", "2.0.0");
        std::fs::write(
            src2.join("package.ox"),
            "name = \"samename\"\nversion = \"2.0.0\"\n",
        )
        .unwrap();
        let installed = install_from_path(&src2).unwrap();
        assert_eq!(installed.manifest.version, "2.0.0");
        let listed = list_installed().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].manifest.version, "2.0.0");
    }

    #[test]
    fn install_rejects_missing_manifest() {
        let g = HomeGuard::new("no-manifest");
        let src = g.dir.join("bad-pkg");
        std::fs::create_dir_all(&src).unwrap();
        let err = install_from_path(&src).unwrap_err();
        assert!(err.contains("package.ox"), "unexpected: {err}");
    }
}
