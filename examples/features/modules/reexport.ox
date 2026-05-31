// === Feature: Modules — pub use Re-exports ===
// Tests that `pub use` re-exports make items accessible as if
// they were defined in the re-exporting module.

mod inner {
    pub fn secret() -> String {
        "secret sauce".to_string()
    }

    pub fn recipe() -> String {
        "recipe".to_string()
    }

    pub struct Data {
        pub value: Int,
    }
}

mod public_api {
    // Re-export specific items from inner
    pub use inner::secret;
    pub use inner::Data;
}

#[test]
fn test_pub_use_simple_reexport() {
    use public_api::secret;
    assert_eq(secret(), "secret sauce");
}

#[test]
fn test_pub_use_reexport_struct() {
    use public_api::Data;
    val d = Data { value: 99 };
    assert_eq(d.value, 99);
}

// === pub use glob re-export ===

mod internal {
    pub fn alpha() -> String {
        "a".to_string()
    }

    pub fn beta() -> String {
        "b".to_string()
    }

    pub fn gamma() -> String {
        "c".to_string()
    }
}

mod facade {
    pub use internal::*;
}

#[test]
fn test_pub_use_glob_reexport() {
    use facade::alpha;
    use facade::beta;
    assert_eq(alpha(), "a");
    assert_eq(beta(), "b");
}

// === Re-export chain ===

mod layer1 {
    pub fn value() -> Int {
        42
    }
}

mod layer2 {
    pub use layer1::value;
}

mod layer3 {
    pub use layer2::value;
}

#[test]
fn test_reexport_chain() {
    use layer3::value;
    assert_eq(value(), 42);
}

// === Re-export with rename ===

mod original {
    pub fn long_function_name() -> String {
        "short".to_string()
    }
}

mod renamed {
    pub use original::long_function_name as short;
}

#[test]
fn test_pub_use_with_rename() {
    use renamed::short;
    assert_eq(short(), "short");
}
