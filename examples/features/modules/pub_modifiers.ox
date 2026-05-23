// === Feature: Modules — pub(crate) and pub(super) Semantics ===
// Tests that pub(crate) is accessible within the same crate
// and pub(super) is accessible from the parent module.

// === pub(crate): accessible from root and sibling modules ===

mod crate_lib {
    pub(crate) fn get_value() -> int {
        42
    }

    pub(crate) struct CrateData {
        pub x: int,
    }

    pub(crate) fn make_data(v: int) -> CrateData {
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
    pub fn use_crate_lib() -> int {
        crate_lib::get_value()
    }
}

#[test]
fn test_pub_crate_from_sibling_module() {
    assert_eq!(other_mod::use_crate_lib(), 42);
}

// === pub(super): accessible from parent module ===

mod parent_lib {
    pub(super) fn super_data() -> int {
        100
    }

    pub(super) struct SuperInfo {
        pub val: int,
    }

    pub fn public_data() -> int {
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
    pub(super) fn parent_visible() -> int {
        300
    }

    pub mod child {
        pub fn access_parent() -> int {
            super::parent_visible()
        }
    }
}

#[test]
fn test_pub_super_from_child_via_super() {
    assert_eq!(container::child::access_parent(), 300);
}
