//! Oxy — Rust syntax, scripting freedom.
//!
//! Core library for the Oxy programming language interpreter.
//! Oxy replicates Rust's syntax without the borrow checker or ownership rules.

// Value contains Rc<RefCell<...>> for shared mutable state (no borrow checker).
// We use Value as HashMap keys intentionally — keys are never mutated while in a map.
#![allow(clippy::mutable_key_type)]
#![allow(clippy::type_complexity)]
#![allow(clippy::useless_format)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::for_kv_map)]
#![allow(clippy::borrowed_box)]
#![allow(clippy::single_match)]
#![allow(clippy::wildcard_in_or_patterns)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::map_clone)]
#![allow(clippy::useless_asref)]
#![allow(clippy::cloned_ref_to_slice_refs)]
#![allow(clippy::needless_late_init)]
#![allow(clippy::assigning_clones)]

/// Abstract syntax tree node definitions.
pub mod ast;
/// Environment and lexical scope management.
pub mod env;
/// Error types used throughout the interpreter.
pub mod errors;
/// HTTP client support for Oxy scripts.
#[cfg(feature = "http")]
pub mod http;
// Interpreter module deleted — 100% bytecode VM.
/// JSON serialization and deserialization helpers.
pub mod json;
/// Lexer (tokenizer) for Oxy source code.
pub mod lexer;
/// Parser that transforms tokens into an AST.
pub mod parser;
/// Built-in standard library modules.
pub mod stdlib;
/// Canonical symbol definitions: keywords, types, methods, modules.
pub mod symbols;
/// Semantic type checker that validates type annotations before execution.
pub mod type_checker;
/// Runtime value types and type metadata.
pub mod types;
/// Stack-based virtual machine for executing compiled bytecode.
pub mod vm;

/// The current version of the Oxy language.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns a formatted version string.
pub fn version_string() -> String {
    format!("Oxy v{VERSION}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_string() {
        let version = version_string();
        assert!(version.starts_with("Oxy v"));
        assert!(version.contains(VERSION));
    }
}
