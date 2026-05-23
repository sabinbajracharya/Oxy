//! Canonical symbol definitions for the Oxy language.
//!
//! This module is the **single source of truth** for:
//! - Keywords
//! - Built-in type names, descriptions, and their methods
//! - Built-in macros
//! - Standard library module paths
//! - Primitive type names
//!
//! Both the compiler/VM and the LSP server import from here.
//! Adding a new type, method, or keyword **must** update this file or
//! consistency tests will fail.

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// A method on a built-in type.
pub struct MethodInfo {
    pub name: &'static str,
    /// Short description for completions, e.g. "(sep: String) -> String"
    pub detail: &'static str,
    /// Markdown hover documentation.
    pub hover_text: &'static str,
}

/// A built-in type with its methods.
pub struct TypeInfo {
    pub name: &'static str,
    /// One-line description for completions.
    pub detail: &'static str,
    /// Markdown hover documentation.
    pub hover_text: &'static str,
    /// Methods available on this type.
    pub methods: &'static [MethodInfo],
}

/// A built-in macro (e.g. `println!`).
pub struct MacroInfo {
    pub name: &'static str,
    /// One-line description for completions.
    pub detail: &'static str,
    /// Markdown hover documentation.
    pub hover_text: &'static str,
}

/// A standard library module.
pub struct ModuleInfo {
    /// Path prefix shown in completions (e.g. "std::fs::", "json::").
    pub path: &'static str,
    /// One-line description.
    pub detail: &'static str,
}

// ---------------------------------------------------------------------------
// Keywords (from lexer/token.rs TokenKind::from_keyword)
// ---------------------------------------------------------------------------

pub const KEYWORDS: &[&str] = &[
    "let", "mut", "fn", "return", "if", "else", "while", "loop", "for", "in", "break", "continue",
    "struct", "enum", "impl", "trait", "match", "pub", "use", "mod", "self", "Self", "as", "ref",
    "const", "static", "type", "where", "move", "async", "await", "dyn", "true", "false", "super",
    "crate",
];

/// Markdown hover text for each keyword. Only includes keywords with useful docs.
pub fn keyword_hover_text(kw: &str) -> Option<&'static str> {
    match kw {
        "let" => Some("Bind a value to a variable.\n\n```oxy\nlet x = 42;\nlet mut y = 0;\n```"),
        "mut" => Some("Mark a variable as mutable."),
        "fn" => Some("Declare a function.\n\n```oxy\nfn add(a: i64, b: i64) -> i64 { a + b }\n```"),
        "struct" => Some("Define a struct type.\n\n```oxy\nstruct Point { x: f64, y: f64 }\n```"),
        "enum" => Some("Define an enum type.\n\n```oxy\nenum Color { Red, Green, Blue }\n```"),
        "impl" => Some("Implement methods on a type."),
        "trait" => Some("Define a trait (interface)."),
        "if" => Some("Conditional branching."),
        "else" => Some("Alternative branch of an `if` expression."),
        "while" => Some("Loop while a condition is true."),
        "loop" => Some("Loop forever (until `break`)."),
        "for" => Some("Iterate over a range or collection.\n\n```oxy\nfor i in 0..10 { println!(\"{}\", i); }\n```"),
        "in" => Some("Used in `for` loops to specify the iterator."),
        "match" => Some("Pattern matching.\n\n```oxy\nmatch value { 1 => \"one\", _ => \"other\" }\n```"),
        "return" => Some("Return a value from a function."),
        "break" => Some("Exit a loop."),
        "continue" => Some("Skip to the next loop iteration."),
        "pub" => Some("Mark an item as public."),
        "mod" => Some("Define or reference a module."),
        "use" => Some("Import items from a module."),
        "const" => Some("Declare a compile-time constant."),
        "static" => Some("Declare a static variable."),
        "type" => Some("Create a type alias."),
        "async" => Some("Mark a function as asynchronous."),
        "await" => Some("Await an async expression."),
        "move" => Some("Move captured variables into a closure."),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Well-known type name constants (used by compiler, LSP, and type system)
// ---------------------------------------------------------------------------

/// Canonical name for Oxy's only signed integer type (`int` = 64-bit
/// internally). The constant name keeps `I64` to mirror the underlying
/// `IntegerWidth::I64`/`Value::I64` storage, but the *string* is `"int"`.
pub const I64_TYPE: &str = "int";
pub const BYTE_TYPE: &str = "byte";
/// Canonical name for Oxy's only float type (`float` = 64-bit internally).
pub const F64_TYPE: &str = "float";
pub const STRING_TYPE: &str = "String";
pub const BOOL_TYPE: &str = "bool";
pub const CHAR_TYPE: &str = "char";
pub const OPTION_TYPE: &str = "Option";
pub const RESULT_TYPE: &str = "Result";
pub const VEC_TYPE: &str = "Vec";
pub const HASHMAP_TYPE: &str = "HashMap";
pub const HASHSET_TYPE: &str = "HashSet";
pub const BTREEMAP_TYPE: &str = "BTreeMap";
pub const BTREESET_TYPE: &str = "BTreeSet";
pub const ITERATOR_TYPE: &str = "Iterator";
pub const UNIT_TYPE: &str = "()";

// ---------------------------------------------------------------------------
// Primitive type names (shown as completions)
// ---------------------------------------------------------------------------

pub const PRIMITIVE_TYPES: &[(&str, &str)] = &[
    ("int", "Signed integer (64-bit wrapping)"),
    ("byte", "Unsigned 8-bit integer (wraps modulo 256)"),
    ("float", "64-bit floating point"),
    ("bool", "Boolean type (true / false)"),
    ("char", "Unicode scalar value"),
    ("String", "Owned UTF-8 string"),
    ("Vec", "Growable array type"),
    ("HashMap", "Hash map collection"),
    ("HashSet", "Hash set collection"),
    ("BTreeMap", "Ordered map collection"),
    ("BTreeSet", "Ordered set collection"),
    ("Option", "Optional value: Some(T) or None"),
    ("Result", "Result type: Ok(T) or Err(E)"),
    ("Self", "Current type in impl block"),
];

// ---------------------------------------------------------------------------
// Built-in macros
// ---------------------------------------------------------------------------

pub const ALL_MACROS: &[MacroInfo] = &[
    MacroInfo {
        name: "println!",
        detail: "Print with newline",
        hover_text: "**println!(fmt, ...)** — Print to stdout with a newline",
    },
    MacroInfo {
        name: "print!",
        detail: "Print without newline",
        hover_text: "**print!(fmt, ...)** — Print to stdout without a newline",
    },
    MacroInfo {
        name: "format!",
        detail: "Format a string",
        hover_text: "**format!(fmt, ...)** — Format into a String",
    },
    MacroInfo {
        name: "eprintln!",
        detail: "Print to stderr",
        hover_text: "**eprintln!(fmt, ...)** — Print to stderr with a newline",
    },
    MacroInfo {
        name: "dbg!",
        detail: "Debug print",
        hover_text: "**dbg!(expr)** — Debug-print an expression and return it",
    },
    MacroInfo {
        name: "panic!",
        detail: "Panic with message",
        hover_text: "**panic!(msg)** — Abort with an error message",
    },
    MacroInfo {
        name: "todo!",
        detail: "Mark unfinished code",
        hover_text: "**todo!()** — Mark unfinished code (panics at runtime)",
    },
    MacroInfo {
        name: "unimplemented!",
        detail: "Mark unimplemented code",
        hover_text: "**unimplemented!()** — Mark unimplemented code (panics at runtime)",
    },
    MacroInfo {
        name: "vec!",
        detail: "Create a Vec",
        hover_text: "**vec![items...]** — Create a Vec from elements",
    },
];

// ---------------------------------------------------------------------------
// Method lists (alphabetical within each type)
// ---------------------------------------------------------------------------

macro_rules! methods {
    ($($name:literal: $detail:expr => $hover:expr),* $(,)?) => {
        &[$(MethodInfo { name: $name, detail: $detail, hover_text: $hover }),*]
    };
}

// --- String ---

pub const STRING_METHODS: &[MethodInfo] = methods![
    "char_at": "(i: i64) -> char" => "Return the character at index `i`.",
    "chars": "() -> Vec<char>" => "Return a Vec of characters.",
    "clone": "() -> String" => "Create a copy of the string.",
    "contains": "(pat: String) -> bool" => "Check if the string contains `pat`.",
    "ends_with": "(pat: String) -> bool" => "Check if the string ends with `pat`.",
    "is_empty": "() -> bool" => "Check if the string is empty.",
    "len": "() -> i64" => "Return the number of characters.",
    "parse_float": "() -> Option<f64>" => "Parse the string as an f64.",
    "parse_int": "() -> Option<i64>" => "Parse the string as an i64.",
    "push_str": "(s: String)" => "Append a string (note: strings are immutable in Oxy).",
    "repeat": "(n: i64) -> String" => "Repeat the string `n` times.",
    "replace": "(from: String, to: String) -> String" => "Replace all occurrences of `from` with `to`.",
    "split": "(pat: String) -> Vec<String>" => "Split the string by `pat`.",
    "starts_with": "(pat: String) -> bool" => "Check if the string starts with `pat`.",
    "substring": "(start: i64, end: i64) -> String" => "Return the substring from `start` to `end`.",
    "to_lowercase": "() -> String" => "Convert the string to lowercase.",
    "to_string": "() -> String" => "Convert to String.",
    "to_uppercase": "() -> String" => "Convert the string to uppercase.",
    "trim": "() -> String" => "Remove leading and trailing whitespace.",
];

// --- Vec ---

pub const VEC_METHODS: &[MethodInfo] = methods![
    "chunks": "(size: i64) -> Vec<Vec<T>>" => "Split into chunks of `size` elements.",
    "clear": "()" => "Remove all elements.",
    "clone": "() -> Vec<T>" => "Create a shallow copy.",
    "contains": "(val: T) -> bool" => "Check if the Vec contains `val`.",
    "dedup": "()" => "Remove consecutive duplicate elements.",
    "extend": "(other: Vec<T>)" => "Append all elements from `other`.",
    "first": "() -> Option<T>" => "Return the first element, or None if empty.",
    "get": "(i: i64) -> Option<T>" => "Return the element at index `i`, or None.",
    "insert": "(i: i64, val: T)" => "Insert `val` at index `i`.",
    "is_empty": "() -> bool" => "Check if the Vec is empty.",
    "iter": "() -> Iterator" => "Return an iterator over the elements.",
    "join": "(sep: String) -> String" => "Join the string representations with `sep`.",
    "last": "() -> Option<T>" => "Return the last element, or None if empty.",
    "len": "() -> i64" => "Return the number of elements.",
    "max": "() -> Option<T>" => "Return the maximum element, or None if empty.",
    "min": "() -> Option<T>" => "Return the minimum element, or None if empty.",
    "pop": "() -> Option<T>" => "Remove and return the last element.",
    "push": "(val: T)" => "Add an element to the end.",
    "remove": "(i: i64) -> Option<T>" => "Remove and return the element at index `i`.",
    "rev": "()" => "Reverse the order of elements in place.",
    "reverse": "()" => "Reverse the order of elements in place.",
    "sort": "()" => "Sort the elements in ascending order.",
    "sort_by": "(fn: (T, T) -> i64)" => "Sort using a custom comparator closure.",
    "sort_by_key": "(fn: (T) -> K)" => "Sort by the key produced by the closure.",
    "windows": "(size: i64) -> Vec<Vec<T>>" => "Return sliding windows of `size` elements.",
];

// --- HashMap ---

pub const HASHMAP_METHODS: &[MethodInfo] = methods![
    "clone": "() -> HashMap<K, V>" => "Create a shallow copy.",
    "contains_key": "(key: K) -> bool" => "Check if the key exists.",
    "get": "(key: K) -> Option<V>" => "Return the value for `key`, or None.",
    "get_or": "(key: K, default: V) -> V" => "Return the value for `key`, or `default`.",
    "insert": "(key: K, val: V) -> Option<V>" => "Insert a key-value pair.",
    "is_empty": "() -> bool" => "Check if the map is empty.",
    "keys": "() -> Vec<K>" => "Return a sorted Vec of all keys.",
    "len": "() -> i64" => "Return the number of entries.",
    "remove": "(key: K) -> Option<V>" => "Remove and return the value for `key`.",
    "values": "() -> Vec<V>" => "Return a sorted Vec of all values.",
];

// --- HashSet ---

pub const HASHSET_METHODS: &[MethodInfo] = methods![
    "clone": "() -> HashSet<T>" => "Create a shallow copy.",
    "contains": "(val: T) -> bool" => "Check if the set contains `val`.",
    "difference": "(other: HashSet<T>) -> HashSet<T>" => "Return elements in self but not in other.",
    "insert": "(val: T) -> bool" => "Insert a value. Returns true if new.",
    "intersection": "(other: HashSet<T>) -> HashSet<T>" => "Return elements in both sets.",
    "is_empty": "() -> bool" => "Check if the set is empty.",
    "len": "() -> i64" => "Return the number of elements.",
    "remove": "(val: T) -> bool" => "Remove a value. Returns true if it existed.",
    "to_vec": "() -> Vec<T>" => "Return a sorted Vec of all elements.",
    "union": "(other: HashSet<T>) -> HashSet<T>" => "Return elements in either set.",
];

// --- BTreeMap ---

pub const BTREEMAP_METHODS: &[MethodInfo] = methods![
    "clone": "() -> BTreeMap<K, V>" => "Create a shallow copy.",
    "contains_key": "(key: K) -> bool" => "Check if the key exists.",
    "get": "(key: K) -> Option<V>" => "Return the value for `key`, or None.",
    "get_or": "(key: K, default: V) -> V" => "Return the value for `key`, or `default`.",
    "insert": "(key: K, val: V) -> Option<V>" => "Insert a key-value pair.",
    "is_empty": "() -> bool" => "Check if the map is empty.",
    "keys": "() -> Vec<K>" => "Return a Vec of all keys in sorted order.",
    "len": "() -> i64" => "Return the number of entries.",
    "remove": "(key: K) -> Option<V>" => "Remove and return the value for `key`.",
    "values": "() -> Vec<V>" => "Return a Vec of all values in key order.",
];

// --- BTreeSet ---

pub const BTREESET_METHODS: &[MethodInfo] = methods![
    "clone": "() -> BTreeSet<T>" => "Create a shallow copy.",
    "contains": "(val: T) -> bool" => "Check if the set contains `val`.",
    "difference": "(other: BTreeSet<T>) -> BTreeSet<T>" => "Return elements in self but not in other.",
    "insert": "(val: T) -> bool" => "Insert a value. Returns true if new.",
    "intersection": "(other: BTreeSet<T>) -> BTreeSet<T>" => "Return elements in both sets.",
    "is_empty": "() -> bool" => "Check if the set is empty.",
    "len": "() -> i64" => "Return the number of elements.",
    "remove": "(val: T) -> bool" => "Remove a value. Returns true if it existed.",
    "to_vec": "() -> Vec<T>" => "Return a Vec of all elements in sorted order.",
    "union": "(other: BTreeSet<T>) -> BTreeSet<T>" => "Return elements in either set.",
];

// --- BinaryHeap ---

pub const BINARYHEAP_METHODS: &[MethodInfo] = methods![
    "clone": "() -> BinaryHeap<T>" => "Create a shallow copy.",
    "is_empty": "() -> bool" => "Check if the heap is empty.",
    "len": "() -> i64" => "Return the number of elements.",
    "peek": "() -> Option<T>" => "Return the maximum element without removing it.",
    "pop": "() -> Option<T>" => "Remove and return the maximum element.",
    "push": "(val: T)" => "Insert an element into the heap.",
    "to_vec": "() -> Vec<T>" => "Return a sorted Vec of all elements.",
];

// --- VecDeque ---

pub const VECDEQUE_METHODS: &[MethodInfo] = methods![
    "back": "() -> Option<T>" => "Return the last element, or None if empty.",
    "clone": "() -> VecDeque<T>" => "Create a shallow copy.",
    "front": "() -> Option<T>" => "Return the first element, or None if empty.",
    "is_empty": "() -> bool" => "Check if the deque is empty.",
    "len": "() -> i64" => "Return the number of elements.",
    "pop_back": "() -> Option<T>" => "Remove and return the last element.",
    "pop_front": "() -> Option<T>" => "Remove and return the first element.",
    "push_back": "(val: T)" => "Add an element to the back.",
    "push_front": "(val: T)" => "Add an element to the front.",
    "to_vec": "() -> Vec<T>" => "Return a Vec of all elements.",
];

// --- Iterator ---

pub const ITERATOR_METHODS: &[MethodInfo] = methods![
    "all": "(fn: (T) -> bool) -> bool" => "Check if all elements satisfy the predicate.",
    "any": "(fn: (T) -> bool) -> bool" => "Check if any element satisfies the predicate.",
    "chain": "(other: Iterator) -> Iterator" => "Chain two iterators together.",
    "collect": "() -> Vec<T>" => "Collect the iterator into a Vec.",
    "count": "() -> i64" => "Count the number of elements.",
    "enumerate": "() -> Iterator" => "Return an iterator of (index, element) pairs.",
    "filter": "(fn: (T) -> bool) -> Vec<T>" => "Keep elements matching the predicate.",
    "find": "(fn: (T) -> bool) -> Option<T>" => "Return the first matching element.",
    "flat_map": "(fn: (T) -> Iterator) -> Vec<U>" => "Map then flatten.",
    "flatten": "() -> Vec<T>" => "Flatten nested iterables.",
    "fold": "(init: U, fn: (U, T) -> U) -> U" => "Reduce to a single value.",
    "for_each": "(fn: (T) -> ())" => "Call closure on each element.",
    "map": "(fn: (T) -> U) -> Vec<U>" => "Transform each element.",
    "next": "() -> Option<T>" => "Return the next element.",
    "nth": "(n: i64) -> Option<T>" => "Return the nth element.",
    "position": "(fn: (T) -> bool) -> Option<i64>" => "Return the index of the first match.",
    "rev": "() -> Vec<T>" => "Return elements in reverse order.",
    "skip": "(n: i64) -> Iterator" => "Skip the first `n` elements.",
    "sum": "() -> T" => "Sum all elements.",
    "take": "(n: i64) -> Iterator" => "Take the first `n` elements.",
    "zip": "(other: Iterator) -> Iterator" => "Zip two iterators into pairs.",
];

// --- Option / Result ---

pub const OPTION_RESULT_METHODS: &[MethodInfo] = methods![
    "and_then": "(fn: (T) -> Option<U>) -> Option<U>" => "Chain a fallible operation.",
    "clone": "() -> Self" => "Create a copy.",
    "err": "() -> Option<E>" => "Convert Result::Err(T) to Some(T), Ok to None.",
    "expect": "(msg: String) -> T" => "Unwrap with a custom error message.",
    "is_err": "() -> bool" => "Check if the Result is Err.",
    "is_none": "() -> bool" => "Check if the Option is None.",
    "is_ok": "() -> bool" => "Check if the Result is Ok.",
    "is_some": "() -> bool" => "Check if the Option is Some.",
    "map": "(fn: (T) -> U) -> Option<U>" => "Transform the inner value.",
    "map_err": "(fn: (E) -> F) -> Result<T, F>" => "Transform the error value.",
    "ok": "() -> Option<T>" => "Convert Result::Ok(T) to Some(T), Err to None.",
    "or": "(other: Option<T>) -> Option<T>" => "Return self if Some, else `other`.",
    "or_else": "(fn: () -> Option<T>) -> Option<T>" => "Return self if Some, else call closure.",
    "to_string": "() -> String" => "Convert to a string representation.",
    "unwrap": "() -> T" => "Extract the inner value (panics on None/Err).",
    "unwrap_err": "() -> E" => "Extract the error value (panics on Ok).",
    "unwrap_or": "(default: T) -> T" => "Extract the value, or return `default`.",
    "unwrap_or_else": "(fn: () -> T) -> T" => "Extract the value, or call closure.",
];

// --- Numeric (all integer & float widths) ---

pub const NUMERIC_METHODS: &[MethodInfo] = methods![
    "abs": "() -> Self" => "Return the absolute value.",
    "ceil": "() -> f64" => "Return the smallest integer >= self.",
    "clamp": "(lo: f64, hi: f64) -> f64" => "Clamp the value to [lo, hi].",
    "cos": "() -> f64" => "Compute the cosine (radians).",
    "floor": "() -> f64" => "Return the largest integer <= self.",
    "max": "(other: f64) -> f64" => "Return the larger of self and `other`.",
    "min": "(other: f64) -> f64" => "Return the smaller of self and `other`.",
    "pow": "(exp: f64) -> f64" => "Raise self to the power `exp`.",
    "round": "() -> f64" => "Round to the nearest integer.",
    "sin": "() -> f64" => "Compute the sine (radians).",
    "sqrt": "() -> f64" => "Return the square root.",
    "tan": "() -> f64" => "Compute the tangent (radians).",
    "to_string": "() -> String" => "Convert to a string representation.",
];

// --- Char ---

pub const CHAR_METHODS: &[MethodInfo] = methods![
    "clone": "() -> char" => "Create a copy.",
    "code": "() -> i64" => "Return the Unicode code point.",
    "is_alphabetic": "() -> bool" => "Check if the char is alphabetic.",
    "is_alphanumeric": "() -> bool" => "Check if the char is alphanumeric.",
    "is_ascii": "() -> bool" => "Check if the char is ASCII.",
    "is_digit": "() -> bool" => "Check if the char is an ASCII digit (0-9).",
    "is_lowercase": "() -> bool" => "Check if the char is lowercase.",
    "is_uppercase": "() -> bool" => "Check if the char is uppercase.",
    "is_whitespace": "() -> bool" => "Check if the char is whitespace.",
    "to_lowercase": "() -> char" => "Convert to lowercase.",
    "to_string": "() -> String" => "Convert to a string.",
    "to_uppercase": "() -> char" => "Convert to uppercase.",
];

// --- Struct / EnumVariant / Tuple (generic methods) ---

pub const GENERIC_METHODS: &[MethodInfo] = methods![
    "clone": "() -> Self" => "Create a shallow copy.",
    "to_string": "() -> String" => "Convert to a string representation.",
    "to_json": "() -> String" => "Serialize to a JSON string.",
    "to_json_pretty": "() -> String" => "Serialize to indented JSON.",
];

// ---------------------------------------------------------------------------
// Master type list — used by the LSP to enumerate all built-in types and their
// methods. Order matters for completions: common types first.
// ---------------------------------------------------------------------------

pub const ALL_TYPES: &[TypeInfo] = &[
    TypeInfo {
        name: "Vec",
        detail: "Growable array type",
        hover_text:
            "**Vec\\<T\\>** — Growable array\n\n```oxy\nlet v: Vec<i64> = vec![1, 2, 3];\n```",
        methods: VEC_METHODS,
    },
    TypeInfo {
        name: "String",
        detail: "Owned UTF-8 string",
        hover_text: "**String** — Owned, heap-allocated UTF-8 string",
        methods: STRING_METHODS,
    },
    TypeInfo {
        name: "HashMap",
        detail: "Hash map collection",
        hover_text: "**HashMap\\<K, V\\>** — Hash map collection",
        methods: HASHMAP_METHODS,
    },
    TypeInfo {
        name: "HashSet",
        detail: "Hash set collection",
        hover_text: "**HashSet\\<T\\>** — Hash set collection",
        methods: HASHSET_METHODS,
    },
    TypeInfo {
        name: "BTreeMap",
        detail: "Ordered map collection",
        hover_text: "**BTreeMap\\<K, V\\>** — Sorted key-value map",
        methods: BTREEMAP_METHODS,
    },
    TypeInfo {
        name: "BTreeSet",
        detail: "Ordered set collection",
        hover_text: "**BTreeSet\\<T\\>** — Sorted unique set",
        methods: BTREESET_METHODS,
    },
    TypeInfo {
        name: "BinaryHeap",
        detail: "Priority queue (max-heap)",
        hover_text: "**BinaryHeap\\<T\\>** — Max-heap priority queue",
        methods: BINARYHEAP_METHODS,
    },
    TypeInfo {
        name: "VecDeque",
        detail: "Double-ended queue",
        hover_text: "**VecDeque\\<T\\>** — Growable ring buffer",
        methods: VECDEQUE_METHODS,
    },
    TypeInfo {
        name: "Option",
        detail: "Optional value: Some(T) or None",
        hover_text: "**Option\\<T\\>** — `Some(value)` or `None`",
        methods: OPTION_RESULT_METHODS,
    },
    TypeInfo {
        name: "Result",
        detail: "Result type: Ok(T) or Err(E)",
        hover_text: "**Result\\<T, E\\>** — `Ok(value)` or `Err(error)`",
        methods: OPTION_RESULT_METHODS,
    },
    TypeInfo {
        name: "Iterator",
        detail: "Lazy iterator over elements",
        hover_text: "**Iterator\\<T\\>** — Iterator over elements",
        methods: ITERATOR_METHODS,
    },
    TypeInfo {
        name: "numeric",
        detail: "Integer and float methods",
        hover_text: "**i64 / f64** — Numeric methods common to all integer and float types",
        methods: NUMERIC_METHODS,
    },
    TypeInfo {
        name: "char",
        detail: "Unicode character methods",
        hover_text: "**char** — Unicode scalar value",
        methods: CHAR_METHODS,
    },
];

/// Generic methods available on Struct, EnumVariant, and Tuple values.
pub const GENERIC_TYPE_METHODS: &[MethodInfo] = GENERIC_METHODS;

// ---------------------------------------------------------------------------
// Per-type method name constants — used by builtins dispatch match arms.
// Replaces raw string literals so that adding/removing a method triggers a
// compile error if the other side (symbols or builtins) is not updated.
// ---------------------------------------------------------------------------

pub mod string_m {
    pub const LEN: &str = "len";
    pub const IS_EMPTY: &str = "is_empty";
    pub const TO_UPPERCASE: &str = "to_uppercase";
    pub const TO_LOWERCASE: &str = "to_lowercase";
    pub const TRIM: &str = "trim";
    pub const CONTAINS: &str = "contains";
    pub const STARTS_WITH: &str = "starts_with";
    pub const ENDS_WITH: &str = "ends_with";
    pub const REPLACE: &str = "replace";
    pub const SPLIT: &str = "split";
    pub const CHARS: &str = "chars";
    pub const REPEAT: &str = "repeat";
    pub const PUSH_STR: &str = "push_str";
    pub const CHAR_AT: &str = "char_at";
    pub const SUBSTRING: &str = "substring";
    pub const PARSE_INT: &str = "parse_int";
    pub const PARSE_FLOAT: &str = "parse_float";
    pub const CLONE: &str = "clone";
    pub const TO_STRING: &str = "to_string";
}

pub mod vec_m {
    pub const LEN: &str = "len";
    pub const IS_EMPTY: &str = "is_empty";
    pub const CONTAINS: &str = "contains";
    pub const PUSH: &str = "push";
    pub const POP: &str = "pop";
    pub const FIRST: &str = "first";
    pub const LAST: &str = "last";
    pub const GET: &str = "get";
    pub const INSERT: &str = "insert";
    pub const REMOVE: &str = "remove";
    pub const CLEAR: &str = "clear";
    pub const REVERSE: &str = "reverse";
    pub const JOIN: &str = "join";
    pub const ITER: &str = "iter";
    pub const CLONE: &str = "clone";
    pub const SORT: &str = "sort";
    pub const DEDUP: &str = "dedup";
    pub const EXTEND: &str = "extend";
    pub const REV: &str = "rev";
    pub const CHUNKS: &str = "chunks";
    pub const WINDOWS: &str = "windows";
    pub const MIN: &str = "min";
    pub const MAX: &str = "max";
    pub const SORT_BY: &str = "sort_by";
    pub const SORT_BY_KEY: &str = "sort_by_key";
}

pub mod hashmap_m {
    pub const LEN: &str = "len";
    pub const IS_EMPTY: &str = "is_empty";
    pub const GET: &str = "get";
    pub const GET_OR: &str = "get_or";
    pub const CONTAINS_KEY: &str = "contains_key";
    pub const INSERT: &str = "insert";
    pub const REMOVE: &str = "remove";
    pub const KEYS: &str = "keys";
    pub const VALUES: &str = "values";
    pub const CLONE: &str = "clone";
}

pub mod hashset_m {
    pub const LEN: &str = "len";
    pub const IS_EMPTY: &str = "is_empty";
    pub const CONTAINS: &str = "contains";
    pub const INSERT: &str = "insert";
    pub const REMOVE: &str = "remove";
    pub const TO_VEC: &str = "to_vec";
    pub const UNION: &str = "union";
    pub const INTERSECTION: &str = "intersection";
    pub const DIFFERENCE: &str = "difference";
    pub const CLONE: &str = "clone";
}

pub mod btreemap_m {
    pub const LEN: &str = "len";
    pub const IS_EMPTY: &str = "is_empty";
    pub const GET: &str = "get";
    pub const GET_OR: &str = "get_or";
    pub const CONTAINS_KEY: &str = "contains_key";
    pub const INSERT: &str = "insert";
    pub const REMOVE: &str = "remove";
    pub const KEYS: &str = "keys";
    pub const VALUES: &str = "values";
    pub const CLONE: &str = "clone";
}

pub mod btreeset_m {
    pub const LEN: &str = "len";
    pub const IS_EMPTY: &str = "is_empty";
    pub const CONTAINS: &str = "contains";
    pub const INSERT: &str = "insert";
    pub const REMOVE: &str = "remove";
    pub const TO_VEC: &str = "to_vec";
    pub const UNION: &str = "union";
    pub const INTERSECTION: &str = "intersection";
    pub const DIFFERENCE: &str = "difference";
    pub const CLONE: &str = "clone";
}

pub mod binaryheap_m {
    pub const LEN: &str = "len";
    pub const IS_EMPTY: &str = "is_empty";
    pub const PEEK: &str = "peek";
    pub const PUSH: &str = "push";
    pub const POP: &str = "pop";
    pub const TO_VEC: &str = "to_vec";
    pub const CLONE: &str = "clone";
}

pub mod vecdeque_m {
    pub const LEN: &str = "len";
    pub const IS_EMPTY: &str = "is_empty";
    pub const FRONT: &str = "front";
    pub const BACK: &str = "back";
    pub const PUSH_FRONT: &str = "push_front";
    pub const PUSH_BACK: &str = "push_back";
    pub const POP_FRONT: &str = "pop_front";
    pub const POP_BACK: &str = "pop_back";
    pub const TO_VEC: &str = "to_vec";
    pub const CLONE: &str = "clone";
}

pub mod iterator_m {
    pub const MAP: &str = "map";
    pub const FILTER: &str = "filter";
    pub const TAKE: &str = "take";
    pub const SKIP: &str = "skip";
    pub const CHAIN: &str = "chain";
    pub const ZIP: &str = "zip";
    pub const ENUMERATE: &str = "enumerate";
    pub const REV: &str = "rev";
    pub const FLAT_MAP: &str = "flat_map";
    pub const FLATTEN: &str = "flatten";
    pub const NEXT: &str = "next";
    pub const COLLECT: &str = "collect";
    pub const SUM: &str = "sum";
    pub const COUNT: &str = "count";
    pub const NTH: &str = "nth";
    pub const ANY: &str = "any";
    pub const ALL: &str = "all";
    pub const FIND: &str = "find";
    pub const POSITION: &str = "position";
    pub const FOLD: &str = "fold";
    pub const FOR_EACH: &str = "for_each";
}

pub mod option_result_m {
    pub const IS_SOME: &str = "is_some";
    pub const IS_NONE: &str = "is_none";
    pub const IS_OK: &str = "is_ok";
    pub const IS_ERR: &str = "is_err";
    pub const UNWRAP: &str = "unwrap";
    pub const EXPECT: &str = "expect";
    pub const UNWRAP_OR: &str = "unwrap_or";
    pub const UNWRAP_OR_ELSE: &str = "unwrap_or_else";
    pub const OR: &str = "or";
    pub const OR_ELSE: &str = "or_else";
    pub const MAP: &str = "map";
    pub const MAP_ERR: &str = "map_err";
    pub const AND_THEN: &str = "and_then";
    pub const UNWRAP_ERR: &str = "unwrap_err";
    pub const OK: &str = "ok";
    pub const ERR: &str = "err";
    pub const CLONE: &str = "clone";
    pub const TO_STRING: &str = "to_string";
}

pub mod numeric_m {
    pub const ABS: &str = "abs";
    pub const SQRT: &str = "sqrt";
    pub const FLOOR: &str = "floor";
    pub const CEIL: &str = "ceil";
    pub const ROUND: &str = "round";
    pub const POW: &str = "pow";
    pub const SIN: &str = "sin";
    pub const COS: &str = "cos";
    pub const TAN: &str = "tan";
    pub const MIN: &str = "min";
    pub const MAX: &str = "max";
    pub const CLAMP: &str = "clamp";
    pub const TO_STRING: &str = "to_string";
}

pub mod char_m {
    pub const IS_DIGIT: &str = "is_digit";
    pub const IS_ALPHABETIC: &str = "is_alphabetic";
    pub const IS_ALPHANUMERIC: &str = "is_alphanumeric";
    pub const IS_WHITESPACE: &str = "is_whitespace";
    pub const IS_LOWERCASE: &str = "is_lowercase";
    pub const IS_UPPERCASE: &str = "is_uppercase";
    pub const IS_ASCII: &str = "is_ascii";
    pub const TO_LOWERCASE: &str = "to_lowercase";
    pub const TO_UPPERCASE: &str = "to_uppercase";
    pub const CLONE: &str = "clone";
    pub const CODE: &str = "code";
    pub const TO_STRING: &str = "to_string";
}

pub mod generic_m {
    pub const CLONE: &str = "clone";
    pub const TO_STRING: &str = "to_string";
}

// ---------------------------------------------------------------------------
// Standard library module paths
// ---------------------------------------------------------------------------

pub const ALL_MODULES: &[ModuleInfo] = &[
    ModuleInfo {
        path: "json::",
        detail: "JSON parsing and serialization",
    },
    ModuleInfo {
        path: "http::",
        detail: "HTTP client (GET, POST, etc.)",
    },
    ModuleInfo {
        path: "math::",
        detail: "Mathematical functions and constants",
    },
    ModuleInfo {
        path: "std::fs::",
        detail: "Filesystem operations",
    },
    ModuleInfo {
        path: "std::env::",
        detail: "Environment variables and paths",
    },
    ModuleInfo {
        path: "std::process::",
        detail: "Process execution",
    },
    ModuleInfo {
        path: "std::regex::",
        detail: "Regular expressions",
    },
    ModuleInfo {
        path: "std::net::",
        detail: "TCP and UDP networking",
    },
    ModuleInfo {
        path: "std::time::",
        detail: "Time and timing utilities",
    },
    ModuleInfo {
        path: "std::rand::",
        detail: "Random number generation",
    },
];
