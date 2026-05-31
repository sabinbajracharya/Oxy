// === Feature: Traits — Derive Macros ===
// `#[derive(...)]` auto-generates trait implementations for structs
// and enums. Currently supported: Default.

// === Derive Default on Named Struct ===

#[derive(Default)]
struct Point {
    x: Int,
    y: Int,
}

#[test]
fn test_derive_default_named() {
    val p = Point::default();
    assert_eq(p.x, 0);
    assert_eq(p.y, 0);
}

// === Derive Default with Explicit Override ===

#[derive(Default)]
struct Config {
    host: String,
    port: Int,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            host: "localhost".to_string(),
            port: 8080,
        }
    }
}

#[test]
fn test_derive_default_overridden() {
    val c = Config::default();
    assert_eq(c.host, "localhost");
    assert_eq(c.port, 8080);
}

// === Default on Multiple Structs ===

#[derive(Default)]
struct Pos3 {
    x: Float,
    y: Float,
    z: Float,
}

#[test]
fn test_derive_default_3d() {
    val p = Pos3::default();
    assert_eq(p.x, 0.0);
    assert_eq(p.y, 0.0);
    assert_eq(p.z, 0.0);
}

// === Derived Default Creates Zero Values ===

#[derive(Default)]
struct Mixed {
    int_val: Int,
    float_val: Float,
    string_val: String,
    bool_val: bool,
}

#[test]
fn test_derive_default_mixed() {
    val m = Mixed::default();
    assert_eq(m.int_val, 0);
    assert_eq(m.float_val, 0.0);
    assert_eq(m.string_val, "");
    assert_eq(m.bool_val, false);
}

// === Multiple Derive Attributes ===

#[derive(Default)]
struct Counter {
    value: Int,
}

#[test]
fn test_multiple_defaults() {
    val a = Counter::default();
    val b = Counter::default();
    assert_eq(a.value, 0);
    assert_eq(b.value, 0);
}
