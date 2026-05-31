//! BTreeMap method implementations — shared by interpreter and VM.

use std::collections::BTreeMap;

use crate::types::Value;

map_dispatch!(BTreeMap, btreemap_m, "BTreeMap", false);

/// Helper to build a BTreeMap value from Rust types.
pub fn from_iter(entries: impl IntoIterator<Item = (Value, Value)>) -> Value {
    let mut m = BTreeMap::new();
    for (k, v) in entries {
        m.insert(k, v);
    }
    Value::BTreeMap(std::rc::Rc::new(std::cell::RefCell::new(m)))
}
