// === Feature: Structs — Derive Macros ===
// `#[derive(...)]` auto-generates trait implementations.
// Supported: Debug, Clone, PartialEq, Default.

// === #[derive(Debug)] ===

#[derive(Debug)]
struct DebugPoint {
    x: Int,
    y: Int,
}

#[test]
fn test_derive_debug() {
    val p = DebugPoint { x: 1, y: 2 };
    val s = p.to_string();
    assert(s.contains("DebugPoint"));
    assert(s.contains("x"));
    assert(s.contains("y"));
}

// === #[derive(Clone)] ===

#[derive(Clone)]
struct CloneData {
    value: Int,
}

#[test]
fn test_derive_clone() {
    val a = CloneData { value: 42 };
    val b = a.clone();
    assert_eq(b.value, 42);
}

// === #[derive(PartialEq)] ===

#[derive(PartialEq)]
struct EqData {
    id: Int,
    name: String,
}

#[test]
fn test_derive_partial_eq_equal() {
    val a = EqData { id: 1, name: "hello" };
    val b = EqData { id: 1, name: "hello" };
    assert(a == b);
}

#[test]
fn test_derive_partial_eq_not_equal() {
    val a = EqData { id: 1, name: "hello" };
    val b = EqData { id: 2, name: "hello" };
    assert(a != b);
}

// === #[derive(Default)] ===

#[derive(Default)]
struct Config {
    port: Int,
    host: String,
    debug: bool,
}

#[test]
fn test_derive_default() {
    val c = Config::default();
    assert_eq(c.port, 0);
    assert_eq(c.host, "");
    assert_eq(c.debug, false);
}

// === Multiple Derives ===

#[derive(Debug, Clone, PartialEq, Default)]
struct FullData {
    count: Int,
    label: String,
}

#[test]
fn test_multiple_derives() {
    val a = FullData { count: 10, label: "test" };
    val b = a.clone();
    assert(a == b);

    val d = FullData::default();
    assert_eq(d.count, 0);
    assert_eq(d.label, "");
}
