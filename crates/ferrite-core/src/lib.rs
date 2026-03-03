//! Ferrite — Rust syntax, scripting freedom.
//!
//! Core library for the Ferrite programming language interpreter.
//! Ferrite replicates Rust's syntax without the borrow checker or ownership rules.

/// Abstract syntax tree node definitions.
pub mod ast;
/// Environment and lexical scope management.
pub mod env;
/// Error types used throughout the interpreter.
pub mod errors;
/// HTTP client support for Ferrite scripts.
pub mod http;
/// Tree-walking interpreter that evaluates the AST.
pub mod interpreter;
/// JSON serialization and deserialization helpers.
pub mod json;
/// Lexer (tokenizer) for Ferrite source code.
pub mod lexer;
/// Parser that transforms tokens into an AST.
pub mod parser;
/// Built-in standard library modules.
pub mod stdlib;
/// Runtime value types and type metadata.
pub mod types;

/// The current version of the Ferrite language.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns a formatted version string.
pub fn version_string() -> String {
    format!("Ferrite v{VERSION}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_string() {
        let version = version_string();
        assert!(version.starts_with("Ferrite v"));
        assert!(version.contains(VERSION));
    }
}
