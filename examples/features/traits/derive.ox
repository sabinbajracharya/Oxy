// === Feature: Traits — Derive Macros ===
// `#[derive(...)]` auto-generates trait implementations for structs
// and enums. Currently supported: Default.

// === Derive Default on Named Struct ===

#[derive(Default)]
struct Point {
    x: i64,
    y: i64,
}

#[test]
fn test_derive_default_named() {
    let p = Point::default();
    assert_eq!(p.x, 0);
    assert_eq!(p.y, 0);
}

// === Derive Default with Explicit Override ===

#[derive(Default)]
struct Config {
    host: String,
    port: i64,
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
    let c = Config::default();
    assert_eq!(c.host, "localhost");
    assert_eq!(c.port, 8080);
}

// === Default on Multiple Structs ===

#[derive(Default)]
struct Pos3 {
    x: f64,
    y: f64,
    z: f64,
}

#[test]
fn test_derive_default_3d() {
    let p = Pos3::default();
    assert_eq!(p.x, 0.0);
    assert_eq!(p.y, 0.0);
    assert_eq!(p.z, 0.0);
}

// === Derived Default Creates Zero Values ===

#[derive(Default)]
struct Mixed {
    int_val: i64,
    float_val: f64,
    string_val: String,
    bool_val: bool,
}

#[test]
fn test_derive_default_mixed() {
    let m = Mixed::default();
    assert_eq!(m.int_val, 0);
    assert_eq!(m.float_val, 0.0);
    assert_eq!(m.string_val, "");
    assert_eq!(m.bool_val, false);
}

// === Multiple Derive Attributes ===

#[derive(Default)]
struct Counter {
    value: i64,
}

#[test]
fn test_multiple_defaults() {
    let a = Counter::default();
    let b = Counter::default();
    assert_eq!(a.value, 0);
    assert_eq!(b.value, 0);
}
