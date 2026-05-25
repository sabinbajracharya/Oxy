//! Tests for `tug new <name>` and `tug init` — project scaffolding.

use std::path::{Path, PathBuf};

use oxy_tug::manifest::TugManifest;
use oxy_tug::scaffold;

fn unique_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tug-scaffold-{}-{label}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    // Caller may want the dir to NOT pre-exist — return the path only.
    dir
}

/// Read a UTF-8 file, panicking with a useful message on failure.
fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

// ---- new ----

#[test]
fn new_creates_directory_with_layout() {
    let target = unique_dir("new-layout");
    scaffold::new_project(&target, "myproj").unwrap();

    assert!(target.is_dir(), "target dir should exist");
    assert!(target.join("tug.toml").is_file(), "tug.toml should exist");
    assert!(target.join("src").is_dir(), "src/ should exist");
    assert!(
        target.join("src/main.ox").is_file(),
        "src/main.ox should exist"
    );
    assert!(
        target.join(".gitignore").is_file(),
        ".gitignore should exist"
    );

    let _ = std::fs::remove_dir_all(&target);
}

#[test]
fn new_writes_valid_manifest() {
    let target = unique_dir("new-manifest");
    scaffold::new_project(&target, "my-cool-proj").unwrap();

    let toml_src = read(&target.join("tug.toml"));
    let m = TugManifest::parse(&toml_src).expect("tug.toml should parse");
    assert_eq!(m.name, "my-cool-proj");
    assert_eq!(m.version, "0.1.0");
    assert!(m.dependencies.is_empty());

    let _ = std::fs::remove_dir_all(&target);
}

#[test]
fn new_writes_runnable_main_stub() {
    let target = unique_dir("new-main");
    scaffold::new_project(&target, "p").unwrap();
    let main = read(&target.join("src/main.ox"));
    assert!(
        main.contains("fn main()"),
        "main.ox needs fn main(): {main}"
    );
    let _ = std::fs::remove_dir_all(&target);
}

#[test]
fn new_writes_gitignore() {
    let target = unique_dir("new-gitignore");
    scaffold::new_project(&target, "p").unwrap();
    let gi = read(&target.join(".gitignore"));
    assert!(
        gi.contains("target") || gi.contains("/target"),
        "gitignore: {gi}"
    );
    let _ = std::fs::remove_dir_all(&target);
}

#[test]
fn new_errors_when_target_exists_and_nonempty() {
    let target = unique_dir("new-existing");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("something.txt"), "occupied").unwrap();

    let err = scaffold::new_project(&target, "p").unwrap_err();
    assert!(
        err.contains("exists") || err.contains("not empty"),
        "got: {err}"
    );

    let _ = std::fs::remove_dir_all(&target);
}

#[test]
fn new_succeeds_when_target_is_empty_dir() {
    let target = unique_dir("new-empty");
    std::fs::create_dir_all(&target).unwrap();
    // Empty pre-existing dir should be allowed (mirrors `cargo new --vcs none` behavior).
    scaffold::new_project(&target, "p").unwrap();
    assert!(target.join("tug.toml").is_file());
    let _ = std::fs::remove_dir_all(&target);
}

#[test]
fn new_rejects_invalid_package_name() {
    let target = unique_dir("new-badname");
    let err = scaffold::new_project(&target, "has spaces").unwrap_err();
    assert!(err.contains("name"), "got: {err}");
    // No files should have been created.
    assert!(
        !target.exists()
            || std::fs::read_dir(&target)
                .map(|d| d.count() == 0)
                .unwrap_or(true)
    );
    let _ = std::fs::remove_dir_all(&target);
}

#[test]
fn new_rejects_empty_package_name() {
    let target = unique_dir("new-emptyname");
    let err = scaffold::new_project(&target, "").unwrap_err();
    assert!(err.contains("name"), "got: {err}");
}

// ---- init ----

#[test]
fn init_creates_layout_in_existing_dir() {
    let target = unique_dir("init-here");
    std::fs::create_dir_all(&target).unwrap();
    scaffold::init_project(&target, "myproj").unwrap();
    assert!(target.join("tug.toml").is_file());
    assert!(target.join("src/main.ox").is_file());
    assert!(target.join(".gitignore").is_file());
    let _ = std::fs::remove_dir_all(&target);
}

#[test]
fn init_errors_when_manifest_already_exists() {
    let target = unique_dir("init-existing");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(
        target.join("tug.toml"),
        "[package]\nname=\"x\"\nversion=\"0.1.0\"\n",
    )
    .unwrap();
    let err = scaffold::init_project(&target, "p").unwrap_err();
    assert!(err.contains("tug.toml"), "got: {err}");
    let _ = std::fs::remove_dir_all(&target);
}

#[test]
fn init_preserves_existing_unrelated_files() {
    let target = unique_dir("init-preserve");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("README.md"), "hi").unwrap();
    scaffold::init_project(&target, "p").unwrap();
    assert!(
        target.join("README.md").is_file(),
        "existing files preserved"
    );
    assert!(target.join("tug.toml").is_file());
    let _ = std::fs::remove_dir_all(&target);
}

#[test]
fn init_uses_provided_name() {
    let target = unique_dir("init-name");
    std::fs::create_dir_all(&target).unwrap();
    scaffold::init_project(&target, "given-name").unwrap();
    let m = TugManifest::parse(&read(&target.join("tug.toml"))).unwrap();
    assert_eq!(m.name, "given-name");
    let _ = std::fs::remove_dir_all(&target);
}

#[test]
fn init_defaults_name_to_dir_basename_when_empty() {
    // When the caller passes "" tug should use the basename of the target dir
    // (mirrors `cargo init` defaulting to the current dir name).
    let target = unique_dir("init-default-name").join("auto-derived");
    std::fs::create_dir_all(&target).unwrap();
    scaffold::init_project(&target, "").unwrap();
    let m = TugManifest::parse(&read(&target.join("tug.toml"))).unwrap();
    assert_eq!(m.name, "auto-derived");
    let _ = std::fs::remove_dir_all(target.parent().unwrap());
}
