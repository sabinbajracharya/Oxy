// === Feature: Modules — pub visibility ===
// Tests that pub items are accessible from root and sibling modules.

// === pub: accessible from root and sibling modules ===

mod crate_lib {
    pub fn get_value() -> Int {
        42
    }

    pub struct CrateData {
        pub x: Int,
    }

    pub fn make_data(v: Int) -> CrateData {
        CrateData { x: v }
    }
}

#[test]
fn test_pub_crate_fn_accessible_from_root() {
    assert::eq(crate_lib::get_value(), 42);
}

#[test]
fn test_pub_crate_struct_accessible_from_root() {
    val d = crate_lib::make_data(10);
    assert::eq(d.x, 10);
}

// === pub accessible from a different (sibling) module ===

mod other_mod {
    pub fn use_crate_lib() -> Int {
        crate_lib::get_value()
    }
}

#[test]
fn test_pub_crate_from_sibling_module() {
    assert::eq(other_mod::use_crate_lib(), 42);
}

// === pub: accessible from parent module ===

mod parent_lib {
    pub fn super_data() -> Int {
        100
    }

    pub struct SuperInfo {
        pub value: Int,
    }

    pub fn public_data() -> Int {
        200
    }
}

#[test]
fn test_pub_super_accessible_from_parent() {
    // Root is the parent of parent_lib, so pub items
    // should be visible from root.
    assert::eq(parent_lib::super_data(), 100);
}

#[test]
fn test_pub_super_struct_from_parent() {
    val s = parent_lib::SuperInfo { value: 77 };
    assert::eq(s.value, 77);
}

#[test]
fn test_regular_pub_still_works() {
    assert::eq(parent_lib::public_data(), 200);
}

// === pub: child modules can access parent's pub ===

mod container {
    pub fn parent_visible() -> Int {
        300
    }

    pub mod child {
        pub fn access_parent() -> Int {
            super::parent_visible()
        }
    }
}

#[test]
fn test_pub_super_from_child_via_super() {
    assert::eq(container::child::access_parent(), 300);
}
