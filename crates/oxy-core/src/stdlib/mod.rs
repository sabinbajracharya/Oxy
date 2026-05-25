//! Built-in standard library modules for the Oxy language.
//!
//! Provides math, random number generation, time utilities, file system
//! operations, environment access, process control, regex, networking,
//! HTTP server, and SQLite database.

/// Command-line argument parsing (flags + positionals) on top of `env::args`.
pub mod args;
/// SQLite database operations (open, query, execute).
#[cfg(feature = "db")]
pub mod db;
/// Environment variable and process argument access.
pub mod env;
/// File system operations (read, write, directory manipulation).
pub mod fs;
/// Standard input reading (read_line, read_to_string from stdin).
pub mod io;
/// Mathematical functions and constants (e.g. `sqrt`, `sin`, `PI`).
pub mod math;
/// TCP/UDP networking and DNS lookup.
pub mod net;
/// Lexical path manipulation (join, dirname, basename, extension, normalize).
pub mod path;
/// Process control and command execution.
pub mod process;
/// Pseudo-random number generation.
pub mod rand;
/// Regular expression matching, searching, and replacement.
pub mod regex;
/// HTTP server with routing, path params, query strings, static files.
#[cfg(feature = "server")]
pub mod server;
/// Time and duration utilities.
pub mod time;

/// JSON parse/serialize dispatcher (wraps `crate::json`).
pub mod json;

/// HTTP client dispatcher (wraps `crate::http`, feature-gated on `http`).
pub mod http;

/// Single-source-of-truth registry mapping built-in paths to their handlers.
/// Both the compiler whitelist and the VM dispatch read from this.
pub mod registry;
