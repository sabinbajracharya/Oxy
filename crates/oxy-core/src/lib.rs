//! Oxy — Rust syntax, scripting freedom.
//!
//! Core library for the Oxy programming language interpreter.
//! Oxy replicates Rust's syntax without the borrow checker or ownership rules.

// Value contains Rc<RefCell<...>> for shared mutable state (no borrow checker).
// We use Value as HashMap keys intentionally — keys are never mutated while in a map.
#![allow(clippy::mutable_key_type)]
// The Value enum and type system are inherently type-complex.
#![allow(clippy::type_complexity)]
// `std::slice::from_ref(&x)` is more verbose than `&[x.clone()]` for single-
// element slices passed to call_fn, and the clone cost is negligible here.
#![allow(clippy::cloned_ref_to_slice_refs)]
// Collapsed match/if-let patterns are sometimes less readable than the
// two-level form, especially in the ir_gen match dispatcher.
#![allow(clippy::collapsible_match)]

/// Abstract syntax tree node definitions.
pub mod ast;
/// Environment and lexical scope management.
pub mod env;
/// Structured diagnostics model shared by CLI/LSP and pipeline stages.
pub mod diagnostics;
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
/// General-purpose utility functions (edit distance, name suggestion).
pub mod util;
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
