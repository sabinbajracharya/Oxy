//! Built-in method dispatch for Oxy types.
//!
//! Each type has its own module with a single entry point.
//! Both the tree-walking interpreter and the bytecode VM route
//! method calls through these functions.

// ---------------------------------------------------------------------------
// Macros — must be defined before module declarations so child modules can
// use them (Rust macros are textually scoped; parent macros are visible to
// children declared later).
// ---------------------------------------------------------------------------

/// Generates `dispatch` and `method_names` for a map-like collection
/// (HashMap, BTreeMap).
macro_rules! map_dispatch {
    ($Variant:ident, $sym:ident, $type_label:literal, $sort_keys:expr) => {
        pub fn dispatch(
            receiver: $crate::types::Value,
            method: &str,
            args: &[$crate::types::Value],
        ) -> Result<$crate::types::Value, String> {
            let $crate::types::Value::$Variant(rc) = &receiver else {
                unreachable!()
            };
            let rc = rc.clone();
            match method {
                $crate::symbols::$sym::LEN => {
                    Ok($crate::types::Value::I64(rc.borrow().len() as i64))
                }
                $crate::symbols::$sym::IS_EMPTY => {
                    Ok($crate::types::Value::Bool(rc.borrow().is_empty()))
                }
                $crate::symbols::$sym::GET => {
                    let key = args.first().cloned().unwrap_or($crate::types::Value::Unit);
                    match rc.borrow().get(&key).cloned() {
                        Some(val) => Ok($crate::types::Value::some(val)),
                        None => Ok($crate::types::Value::none()),
                    }
                }
                $crate::symbols::$sym::GET_OR => {
                    let key = args.first().cloned().unwrap_or($crate::types::Value::Unit);
                    let default = args.get(1).cloned().unwrap_or($crate::types::Value::Unit);
                    match rc.borrow().get(&key).cloned() {
                        Some(val) => Ok(val),
                        None => Ok(default),
                    }
                }
                $crate::symbols::$sym::CONTAINS_KEY => {
                    let key = args.first().cloned().unwrap_or($crate::types::Value::Unit);
                    Ok($crate::types::Value::Bool(rc.borrow().contains_key(&key)))
                }
                $crate::symbols::$sym::INSERT => {
                    let key = args.first().cloned().unwrap_or($crate::types::Value::Unit);
                    let val = args.get(1).cloned().unwrap_or($crate::types::Value::Unit);
                    let old = rc.borrow_mut().insert(key, val);
                    match old {
                        Some(v) => Ok($crate::types::Value::some(v)),
                        None => Ok($crate::types::Value::none()),
                    }
                }
                $crate::symbols::$sym::REMOVE => {
                    let key = args.first().cloned().unwrap_or($crate::types::Value::Unit);
                    match rc.borrow_mut().remove(&key) {
                        Some(val) => Ok($crate::types::Value::some(val)),
                        None => Ok($crate::types::Value::none()),
                    }
                }
                $crate::symbols::$sym::KEYS => {
                    let mut keys: Vec<$crate::types::Value> = rc.borrow().keys().cloned().collect();
                    if $sort_keys {
                        keys.sort();
                    }
                    Ok($crate::types::Value::Vec(std::rc::Rc::new(
                        std::cell::RefCell::new(keys),
                    )))
                }
                $crate::symbols::$sym::VALUES => {
                    let mut values: Vec<$crate::types::Value> =
                        rc.borrow().values().cloned().collect();
                    if $sort_keys {
                        values.sort();
                    }
                    Ok($crate::types::Value::Vec(std::rc::Rc::new(
                        std::cell::RefCell::new(values),
                    )))
                }
                $crate::symbols::$sym::CLONE => Ok($crate::types::Value::$Variant(
                    std::rc::Rc::new(std::cell::RefCell::new(rc.borrow().clone())),
                )),
                _ => Err(format!("no method '{}' on type {}", method, $type_label)),
            }
        }

        pub fn method_names() -> &'static [&'static str] {
            &[
                $crate::symbols::$sym::LEN,
                $crate::symbols::$sym::IS_EMPTY,
                $crate::symbols::$sym::GET,
                $crate::symbols::$sym::GET_OR,
                $crate::symbols::$sym::CONTAINS_KEY,
                $crate::symbols::$sym::INSERT,
                $crate::symbols::$sym::REMOVE,
                $crate::symbols::$sym::KEYS,
                $crate::symbols::$sym::VALUES,
                $crate::symbols::$sym::CLONE,
            ]
        }
    };
}

/// Generates `dispatch` and `method_names` for a set-like collection
/// (HashSet, BTreeSet).
macro_rules! set_dispatch {
    ($Variant:ident, $RustType:ident, $sym:ident, $type_label:literal, $sort_to_vec:expr) => {
        pub fn dispatch(
            receiver: $crate::types::Value,
            method: &str,
            args: &[$crate::types::Value],
        ) -> Result<$crate::types::Value, String> {
            let $crate::types::Value::$Variant(rc) = &receiver else {
                unreachable!()
            };
            let rc = rc.clone();
            match method {
                $crate::symbols::$sym::LEN => {
                    Ok($crate::types::Value::I64(rc.borrow().len() as i64))
                }
                $crate::symbols::$sym::IS_EMPTY => {
                    Ok($crate::types::Value::Bool(rc.borrow().is_empty()))
                }
                $crate::symbols::$sym::CONTAINS => {
                    let val = args.first().cloned().unwrap_or($crate::types::Value::Unit);
                    Ok($crate::types::Value::Bool(rc.borrow().contains(&val)))
                }
                $crate::symbols::$sym::INSERT => {
                    let val = args.first().cloned().unwrap_or($crate::types::Value::Unit);
                    let was_new = rc.borrow_mut().insert(val);
                    Ok($crate::types::Value::Bool(was_new))
                }
                $crate::symbols::$sym::REMOVE => {
                    let val = args.first().cloned().unwrap_or($crate::types::Value::Unit);
                    let existed = rc.borrow_mut().remove(&val);
                    Ok($crate::types::Value::Bool(existed))
                }
                $crate::symbols::$sym::TO_VEC => {
                    let s = rc.borrow();
                    let mut v: Vec<$crate::types::Value> = s.iter().cloned().collect();
                    if $sort_to_vec {
                        v.sort();
                    }
                    Ok($crate::types::Value::Vec(std::rc::Rc::new(
                        std::cell::RefCell::new(v),
                    )))
                }
                $crate::symbols::$sym::UNION => {
                    let other = args.first().cloned().unwrap_or($crate::types::Value::Unit);
                    if let $crate::types::Value::$Variant(other_rc) = other {
                        let union: std::collections::$RustType<$crate::types::Value> =
                            rc.borrow().union(&other_rc.borrow()).cloned().collect();
                        Ok($crate::types::Value::$Variant(std::rc::Rc::new(
                            std::cell::RefCell::new(union),
                        )))
                    } else {
                        Err(concat!(
                            stringify!($RustType),
                            "::union requires a ",
                            stringify!($RustType),
                            " argument"
                        )
                        .into())
                    }
                }
                $crate::symbols::$sym::INTERSECTION => {
                    let other = args.first().cloned().unwrap_or($crate::types::Value::Unit);
                    if let $crate::types::Value::$Variant(other_rc) = other {
                        let intersection: std::collections::$RustType<$crate::types::Value> = rc
                            .borrow()
                            .intersection(&other_rc.borrow())
                            .cloned()
                            .collect();
                        Ok($crate::types::Value::$Variant(std::rc::Rc::new(
                            std::cell::RefCell::new(intersection),
                        )))
                    } else {
                        Err(concat!(
                            stringify!($RustType),
                            "::intersection requires a ",
                            stringify!($RustType),
                            " argument"
                        )
                        .into())
                    }
                }
                $crate::symbols::$sym::DIFFERENCE => {
                    let other = args.first().cloned().unwrap_or($crate::types::Value::Unit);
                    if let $crate::types::Value::$Variant(other_rc) = other {
                        let difference: std::collections::$RustType<$crate::types::Value> = rc
                            .borrow()
                            .difference(&other_rc.borrow())
                            .cloned()
                            .collect();
                        Ok($crate::types::Value::$Variant(std::rc::Rc::new(
                            std::cell::RefCell::new(difference),
                        )))
                    } else {
                        Err(concat!(
                            stringify!($RustType),
                            "::difference requires a ",
                            stringify!($RustType),
                            " argument"
                        )
                        .into())
                    }
                }
                $crate::symbols::$sym::CLONE => Ok($crate::types::Value::$Variant(
                    std::rc::Rc::new(std::cell::RefCell::new(rc.borrow().clone())),
                )),
                _ => Err(format!("no method '{}' on type {}", method, $type_label)),
            }
        }

        pub fn method_names() -> &'static [&'static str] {
            &[
                $crate::symbols::$sym::LEN,
                $crate::symbols::$sym::IS_EMPTY,
                $crate::symbols::$sym::CONTAINS,
                $crate::symbols::$sym::INSERT,
                $crate::symbols::$sym::REMOVE,
                $crate::symbols::$sym::TO_VEC,
                $crate::symbols::$sym::UNION,
                $crate::symbols::$sym::INTERSECTION,
                $crate::symbols::$sym::DIFFERENCE,
                $crate::symbols::$sym::CLONE,
            ]
        }
    };
}

pub mod binary_heap;
pub mod btreemap;
pub mod btreeset;
pub mod hashmap;
pub mod hashset;
pub mod iterator;
pub mod numeric;
pub mod option;
pub mod result;
pub mod string;
pub mod vec;
pub mod vec_deque;
