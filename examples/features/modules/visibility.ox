// === Feature: Modules — Visibility Enforcement ===
// Tests that `pub` items are accessible from outside the module
// and private items are not (can only be accessed through public wrappers).

mod api {
    pub fn public_fn() -> i64 {
        42
    }

    fn private_fn() -> i64 {
        99
    }

    pub struct PublicStruct {
        pub x: i64,
    }

    struct PrivateStruct {
        pub x: i64,
    }

    pub fn call_private() -> i64 {
        private_fn()
    }

    pub fn make_private_struct() -> PrivateStruct {
        PrivateStruct { x: 7 }
    }

    pub fn get_private_x(s: PrivateStruct) -> i64 {
        s.x
    }
}

#[test]
fn test_public_fn_accessible() {
    assert_eq!(api::public_fn(), 42);
}

#[test]
fn test_private_fn_not_directly_accessible() {
    // Access private fn through public wrapper
    assert_eq!(api::call_private(), 99);
}

#[test]
fn test_private_struct_via_wrapper() {
    let s = api::make_private_struct();
    assert_eq!(api::get_private_x(s), 7);
}

// === Visibility with nested modules ===

mod parent {
    pub fn parent_fn() -> i64 {
        10
    }

    pub mod child {
        pub fn child_fn() -> i64 {
            20
        }
    }

    mod private_child {
        pub fn hidden_fn() -> i64 {
            30
        }
    }

    pub fn access_private_child() -> i64 {
        private_child::hidden_fn()
    }
}

#[test]
fn test_nested_pub_access() {
    assert_eq!(parent::child::child_fn(), 20);
}

#[test]
fn test_private_child_via_wrapper() {
    assert_eq!(parent::access_private_child(), 30);
}

// === Impl methods on structs with private fields ===

mod store {
    pub struct Item {
        pub name: String,
        price: i64,
    }

    impl Item {
        pub fn new(name: String, price: i64) -> Item {
            Item { name, price }
        }

        pub fn get_price(self) -> i64 {
            self.price
        }
    }
}

#[test]
fn test_pub_struct_with_private_fields() {
    let item = store::Item::new("widget".to_string(), 100);
    assert_eq!(item.name, "widget");
    assert_eq!(item.get_price(), 100);
}
