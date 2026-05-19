// === Feature: Structs — Derive Macros ===
// `#[derive(...)]` auto-generates trait implementations.
// Supported: Debug, Clone, PartialEq, Default.

// === #[derive(Debug)] ===

#[derive(Debug)]
struct DebugPoint {
    x: i64,
    y: i64,
}

#[test]
fn test_derive_debug() {
    let p = DebugPoint { x: 1, y: 2 };
    let s = p.to_string();
    assert!(s.contains("DebugPoint"));
    assert!(s.contains("x"));
    assert!(s.contains("y"));
}

// === #[derive(Clone)] ===

#[derive(Clone)]
struct CloneData {
    value: i64,
}

#[test]
fn test_derive_clone() {
    let a = CloneData { value: 42 };
    let b = a.clone();
    assert_eq!(b.value, 42);
}

// === #[derive(PartialEq)] ===

#[derive(PartialEq)]
struct EqData {
    id: i64,
    name: String,
}

#[test]
fn test_derive_partial_eq_equal() {
    let a = EqData { id: 1, name: "hello" };
    let b = EqData { id: 1, name: "hello" };
    assert!(a == b);
}

#[test]
fn test_derive_partial_eq_not_equal() {
    let a = EqData { id: 1, name: "hello" };
    let b = EqData { id: 2, name: "hello" };
    assert!(a != b);
}

// === #[derive(Default)] ===

#[derive(Default)]
struct Config {
    port: i64,
    host: String,
    debug: bool,
}

#[test]
fn test_derive_default() {
    let c = Config::default();
    assert_eq!(c.port, 0);
    assert_eq!(c.host, "");
    assert_eq!(c.debug, false);
}

// === Multiple Derives ===

#[derive(Debug, Clone, PartialEq, Default)]
struct FullData {
    count: i64,
    label: String,
}

#[test]
fn test_multiple_derives() {
    let a = FullData { count: 10, label: "test" };
    let b = a.clone();
    assert!(a == b);

    let d = FullData::default();
    assert_eq!(d.count, 0);
    assert_eq!(d.label, "");
}
