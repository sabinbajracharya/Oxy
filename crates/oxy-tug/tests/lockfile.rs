//! Tests for `tug.lock` parsing and serialization.

use oxy_tug::lockfile::{LockedPackage, TugLock};

#[test]
fn parses_empty_lockfile() {
    let src = "version = 1\n";
    let lock = TugLock::parse(src).unwrap();
    assert_eq!(lock.version, 1);
    assert!(lock.packages.is_empty());
}

#[test]
fn parses_lockfile_with_packages() {
    let src = r#"
version = 1

[[package]]
name = "json"
source = "git+https://github.com/oxy-lang/oxy-json"
resolved = "abc1234567890abcdef1234567890abcdef123456"
checksum = "sha256:fake"

[[package]]
name = "local-pkg"
source = "path+../local-pkg"
resolved = "00000000"
"#;
    let lock = TugLock::parse(src).unwrap();
    assert_eq!(lock.version, 1);
    assert_eq!(lock.packages.len(), 2);
    let json = lock.packages.iter().find(|p| p.name == "json").unwrap();
    assert_eq!(json.source, "git+https://github.com/oxy-lang/oxy-json");
    assert_eq!(json.resolved, "abc1234567890abcdef1234567890abcdef123456");
    assert_eq!(json.checksum.as_deref(), Some("sha256:fake"));

    let local = lock
        .packages
        .iter()
        .find(|p| p.name == "local-pkg")
        .unwrap();
    assert!(local.checksum.is_none());
}

#[test]
fn errors_on_missing_version() {
    let src = r#"
[[package]]
name = "x"
source = "path+./x"
resolved = "0"
"#;
    let err = TugLock::parse(src).unwrap_err();
    assert!(err.contains("version"), "got: {err}");
}

#[test]
fn errors_on_unsupported_version() {
    let src = "version = 99\n";
    let err = TugLock::parse(src).unwrap_err();
    assert!(
        err.contains("unsupported lockfile version") || err.contains("99"),
        "got: {err}"
    );
}

#[test]
fn errors_on_package_missing_fields() {
    let src = r#"
version = 1

[[package]]
name = "x"
"#;
    let err = TugLock::parse(src).unwrap_err();
    assert!(
        err.contains("source") || err.contains("resolved"),
        "got: {err}"
    );
}

#[test]
fn round_trip_preserves_packages() {
    let lock = TugLock {
        version: 1,
        packages: vec![
            LockedPackage {
                name: "json".into(),
                source: "git+https://example.com/json.git".into(),
                resolved: "abc123".into(),
                checksum: Some("sha256:fake".into()),
            },
            LockedPackage {
                name: "local".into(),
                source: "path+../local".into(),
                resolved: "0".into(),
                checksum: None,
            },
        ],
    };
    let s = lock.to_string();
    let re = TugLock::parse(&s).unwrap();
    assert_eq!(re.version, 1);
    assert_eq!(re.packages.len(), 2);
    let json = re.packages.iter().find(|p| p.name == "json").unwrap();
    assert_eq!(json.resolved, "abc123");
    assert_eq!(json.checksum.as_deref(), Some("sha256:fake"));
}

#[test]
fn deterministic_order_in_serialized_output() {
    // Packages should be sorted by name for stable diffs in version control.
    let lock = TugLock {
        version: 1,
        packages: vec![
            LockedPackage {
                name: "z".into(),
                source: "x".into(),
                resolved: "1".into(),
                checksum: None,
            },
            LockedPackage {
                name: "a".into(),
                source: "x".into(),
                resolved: "1".into(),
                checksum: None,
            },
            LockedPackage {
                name: "m".into(),
                source: "x".into(),
                resolved: "1".into(),
                checksum: None,
            },
        ],
    };
    let s = lock.to_string();
    let a_pos = s.find("name = \"a\"").expect("a present");
    let m_pos = s.find("name = \"m\"").expect("m present");
    let z_pos = s.find("name = \"z\"").expect("z present");
    assert!(
        a_pos < m_pos && m_pos < z_pos,
        "expected alphabetical order"
    );
}

#[test]
fn lockfile_finds_package_by_name() {
    let lock = TugLock {
        version: 1,
        packages: vec![LockedPackage {
            name: "json".into(),
            source: "x".into(),
            resolved: "1".into(),
            checksum: None,
        }],
    };
    assert!(lock.find("json").is_some());
    assert!(lock.find("missing").is_none());
}
