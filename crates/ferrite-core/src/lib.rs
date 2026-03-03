//! Ferrite — Rust syntax, scripting freedom.
//!
//! Core library for the Ferrite programming language interpreter.
//! Ferrite replicates Rust's syntax without the borrow checker or ownership rules.

pub mod errors;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod types;
pub mod env;
pub mod interpreter;

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
