//! Tests for `Project::find` (manifest discovery) and `add`/`remove`/`update`
//! dependency-management operations on a project on disk.

use std::path::{Path, PathBuf};

use oxy_tug::manifest::{Dependency, TugManifest};
use oxy_tug::project::Project;

fn unique_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tug-project-{}-{label}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_manifest(root: &Path, body: &str) {
    std::fs::write(root.join("tug.toml"), body).unwrap();
}

// ---- discovery ----

#[test]
fn find_locates_manifest_in_cwd() {
    let dir = unique_dir("find-cwd");
    write_manifest(&dir, "[package]\nname=\"x\"\nversion=\"0.1.0\"\n");
    let p = Project::find(&dir).unwrap();
    assert_eq!(p.root(), dir);
    assert_eq!(p.manifest().name, "x");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn find_walks_up_to_parent_directory() {
    let root = unique_dir("find-parent");
    let nested = root.join("a/b/c");
    std::fs::create_dir_all(&nested).unwrap();
    write_manifest(
        &root,
        "[package]\nname=\"parent-proj\"\nversion=\"0.1.0\"\n",
    );
    let p = Project::find(&nested).unwrap();
    assert_eq!(p.root(), root);
    assert_eq!(p.manifest().name, "parent-proj");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn find_errors_when_no_manifest_in_ancestors() {
    let dir = unique_dir("find-none");
    let err = Project::find(&dir).unwrap_err();
    assert!(err.contains("tug.toml"), "got: {err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn find_errors_on_malformed_manifest() {
    let dir = unique_dir("find-bad");
    write_manifest(&dir, "[package\nname=\"x\"\n");
    let err = Project::find(&dir).unwrap_err();
    assert!(!err.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

// ---- save_manifest + add_dependency ----

#[test]
fn add_dependency_persists_to_disk() {
    let dir = unique_dir("add-persist");
    write_manifest(&dir, "[package]\nname=\"p\"\nversion=\"0.1.0\"\n");
    let mut p = Project::find(&dir).unwrap();
    p.add_dependency(Dependency::path("local", "../local-pkg"))
        .unwrap();

    // Reload from disk and verify.
    let reloaded = Project::find(&dir).unwrap();
    let dep = reloaded
        .manifest()
        .dependencies
        .iter()
        .find(|d| d.name == "local")
        .expect("dep persisted");
    match &dep.source {
        oxy_tug::manifest::Source::Path(p) => assert_eq!(p, "../local-pkg"),
        _ => panic!("expected path dep"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn add_dependency_replaces_same_name() {
    let dir = unique_dir("add-replace");
    write_manifest(&dir, "[package]\nname=\"p\"\nversion=\"0.1.0\"\n");
    let mut p = Project::find(&dir).unwrap();
    p.add_dependency(Dependency::version("x", "1.0.0")).unwrap();
    p.add_dependency(Dependency::version("x", "2.0.0")).unwrap();

    let reloaded = Project::find(&dir).unwrap();
    assert_eq!(reloaded.manifest().dependencies.len(), 1);
    let dep = &reloaded.manifest().dependencies[0];
    match &dep.source {
        oxy_tug::manifest::Source::Version(v) => assert_eq!(v, "2.0.0"),
        _ => panic!("expected version dep"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn add_dependency_rejects_invalid_name() {
    let dir = unique_dir("add-invalid");
    write_manifest(&dir, "[package]\nname=\"p\"\nversion=\"0.1.0\"\n");
    let mut p = Project::find(&dir).unwrap();
    let err = p
        .add_dependency(Dependency::version("has spaces", "1.0.0"))
        .unwrap_err();
    assert!(err.contains("name"), "got: {err}");
    let _ = std::fs::remove_dir_all(&dir);
}

// ---- remove_dependency ----

#[test]
fn remove_dependency_persists_to_disk() {
    let dir = unique_dir("rm-persist");
    write_manifest(
        &dir,
        "[package]\nname=\"p\"\nversion=\"0.1.0\"\n\n[dependencies]\na = \"1.0.0\"\nb = \"2.0.0\"\n",
    );
    let mut p = Project::find(&dir).unwrap();
    assert!(p.remove_dependency("a").unwrap());

    let reloaded = Project::find(&dir).unwrap();
    assert_eq!(reloaded.manifest().dependencies.len(), 1);
    assert_eq!(reloaded.manifest().dependencies[0].name, "b");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn remove_dependency_returns_false_when_absent() {
    let dir = unique_dir("rm-absent");
    write_manifest(&dir, "[package]\nname=\"p\"\nversion=\"0.1.0\"\n");
    let mut p = Project::find(&dir).unwrap();
    assert!(!p.remove_dependency("nothere").unwrap());
    let _ = std::fs::remove_dir_all(&dir);
}

// ---- lockfile coordination ----

#[test]
fn add_path_dependency_records_unresolved_lockfile_entry() {
    let dir = unique_dir("lock-add");
    write_manifest(&dir, "[package]\nname=\"p\"\nversion=\"0.1.0\"\n");
    let mut p = Project::find(&dir).unwrap();
    p.add_dependency(Dependency::path("local", "../local-pkg"))
        .unwrap();

    // No installation has happened yet, so the lockfile records the dep but
    // with a placeholder `resolved` ("pending"). `tug install` would replace
    // this with the real digest.
    let reloaded = Project::find(&dir).unwrap();
    let lock = reloaded.lock();
    let entry = lock.find("local").expect("lock entry");
    assert!(entry.source.starts_with("path+"));
    assert!(entry.resolved.is_empty() || entry.resolved == "pending");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn remove_dependency_also_removes_lockfile_entry() {
    let dir = unique_dir("lock-rm");
    write_manifest(&dir, "[package]\nname=\"p\"\nversion=\"0.1.0\"\n");
    let mut p = Project::find(&dir).unwrap();
    p.add_dependency(Dependency::path("local", "../local-pkg"))
        .unwrap();
    assert!(p.remove_dependency("local").unwrap());

    let reloaded = Project::find(&dir).unwrap();
    assert!(reloaded.lock().find("local").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn parse_dep_spec_version() {
    // tug add x@1.2.3   →   version dep
    let dep = oxy_tug::project::parse_dep_spec("x@1.2.3", None, None, None, None).unwrap();
    assert_eq!(dep.name, "x");
    match &dep.source {
        oxy_tug::manifest::Source::Version(v) => assert_eq!(v, "1.2.3"),
        _ => panic!("expected version"),
    }
}

#[test]
fn parse_dep_spec_git_with_tag() {
    let dep = oxy_tug::project::parse_dep_spec(
        "json",
        Some("https://example.com/json.git".to_string()),
        Some("v1".to_string()),
        None,
        None,
    )
    .unwrap();
    assert_eq!(dep.name, "json");
    match &dep.source {
        oxy_tug::manifest::Source::Git { url, reference } => {
            assert_eq!(url, "https://example.com/json.git");
            assert!(matches!(reference, oxy_tug::manifest::GitRef::Tag(t) if t == "v1"));
        }
        _ => panic!("expected git"),
    }
}

#[test]
fn parse_dep_spec_path() {
    let dep = oxy_tug::project::parse_dep_spec(
        "local",
        None,
        None,
        None,
        Some("../local-pkg".to_string()),
    )
    .unwrap();
    match &dep.source {
        oxy_tug::manifest::Source::Path(p) => assert_eq!(p, "../local-pkg"),
        _ => panic!("expected path"),
    }
}

#[test]
fn parse_dep_spec_rejects_conflicting_sources() {
    let err = oxy_tug::project::parse_dep_spec(
        "x",
        Some("https://example.com/x.git".to_string()),
        None,
        None,
        Some("../x".to_string()),
    )
    .unwrap_err();
    assert!(
        err.contains("one of") || err.contains("conflict"),
        "got: {err}"
    );
}

// Sanity test for the in-memory TugManifest used as a side check.
#[test]
fn manifest_roundtrip_after_project_edits() {
    let dir = unique_dir("rt-after");
    write_manifest(&dir, "[package]\nname=\"p\"\nversion=\"0.1.0\"\n");
    let mut p = Project::find(&dir).unwrap();
    p.add_dependency(Dependency::version("a", "1.0.0")).unwrap();
    p.add_dependency(Dependency::path("b", "../b")).unwrap();

    let src = std::fs::read_to_string(dir.join("tug.toml")).unwrap();
    let _m = TugManifest::parse(&src).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}
