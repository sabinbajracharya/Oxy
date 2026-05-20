// === Feature: Modules — pub(crate) and pub(super) Modifiers ===
// Tests that pub(crate) is accessible within the same compilation unit
// and pub(super) is accessible from the parent module.

// === pub(crate): accessible from anywhere in the same crate ===

mod crate_vis {
    pub(crate) fn crate_level_fn() -> i64 {
        100
    }

    pub fn regular_pub_fn() -> i64 {
        200
    }

    pub(crate) struct CrateStruct {
        pub x: i64,
    }
}

#[test]
fn test_pub_crate_fn_from_root() {
    assert_eq!(crate_vis::crate_level_fn(), 100);
}

#[test]
fn test_pub_crate_struct() {
    let s = crate_vis::CrateStruct { x: 42 };
    assert_eq!(s.x, 42);
}

// === pub(super): accessible from parent module ===

mod parent_mod {
    pub(super) fn super_level_fn() -> i64 {
        300
    }

    pub(super) struct SuperStruct {
        pub val: i64,
    }
}

#[test]
fn test_pub_super_from_root() {
    assert_eq!(parent_mod::super_level_fn(), 300);
}

#[test]
fn test_pub_super_struct() {
    let s = parent_mod::SuperStruct { val: 77 };
    assert_eq!(s.val, 77);
}

// === pub(crate) in nested module ===

mod outer_crate {
    pub(crate) mod inner_crate {
        pub fn deep_fn() -> i64 {
            400
        }
    }
}

#[test]
fn test_pub_crate_nested_module() {
    assert_eq!(outer_crate::inner_crate::deep_fn(), 400);
}

// === pub(super) visible from root ===

mod sibling_a {
    pub(super) fn shared_fn() -> i64 {
        555
    }
}

#[test]
fn test_pub_super_visible_from_root() {
    assert_eq!(sibling_a::shared_fn(), 555);
}
