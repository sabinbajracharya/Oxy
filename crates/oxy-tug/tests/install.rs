//! Tests for `tug install <path>`, `tug uninstall <name>`, `tug list`, and
//! `find_installed_entry` — the package-store side of tug.
//!
//! Package store lives at `$HOME/.oxy/packages/<name>/`. Tests isolate by
//! overriding `$HOME` to a per-test tempdir, serialized via a mutex.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use oxy_tug::install;

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
            "tug-install-test-{}-{label}-{}",
            std::process::id(),
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
            dir,
            prev,
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
    std::fs::create_dir_all(src.join("src")).unwrap();
    std::fs::write(
        src.join("tug.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\n"),
    )
    .unwrap();
    std::fs::write(src.join("src/lib.ox"), "pub fn hi() {}\n").unwrap();
    src
}

// ---- packages_dir ----

#[test]
fn packages_dir_is_under_home_dot_oxy() {
    let g = HomeGuard::new("pkgdir");
    let dir = install::packages_dir();
    assert!(dir.starts_with(&g.dir));
    assert!(dir.ends_with(".oxy/packages") || dir.ends_with(".oxy\\packages"));
}

// ---- install_from_path ----

#[test]
fn install_from_path_creates_directory() {
    let g = HomeGuard::new("install-path");
    let src = make_pkg(&g.dir, "demo", "1.2.3");
    let installed = install::install_from_path(&src).unwrap();
    assert_eq!(installed.manifest.name, "demo");
    assert_eq!(installed.manifest.version, "1.2.3");
    assert_eq!(installed.path, install::packages_dir().join("demo"));
    assert!(installed.path.is_dir());
    assert!(installed.path.join("tug.toml").is_file());
    assert!(installed.path.join("src/lib.ox").is_file());
}

#[test]
fn install_rejects_missing_manifest() {
    let g = HomeGuard::new("install-no-toml");
    let src = g.dir.join("bad-pkg");
    std::fs::create_dir_all(&src).unwrap();
    let err = install::install_from_path(&src).unwrap_err();
    assert!(err.contains("tug.toml"), "got: {err}");
}

#[test]
fn install_rejects_malformed_manifest() {
    let g = HomeGuard::new("install-bad-toml");
    let src = g.dir.join("bad-pkg");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("tug.toml"), "[package\nname = \"x\"\n").unwrap();
    let err = install::install_from_path(&src).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn install_overwrites_existing_installation() {
    let g = HomeGuard::new("install-overwrite");
    let src1 = make_pkg(&g.dir, "samename", "1.0.0");
    install::install_from_path(&src1).unwrap();
    let src2 = make_pkg(&g.dir, "samename", "2.0.0");
    std::fs::write(
        src2.join("tug.toml"),
        "[package]\nname = \"samename\"\nversion = \"2.0.0\"\n",
    )
    .unwrap();
    let pkg = install::install_from_path(&src2).unwrap();
    assert_eq!(pkg.manifest.version, "2.0.0");
    let listed = install::list_installed().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].manifest.version, "2.0.0");
}

// ---- uninstall ----

#[test]
fn uninstall_removes_existing_package() {
    let g = HomeGuard::new("uninstall");
    let src = make_pkg(&g.dir, "todelete", "0.1.0");
    install::install_from_path(&src).unwrap();
    let removed = install::uninstall("todelete").unwrap();
    assert_eq!(removed, install::packages_dir().join("todelete"));
    assert!(!removed.exists());
}

#[test]
fn uninstall_missing_returns_err() {
    let _g = HomeGuard::new("uninstall-missing");
    let err = install::uninstall("nope-not-here").unwrap_err();
    assert!(err.contains("not installed"), "got: {err}");
}

// ---- list_installed ----

#[test]
fn list_installed_empty_when_no_packages_dir() {
    let _g = HomeGuard::new("list-empty");
    let list = install::list_installed().unwrap();
    assert!(list.is_empty());
}

#[test]
fn list_installed_returns_all_packages() {
    let g = HomeGuard::new("list-multi");
    install::install_from_path(&make_pkg(&g.dir, "a", "0.1.0")).unwrap();
    install::install_from_path(&make_pkg(&g.dir, "b", "0.2.0")).unwrap();
    install::install_from_path(&make_pkg(&g.dir, "c", "0.3.0")).unwrap();
    let listed = install::list_installed().unwrap();
    let mut names: Vec<_> = listed.iter().map(|p| p.manifest.name.clone()).collect();
    names.sort();
    assert_eq!(
        names,
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
}

#[test]
fn list_installed_ignores_directories_without_tug_toml() {
    let g = HomeGuard::new("list-ignores-stray");
    install::install_from_path(&make_pkg(&g.dir, "real", "1.0.0")).unwrap();
    // Drop an unrelated directory under the packages dir.
    let stray = install::packages_dir().join("stray");
    std::fs::create_dir_all(&stray).unwrap();
    std::fs::write(stray.join("README.md"), "no manifest here").unwrap();

    let listed = install::list_installed().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].manifest.name, "real");
}

// ---- find_installed_entry ----

#[test]
fn find_installed_entry_returns_lib_ox() {
    let g = HomeGuard::new("find-lib");
    install::install_from_path(&make_pkg(&g.dir, "json", "0.1.0")).unwrap();
    let p = install::find_installed_entry("json").expect("should find json entry");
    assert!(p.ends_with("src/lib.ox") || p.ends_with("src\\lib.ox"));
    assert!(p.is_file());
}

#[test]
fn find_installed_entry_falls_back_to_main_ox() {
    let g = HomeGuard::new("find-main");
    // Package with src/main.ox but NO src/lib.ox.
    let src = g.dir.join("src-mainonly");
    std::fs::create_dir_all(src.join("src")).unwrap();
    std::fs::write(
        src.join("tug.toml"),
        "[package]\nname = \"mainonly\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(src.join("src/main.ox"), "fn main() {}\n").unwrap();
    install::install_from_path(&src).unwrap();

    let p = install::find_installed_entry("mainonly").expect("should find entry");
    assert!(p.ends_with("src/main.ox") || p.ends_with("src\\main.ox"));
}

#[test]
fn find_installed_entry_returns_none_when_missing() {
    let _g = HomeGuard::new("find-missing");
    assert!(install::find_installed_entry("does-not-exist").is_none());
}

#[test]
fn find_installed_entry_returns_none_when_no_oxy_files() {
    let g = HomeGuard::new("find-no-oxy");
    let src = g.dir.join("src-empty");
    std::fs::create_dir_all(src.join("src")).unwrap();
    std::fs::write(
        src.join("tug.toml"),
        "[package]\nname = \"empty\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    // No .ox files inside src/.
    install::install_from_path(&src).unwrap();
    assert!(install::find_installed_entry("empty").is_none());
}
