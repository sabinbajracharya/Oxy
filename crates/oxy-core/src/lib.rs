//! Oxy — Rust syntax, scripting freedom.
//!
//! Core library for the Oxy programming language interpreter.
//! Oxy replicates Rust's syntax without the borrow checker or ownership rules.

/// Abstract syntax tree node definitions.
pub mod ast;
/// Bytecode compiler: AST → stack-based VM opcodes.
pub mod compiler;
/// Environment and lexical scope management.
pub mod env;
/// Error types used throughout the interpreter.
pub mod errors;
/// HTTP client support for Oxy scripts.
#[cfg(feature = "http")]
pub mod http;
/// Tree-walking interpreter that evaluates the AST.
pub mod interpreter;
/// JSON serialization and deserialization helpers.
pub mod json;
/// Lexer (tokenizer) for Oxy source code.
pub mod lexer;
/// Package manager: install, manifest parsing, registry support.
pub mod package;
/// Parser that transforms tokens into an AST.
pub mod parser;
/// Built-in standard library modules.
pub mod stdlib;
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
