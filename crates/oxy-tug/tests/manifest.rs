//! Tests for `tug.toml` parsing and serialization.

use oxy_tug::manifest::{Dependency, GitRef, TugManifest};

// ---- happy-path parsing ----

#[test]
fn parses_minimal_manifest() {
    let src = r#"
[package]
name = "myproj"
version = "0.1.0"
"#;
    let m = TugManifest::parse(src).unwrap();
    assert_eq!(m.name, "myproj");
    assert_eq!(m.version, "0.1.0");
    assert!(m.dependencies.is_empty());
}

#[test]
fn parses_version_as_string_dep() {
    let src = r#"
[package]
name = "myproj"
version = "0.1.0"

[dependencies]
serde_ish = "1.0.0"
"#;
    let m = TugManifest::parse(src).unwrap();
    assert_eq!(m.dependencies.len(), 1);
    let dep = &m.dependencies[0];
    assert_eq!(dep.name, "serde_ish");
    assert!(
        matches!(&dep, Dependency { source: oxy_tug::manifest::Source::Version(v), .. } if v == "1.0.0"),
        "expected Version dep, got {dep:?}"
    );
}

#[test]
fn parses_git_dep_with_tag() {
    let src = r#"
[package]
name = "p"
version = "0.1.0"

[dependencies]
json = { git = "https://github.com/oxy-lang/oxy-json", tag = "v1.0.0" }
"#;
    let m = TugManifest::parse(src).unwrap();
    let dep = m.dependencies.iter().find(|d| d.name == "json").unwrap();
    match &dep.source {
        oxy_tug::manifest::Source::Git { url, reference } => {
            assert_eq!(url, "https://github.com/oxy-lang/oxy-json");
            assert!(matches!(reference, GitRef::Tag(t) if t == "v1.0.0"));
        }
        other => panic!("expected Git dep, got {other:?}"),
    }
}

#[test]
fn parses_git_dep_with_rev() {
    let src = r#"
[package]
name = "p"
version = "0.1.0"

[dependencies]
x = { git = "https://example.com/x.git", rev = "deadbeef" }
"#;
    let m = TugManifest::parse(src).unwrap();
    let dep = m.dependencies.iter().find(|d| d.name == "x").unwrap();
    match &dep.source {
        oxy_tug::manifest::Source::Git { reference, .. } => {
            assert!(matches!(reference, GitRef::Rev(r) if r == "deadbeef"));
        }
        _ => panic!("expected Git dep"),
    }
}

#[test]
fn parses_git_dep_with_branch() {
    let src = r#"
[package]
name = "p"
version = "0.1.0"

[dependencies]
x = { git = "https://example.com/x.git", branch = "develop" }
"#;
    let m = TugManifest::parse(src).unwrap();
    let dep = m.dependencies.iter().find(|d| d.name == "x").unwrap();
    match &dep.source {
        oxy_tug::manifest::Source::Git { reference, .. } => {
            assert!(matches!(reference, GitRef::Branch(b) if b == "develop"));
        }
        _ => panic!("expected Git dep"),
    }
}

#[test]
fn parses_git_dep_without_ref_defaults_to_default_branch() {
    let src = r#"
[package]
name = "p"
version = "0.1.0"

[dependencies]
x = { git = "https://example.com/x.git" }
"#;
    let m = TugManifest::parse(src).unwrap();
    let dep = m.dependencies.iter().find(|d| d.name == "x").unwrap();
    match &dep.source {
        oxy_tug::manifest::Source::Git { reference, .. } => {
            assert!(matches!(reference, GitRef::Default));
        }
        _ => panic!("expected Git dep"),
    }
}

#[test]
fn parses_path_dep() {
    let src = r#"
[package]
name = "p"
version = "0.1.0"

[dependencies]
local = { path = "../local-pkg" }
"#;
    let m = TugManifest::parse(src).unwrap();
    let dep = m.dependencies.iter().find(|d| d.name == "local").unwrap();
    match &dep.source {
        oxy_tug::manifest::Source::Path(p) => {
            assert_eq!(p, "../local-pkg");
        }
        _ => panic!("expected Path dep"),
    }
}

#[test]
fn parses_multiple_dependencies() {
    let src = r#"
[package]
name = "p"
version = "0.1.0"

[dependencies]
a = "1.0.0"
b = { git = "https://example.com/b.git", tag = "v2" }
c = { path = "../c" }
"#;
    let m = TugManifest::parse(src).unwrap();
    assert_eq!(m.dependencies.len(), 3);
    let names: Vec<_> = m.dependencies.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"a"));
    assert!(names.contains(&"b"));
    assert!(names.contains(&"c"));
}

// ---- error cases ----

#[test]
fn errors_on_missing_package_table() {
    let src = r#"
[dependencies]
a = "1.0.0"
"#;
    let err = TugManifest::parse(src).unwrap_err();
    assert!(err.contains("[package]"), "got: {err}");
}

#[test]
fn errors_on_missing_name() {
    let src = r#"
[package]
version = "0.1.0"
"#;
    let err = TugManifest::parse(src).unwrap_err();
    assert!(err.contains("name"), "got: {err}");
}

#[test]
fn errors_on_missing_version() {
    let src = r#"
[package]
name = "x"
"#;
    let err = TugManifest::parse(src).unwrap_err();
    assert!(err.contains("version"), "got: {err}");
}

#[test]
fn errors_on_invalid_toml() {
    let src = r#"
[package
name = "x"
"#;
    let err = TugManifest::parse(src).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn errors_on_git_dep_with_multiple_refs() {
    let src = r#"
[package]
name = "p"
version = "0.1.0"

[dependencies]
x = { git = "https://example.com/x.git", tag = "v1", rev = "abc" }
"#;
    let err = TugManifest::parse(src).unwrap_err();
    assert!(
        err.contains("tag") || err.contains("rev") || err.contains("one of"),
        "got: {err}"
    );
}

#[test]
fn errors_on_dep_with_no_source() {
    // Inline table dep needs at least one of: git, path, version
    let src = r#"
[package]
name = "p"
version = "0.1.0"

[dependencies]
x = { tag = "v1" }
"#;
    let err = TugManifest::parse(src).unwrap_err();
    assert!(
        err.contains("source") || err.contains("git") || err.contains("path"),
        "got: {err}"
    );
}

#[test]
fn errors_on_invalid_dependency_name() {
    // Names with weird characters not allowed.
    let src = r#"
[package]
name = "p"
version = "0.1.0"

[dependencies]
"has spaces" = "1.0.0"
"#;
    let err = TugManifest::parse(src).unwrap_err();
    assert!(
        err.contains("name") || err.contains("identifier"),
        "got: {err}"
    );
}

// ---- serialization round-trip ----

#[test]
fn serializes_minimal_manifest() {
    let m = TugManifest::new("hello", "0.1.0");
    let s = m.to_string();
    let re_parsed = TugManifest::parse(&s).unwrap();
    assert_eq!(re_parsed.name, "hello");
    assert_eq!(re_parsed.version, "0.1.0");
}

#[test]
fn round_trip_preserves_dependencies() {
    let src = r#"
[package]
name = "p"
version = "0.1.0"

[dependencies]
a = "1.2.3"
b = { git = "https://example.com/b.git", tag = "v2" }
c = { path = "../c" }
"#;
    let m = TugManifest::parse(src).unwrap();
    let s = m.to_string();
    let m2 = TugManifest::parse(&s).unwrap();
    assert_eq!(m.name, m2.name);
    assert_eq!(m.version, m2.version);
    assert_eq!(m.dependencies.len(), m2.dependencies.len());
    for orig in &m.dependencies {
        let found = m2
            .dependencies
            .iter()
            .find(|d| d.name == orig.name)
            .expect("dep in round-trip");
        assert_eq!(format!("{:?}", orig.source), format!("{:?}", found.source));
    }
}

#[test]
fn builder_adds_and_removes_deps() {
    let mut m = TugManifest::new("p", "0.1.0");
    m.add_dependency(Dependency::path("local", "../local"));
    m.add_dependency(Dependency::git_tag(
        "json",
        "https://github.com/x/oxy-json",
        "v1",
    ));
    assert_eq!(m.dependencies.len(), 2);

    assert!(m.remove_dependency("local"));
    assert_eq!(m.dependencies.len(), 1);
    assert_eq!(m.dependencies[0].name, "json");

    // Removing a missing dep returns false.
    assert!(!m.remove_dependency("not-there"));
}

#[test]
fn add_dependency_replaces_existing_with_same_name() {
    let mut m = TugManifest::new("p", "0.1.0");
    m.add_dependency(Dependency::version("x", "1.0.0"));
    m.add_dependency(Dependency::version("x", "2.0.0"));
    assert_eq!(m.dependencies.len(), 1);
    match &m.dependencies[0].source {
        oxy_tug::manifest::Source::Version(v) => assert_eq!(v, "2.0.0"),
        _ => panic!("expected Version source"),
    }
}
