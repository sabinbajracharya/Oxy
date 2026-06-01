// === Feature: Modules — Struct Field Visibility ===
// Tests that private fields on pub structs are NOT accessible from
// outside the defining module.

mod database {
    pub struct Record {
        pub name: String,
        secret_key: Int,  // private field
    }

    impl Record {
        // Public constructor — creates the struct (allowed inside module)
        pub fn new(name: String, key: Int) -> Record {
            Record { name, secret_key: key }
        }

        // Public accessor for private field
        pub fn get_key(self) -> Int {
            self.secret_key
        }
    }

    // Helper that internally accesses private fields
    pub fn make_admin() -> Record {
        Record { name: "admin".to_string(), secret_key: 9999 }
    }
}

#[test]
fn test_public_field_accessible() {
    use database::Record;
    val r = Record::new("user".to_string(), 1234);
    assert::eq(r.name, "user");
}

#[test]
fn test_private_field_via_accessor() {
    val r = database::make_admin();
    assert::eq(r.get_key(), 9999);
}

#[test]
fn test_public_constructor() {
    use database::Record;
    val r = Record::new("test".to_string(), 42);
    assert::eq(r.name, "test");
    assert::eq(r.get_key(), 42);
}

// === Struct with all public fields ===

mod geometry {
    pub struct Point {
        pub x: Float,
        pub y: Float,
    }

    pub fn origin() -> Point {
        Point { x: 0.0, y: 0.0 }
    }
}

#[test]
fn test_pub_fields_directly_accessible() {
    val p = geometry::origin();
    assert::eq(p.x, 0.0);
    assert::eq(p.y, 0.0);
}

#[test]
fn test_pub_struct_init_from_outside() {
    use geometry::Point;
    val p = Point { x: 1.0, y: 2.0 };
    assert::eq(p.x, 1.0);
    assert::eq(p.y, 2.0);
}

// === Mix of public and private fields ===

mod config {
    pub struct Settings {
        pub debug: bool,
        pub max_connections: Int,
        internal_counter: Int,
    }

    impl Settings {
        pub fn new(debug: bool, max: Int) -> Settings {
            Settings { debug, max_connections: max, internal_counter: 0 }
        }

        pub fn get_counter(self) -> Int {
            self.internal_counter
        }
    }
}

#[test]
fn test_mixed_visibility_via_constructor() {
    use config::Settings;
    val s = Settings::new(true, 10);
    assert::eq(s.debug, true);
    assert::eq(s.max_connections, 10);
    assert::eq(s.get_counter(), 0);
}

// === Negative tests: private fields NOT accessible from outside ===

#[compile_error]
fn test_cannot_read_private_field_from_outside() {
    val r = database::make_admin();
    val _ = r.secret_key; // ERROR: private field
}

#[compile_error]
fn test_cannot_init_private_field_from_outside() {
    use database::Record;
    val _ = Record { name: "x".to_string(), secret_key: 1 }; // ERROR: private field
}

#[compile_error]
fn test_cannot_read_mixed_private_field() {
    use config::Settings;
    val s = Settings::new(true, 10);
    val _ = s.internal_counter; // ERROR: private field
}

#[compile_error]
fn test_cannot_init_mixed_private_field() {
    use config::Settings;
    val _ = Settings { debug: true, max_connections: 5, internal_counter: 0 }; // ERROR: private field
}
