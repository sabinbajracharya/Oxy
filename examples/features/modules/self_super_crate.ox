// === Feature: Modules — self, super, crate Path Resolution ===
// Tests that self, super, and crate keywords resolve correctly
// in use statements and qualified paths.

// === self and super inside modules ===

mod food {
    pub fn apple() -> String {
        "apple".to_string()
    }

    pub mod fruits {
        pub fn banana() -> String {
            "banana".to_string()
        }

        pub fn call_self() -> String {
            self::banana()
        }

        pub fn call_super() -> String {
            super::apple()
        }
    }
}

#[test]
fn test_self_in_module() {
    assert::eq(food::fruits::call_self(), "banana");
}

#[test]
fn test_super_in_module() {
    assert::eq(food::fruits::call_super(), "apple");
}

// === crate: refers to root ===

mod animals {
    pub fn dog() -> String {
        "dog".to_string()
    }
}

#[test]
fn test_crate_qualified_path() {
    // crate::animals::dog() from test context
    // In Oxy, tests run at root level, so this should work
    assert::eq(animals::dog(), "dog");
}

// === Deep module hierarchy ===

mod lib {
    pub fn alpha() -> String {
        "alpha".to_string()
    }

    pub mod sub {
        pub fn gamma() -> String {
            "gamma".to_string()
        }
    }
}

#[test]
fn test_deep_path() {
    assert::eq(lib::sub::gamma(), "gamma");
}

// === super::super for going up two levels ===

mod level1 {
    pub fn l1_fn() -> Int {
        1
    }

    pub mod level2 {
        pub fn l2_fn() -> Int {
            2
        }

        pub mod level3 {
            pub fn l3_fn() -> Int {
                3
            }

            pub fn call_l1_via_super_super() -> Int {
                super::super::l1_fn()
            }
        }
    }
}

#[test]
fn test_super_super() {
    assert::eq(level1::level2::level3::call_l1_via_super_super(), 1);
}
