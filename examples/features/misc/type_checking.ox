// === Feature: Type Checking — Module-Aware Resolution ===
// Tests that the type checker infers concrete types using the
// module-aware resolution layer (struct_defs, use_aliases, etc.)

// === Field Access Returns Correct Type ===

mod data {
    pub struct Entry {
        pub name: String,
        pub count: int,
    }

    pub fn make_entry() -> Entry {
        Entry { name: "test".to_string(), count: 42 }
    }
}

use data::Entry;

#[test]
fn test_field_access_returns_string() {
    let e = data::make_entry();
    let n: String = e.name;
    assert_eq!(n, "test");
}

#[test]
fn test_field_access_returns_i64() {
    let e = data::make_entry();
    let c: int = e.count;
    assert_eq!(c, 42);
}

// === Self in Impl Blocks ===

mod shapes {
    pub struct Rect {
        pub w: int,
        pub h: int,
    }

    impl Rect {
        pub fn new(w: int, h: int) -> Rect {
            Rect { w, h }
        }

        pub fn area(self) -> int {
            self.w * self.h
        }
    }
}

use shapes::Rect;

#[test]
fn test_self_in_impl_block() {
    let r = Rect::new(5, 10);
    assert_eq!(r.area(), 50);
}

// === Struct Name Resolution Through Use Aliases ===

mod lib {
    pub struct Widget {
        pub label: String,
    }

    pub fn create() -> Widget {
        Widget { label: "ok".to_string() }
    }
}

use lib::Widget;

#[test]
fn test_struct_via_use_alias() {
    let w: Widget = lib::create();
    assert_eq!(w.label, "ok");
}

// === String Indexing ===

#[test]
fn test_string_index_type() {
    let s = "hello".to_string();
    let c: char = s[0];
    assert_eq!(c, 'h');
}

// === Type Annotation on PathCall Return ===

mod calc {
    pub fn value() -> int {
        100
    }
}

#[test]
fn test_path_call_return_type() {
    let v: int = calc::value();
    assert_eq!(v, 100);
}

// === Nested Field Access Chaining ===

mod outer_mod {
    pub struct Inner {
        pub val: int,
    }

    pub struct Outer {
        pub inner: Inner,
    }

    pub fn make() -> Outer {
        Outer { inner: Inner { val: 99 } }
    }
}

use outer_mod::Outer;

#[test]
fn test_nested_field_access() {
    let o = outer_mod::make();
    let v: int = o.inner.val;
    assert_eq!(v, 99);
}
