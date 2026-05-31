//! HashMap method implementations — shared by interpreter and VM.

use std::collections::HashMap;

use crate::types::Value;

map_dispatch!(HashMap, hashmap_m, "Map", true);

/// Helper to build a HashMap value from Rust types.
pub fn from_iter(entries: impl IntoIterator<Item = (Value, Value)>) -> Value {
    let mut m = HashMap::new();
    for (k, v) in entries {
        m.insert(k, v);
    }
    Value::HashMap(std::rc::Rc::new(std::cell::RefCell::new(m)))
}
