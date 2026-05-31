//! Consistency tests between `symbols.rs` and actual builtins/lexer/type system.
//!
//! These tests ensure the canonical symbol definitions stay in sync with the
//! compiler implementation. Adding a method to a builtins dispatch without
//! updating `symbols.rs` will cause a test failure.

use oxy_core::lexer::TokenKind;
use oxy_core::symbols;
use oxy_core::types::Value;
use oxy_core::vm::builtins;

// ---------------------------------------------------------------------------
// Helper: collect method names from symbols::ALL_TYPES for a given type
// ---------------------------------------------------------------------------

fn symbols_methods(type_name: &str) -> Vec<&'static str> {
    for ty in symbols::ALL_TYPES {
        if ty.name == type_name {
            return ty.methods.iter().map(|m| m.name).collect();
        }
    }
    if type_name == "struct" || type_name == "enum" || type_name == "tuple" {
        return symbols::GENERIC_TYPE_METHODS
            .iter()
            .map(|m| m.name)
            .collect();
    }
    vec![]
}

// ---------------------------------------------------------------------------
// Test 1: Every method in builtins dispatch exists in symbols
// ---------------------------------------------------------------------------

fn check_builtins_in_symbols(builtin_methods: &[&str], type_name: &str) {
    let sym = symbols_methods(type_name);
    for m in builtin_methods {
        assert!(
            sym.contains(m),
            "builtins::{t} method '{m}' missing from symbols::ALL_TYPES",
            t = type_name,
            m = m
        );
    }
}

#[test]
fn test_string_methods_in_symbols() {
    check_builtins_in_symbols(builtins::string::method_names(), "String");
}

#[test]
fn test_vec_methods_in_symbols() {
    check_builtins_in_symbols(builtins::vec::method_names(), "List");
}

#[test]
fn test_hashmap_methods_in_symbols() {
    check_builtins_in_symbols(builtins::hashmap::method_names(), "Map");
}

#[test]
fn test_hashset_methods_in_symbols() {
    check_builtins_in_symbols(builtins::hashset::method_names(), "Set");
}

#[test]
fn test_btreemap_methods_in_symbols() {
    check_builtins_in_symbols(builtins::btreemap::method_names(), "BTreeMap");
}

#[test]
fn test_btreeset_methods_in_symbols() {
    check_builtins_in_symbols(builtins::btreeset::method_names(), "BTreeSet");
}

#[test]
fn test_binaryheap_methods_in_symbols() {
    check_builtins_in_symbols(builtins::binary_heap::method_names(), "BinaryHeap");
}

#[test]
fn test_vecdeque_methods_in_symbols() {
    check_builtins_in_symbols(builtins::vec_deque::method_names(), "VecDeque");
}

#[test]
fn test_iterator_methods_in_symbols() {
    check_builtins_in_symbols(builtins::iterator::method_names(), "Iterator");
}

#[test]
fn test_option_methods_in_symbols() {
    check_builtins_in_symbols(builtins::option::method_names(), "Option");
}

#[test]
fn test_result_methods_in_symbols() {
    check_builtins_in_symbols(builtins::result::method_names(), "Result");
}

#[test]
fn test_numeric_methods_in_symbols() {
    check_builtins_in_symbols(builtins::numeric::method_names(), "numeric");
}

// ---------------------------------------------------------------------------
// Test 2: Every method in symbols exists in builtins dispatch (reverse check)
// ---------------------------------------------------------------------------

fn check_symbols_in_builtins(builtin_methods: &[&str], type_name: &str) {
    let sym = symbols_methods(type_name);
    for m in &sym {
        assert!(
            builtin_methods.contains(m),
            "symbols::ALL_TYPES[{t}].methods '{m}' not found in builtins dispatch",
            t = type_name,
            m = m
        );
    }
}

#[test]
fn test_symbols_in_string_builtins() {
    check_symbols_in_builtins(builtins::string::method_names(), "String");
}

#[test]
fn test_symbols_in_vec_builtins() {
    check_symbols_in_builtins(builtins::vec::method_names(), "List");
}

#[test]
fn test_symbols_in_hashmap_builtins() {
    check_symbols_in_builtins(builtins::hashmap::method_names(), "Map");
}

#[test]
fn test_symbols_in_hashset_builtins() {
    check_symbols_in_builtins(builtins::hashset::method_names(), "Set");
}

#[test]
fn test_symbols_in_btreemap_builtins() {
    check_symbols_in_builtins(builtins::btreemap::method_names(), "BTreeMap");
}

#[test]
fn test_symbols_in_btreeset_builtins() {
    check_symbols_in_builtins(builtins::btreeset::method_names(), "BTreeSet");
}

#[test]
fn test_symbols_in_binaryheap_builtins() {
    check_symbols_in_builtins(builtins::binary_heap::method_names(), "BinaryHeap");
}

#[test]
fn test_symbols_in_vecdeque_builtins() {
    check_symbols_in_builtins(builtins::vec_deque::method_names(), "VecDeque");
}

#[test]
fn test_symbols_in_iterator_builtins() {
    check_symbols_in_builtins(builtins::iterator::method_names(), "Iterator");
}

#[test]
fn test_symbols_in_option_builtins() {
    check_symbols_in_builtins(builtins::option::method_names(), "Option");
}

#[test]
fn test_symbols_in_result_builtins() {
    check_symbols_in_builtins(builtins::result::method_names(), "Result");
}

#[test]
fn test_symbols_in_numeric_builtins() {
    check_symbols_in_builtins(builtins::numeric::method_names(), "numeric");
}

// ---------------------------------------------------------------------------
// Test 3: Keywords match lexer
// ---------------------------------------------------------------------------

#[test]
fn test_keywords_match_lexer() {
    for kw in symbols::KEYWORDS {
        assert!(
            TokenKind::from_keyword(kw).is_some(),
            "symbols::KEYWORDS contains '{kw}' but lexer TokenKind::from_keyword returns None"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 4: Type name constants match Value::type_name()
// ---------------------------------------------------------------------------

#[test]
fn test_type_name_constants() {
    assert_eq!(Value::I64(0).type_name(), symbols::I64_TYPE);
    assert_eq!(Value::F64(0.0).type_name(), symbols::F64_TYPE);
    assert_eq!(
        Value::String(String::new()).type_name(),
        symbols::STRING_TYPE
    );
    assert_eq!(Value::Bool(true).type_name(), symbols::BOOL_TYPE);
    assert_eq!(Value::Char('a').type_name(), symbols::CHAR_TYPE);
    assert_eq!(Value::Unit.type_name(), symbols::UNIT_TYPE);

    let some_val = Value::some(Value::Unit);
    let none_val = Value::none();
    assert_eq!(some_val.type_name(), symbols::OPTION_TYPE);
    assert_eq!(none_val.type_name(), symbols::OPTION_TYPE);

    let ok_val = Value::ok(Value::Unit);
    let err_val = Value::err(Value::String("e".into()));
    assert_eq!(ok_val.type_name(), symbols::RESULT_TYPE);
    assert_eq!(err_val.type_name(), symbols::RESULT_TYPE);
}

// ---------------------------------------------------------------------------
// Test 5: Primitive types list is non-empty and includes key types
// ---------------------------------------------------------------------------

#[test]
fn test_primitive_types_not_empty() {
    assert!(!symbols::PRIMITIVE_TYPES.is_empty());
    let names: Vec<&str> = symbols::PRIMITIVE_TYPES.iter().map(|(n, _)| *n).collect();
    assert!(names.contains(&"Int"));
    assert!(names.contains(&"Float"));
    assert!(names.contains(&"String"));
    assert!(names.contains(&"List"));
    assert!(names.contains(&"bool"));
}

// ---------------------------------------------------------------------------
// Test 6: Module list includes expected entries
// ---------------------------------------------------------------------------

#[test]
fn test_modules_not_empty() {
    assert!(!symbols::ALL_MODULES.is_empty());
    let paths: Vec<&str> = symbols::ALL_MODULES.iter().map(|m| m.path).collect();
    assert!(paths.contains(&"json::"));
    assert!(paths.contains(&"std::fs::"));
    assert!(paths.contains(&"math::"));
}

/// Every module the LSP advertises in `symbols::ALL_MODULES` must be a real
/// module registered for runtime dispatch in `stdlib::registry::MODULES`.
/// Otherwise completion would offer a `module::` path that fails to resolve at
/// runtime. (The reverse is intentionally *not* required: `io`/`args`/`path`
/// dispatch at runtime but are deliberately kept out of completions.)
#[test]
fn test_all_modules_resolve_in_registry() {
    use oxy_core::stdlib::registry;
    for m in symbols::ALL_MODULES {
        // Normalize the completion path (`std::fs::`, `json::`) to the bare
        // dispatch name (`fs`, `json`) the registry keys on.
        let bare = m.path.trim_end_matches("::");
        let bare = bare.strip_prefix("std::").unwrap_or(bare);
        assert!(
            registry::lookup_module(bare).is_some(),
            "symbols::ALL_MODULES advertises `{}` (bare `{bare}`), but no such \
             module is registered in stdlib::registry::MODULES",
            m.path
        );
    }
}

// ---------------------------------------------------------------------------
// Test 7: Keyword list includes core keywords
// ---------------------------------------------------------------------------

#[test]
fn test_keywords_not_empty() {
    assert!(!symbols::KEYWORDS.is_empty());
    assert!(symbols::KEYWORDS.contains(&"let"));
    assert!(symbols::KEYWORDS.contains(&"fn"));
    assert!(symbols::KEYWORDS.contains(&"struct"));
    assert!(symbols::KEYWORDS.contains(&"match"));
    assert!(symbols::KEYWORDS.contains(&"return"));
}

// ---------------------------------------------------------------------------
// Test 8: ALL_MACROS includes known macros
// ---------------------------------------------------------------------------

#[test]
fn test_macros_not_empty() {
    assert!(!symbols::ALL_MACROS.is_empty());
    let names: Vec<&str> = symbols::ALL_MACROS.iter().map(|m| m.name).collect();
    assert!(names.contains(&"println"));
    assert!(names.contains(&"format"));
    assert!(names.contains(&"panic"));
}

// ---------------------------------------------------------------------------
// Test 9: Dispatch registry — every VM dispatch type has a symbols entry,
//         and every symbols type has a VM dispatch (catches new builtin types
//         added without updating symbols or vice versa).
// ---------------------------------------------------------------------------

/// Map a dispatch type name (from `dispatched_type_names()`) to its symbols
/// lookup key in `ALL_TYPES` / generic methods.
fn dispatch_to_symbols_type(disp: &str) -> &str {
    match disp {
        "char" => "char",
        "numeric" => "numeric",
        "enum" | "struct" | "tuple" => disp, // covered by GENERIC_TYPE_METHODS
        _ => disp,
    }
}

#[test]
fn test_dispatched_types_in_symbols() {
    let all_type_names: Vec<&str> = symbols::ALL_TYPES.iter().map(|t| t.name).collect();
    let generic_names = ["enum", "struct", "tuple"];

    for disp in oxy_core::vm::dispatched_type_names() {
        let sym = dispatch_to_symbols_type(disp);

        let found = all_type_names.contains(&sym) || generic_names.contains(&sym);
        assert!(
            found,
            "VM dispatch type '{disp}' not found in symbols::ALL_TYPES or generic methods"
        );
    }
}

#[test]
fn test_symbols_types_have_dispatch() {
    let dispatched: Vec<&str> = oxy_core::vm::dispatched_type_names();
    let generic_type_names = ["enum", "struct", "tuple"];

    for ty in symbols::ALL_TYPES {
        // Map symbols name back to dispatch name
        let disp = ty.name;
        let found = dispatched.contains(&disp) || generic_type_names.contains(&ty.name);
        assert!(
            found,
            "symbols::ALL_TYPES type '{}' has no VM dispatch arm",
            ty.name
        );
    }
}
