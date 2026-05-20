// === Feature: Modules — pub(crate) and pub(super) Semantics ===
// Tests that pub(crate) is accessible within the same crate
// and pub(super) is accessible from the parent module.

// === pub(crate): accessible from root and sibling modules ===

mod crate_lib {
    pub(crate) fn get_value() -> i64 {
        42
    }

    pub(crate) struct CrateData {
        pub x: i64,
    }

    pub(crate) fn make_data(v: i64) -> CrateData {
        CrateData { x: v }
    }
}

#[test]
fn test_pub_crate_fn_accessible_from_root() {
    assert_eq!(crate_lib::get_value(), 42);
}

#[test]
fn test_pub_crate_struct_accessible_from_root() {
    let d = crate_lib::make_data(10);
    assert_eq!(d.x, 10);
}

// === pub(crate) accessible from a different (sibling) module ===

mod other_mod {
    pub fn use_crate_lib() -> i64 {
        crate_lib::get_value()
    }
}

#[test]
fn test_pub_crate_from_sibling_module() {
    assert_eq!(other_mod::use_crate_lib(), 42);
}

// === pub(super): accessible from parent module ===

mod parent_lib {
    pub(super) fn super_data() -> i64 {
        100
    }

    pub(super) struct SuperInfo {
        pub val: i64,
    }

    pub fn public_data() -> i64 {
        200
    }
}

#[test]
fn test_pub_super_accessible_from_parent() {
    // Root is the parent of parent_lib, so pub(super) items
    // should be visible from root.
    assert_eq!(parent_lib::super_data(), 100);
}

#[test]
fn test_pub_super_struct_from_parent() {
    let s = parent_lib::SuperInfo { val: 77 };
    assert_eq!(s.val, 77);
}

#[test]
fn test_regular_pub_still_works() {
    assert_eq!(parent_lib::public_data(), 200);
}

// === pub(super): child modules can access parent's pub(super) ===

mod container {
    pub(super) fn parent_visible() -> i64 {
        300
    }

    pub mod child {
        pub fn access_parent() -> i64 {
            super::parent_visible()
        }
    }
}

#[test]
fn test_pub_super_from_child_via_super() {
    assert_eq!(container::child::access_parent(), 300);
}
