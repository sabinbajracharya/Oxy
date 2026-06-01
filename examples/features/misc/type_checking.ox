// === Feature: Type Checking — Module-Aware Resolution ===
// Tests that the type checker infers concrete types using the
// module-aware resolution layer (struct_defs, use_aliases, etc.)

// === Field Access Returns Correct Type ===

mod data {
    pub struct Entry {
        pub name: String,
        pub count: Int,
    }

    pub fn make_entry() -> Entry {
        Entry { name: "test".to_string(), count: 42 }
    }
}

use data::Entry;

#[test]
fn test_field_access_returns_string() {
    val e = data::make_entry();
    val n: String = e.name;
    assert::eq(n, "test");
}

#[test]
fn test_field_access_returns_i64() {
    val e = data::make_entry();
    val c: Int = e.count;
    assert::eq(c, 42);
}

// === Self in Impl Blocks ===

mod shapes {
    pub struct Rect {
        pub w: Int,
        pub h: Int,
    }

    impl Rect {
        pub fn new(w: Int, h: Int) -> Rect {
            Rect { w, h }
        }

        pub fn area(self) -> Int {
            self.w * self.h
        }
    }
}

use shapes::Rect;

#[test]
fn test_self_in_impl_block() {
    val r = Rect::new(5, 10);
    assert::eq(r.area(), 50);
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
    val w: Widget = lib::create();
    assert::eq(w.label, "ok");
}

// === String Indexing ===

#[test]
fn test_string_index_type() {
    val s = "hello".to_string();
    val c: char = s[0];
    assert::eq(c, 'h');
}

// === Type Annotation on PathCall Return ===

mod calc {
    pub fn value() -> Int {
        100
    }
}

#[test]
fn test_path_call_return_type() {
    val v: Int = calc::value();
    assert::eq(v, 100);
}

// === Nested Field Access Chaining ===

mod outer_mod {
    pub struct Inner {
        pub value: Int,
    }

    pub struct Outer {
        pub inner: Inner,
    }

    pub fn make() -> Outer {
        Outer { inner: Inner { value: 99 } }
    }
}

use outer_mod::Outer;

#[test]
fn test_nested_field_access() {
    val o = outer_mod::make();
    val v: Int = o.inner.value;
    assert::eq(v, 99);
}
