// === Feature: Private function call enforcement via use aliases ===
// Tests that private functions cannot be called through `use` aliases,
// `use module::*` glob imports, or direct qualified paths.

mod secret {
    pub fn public_fn() -> int {
        42
    }

    fn hidden() -> int {
        99
    }
}

// Direct `use` of public function — should work
use secret::public_fn;

#[test]
fn test_public_fn_via_use() {
    assert_eq(public_fn(), 42);
}

// Direct qualified path to public function — should work
#[test]
fn test_public_fn_direct_path() {
    assert_eq(secret::public_fn(), 42);
}

// === Negative tests: private functions ===

#[compile_error]
fn test_private_fn_via_use() {
    use secret::hidden;
    hidden(); // ERROR: use of private function should be rejected
}

#[compile_error]
fn test_private_fn_direct_path() {
    secret::hidden(); // ERROR: private function via direct path
}

// === Glob import of private function ===

#[compile_error]
fn test_private_fn_via_glob() {
    use secret::*;
    hidden(); // ERROR: glob should not import private functions
}

// === Private struct via use alias ===

mod warehouse {
    pub struct PublicItem {
        pub name: String,
    }

    struct PrivateItem {
        pub id: int,
    }

    pub fn make_private() -> PrivateItem {
        PrivateItem { id: 42 }
    }
}

#[test]
fn test_public_struct_via_use() {
    use warehouse::PublicItem;
    let item = PublicItem { name: "test".to_string() };
    assert_eq(item.name, "test");
}

#[compile_error]
fn test_private_struct_via_use() {
    use warehouse::PrivateItem;
    let _ = PrivateItem { id: 1 }; // ERROR: use of private struct
}
