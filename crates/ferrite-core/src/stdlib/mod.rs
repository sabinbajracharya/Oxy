//! Built-in standard library modules for the Ferrite language.
//!
//! Provides math, random number generation, time utilities, file system
//! operations, environment access, process control, regex, networking,
//! and HTTP server.

/// Environment variable and process argument access.
pub mod env;
/// File system operations (read, write, directory manipulation).
pub mod fs;
/// Mathematical functions and constants (e.g. `sqrt`, `sin`, `PI`).
pub mod math;
/// TCP/UDP networking and DNS lookup.
pub mod net;
/// Process control and command execution.
pub mod process;
/// Pseudo-random number generation.
pub mod rand;
/// Regular expression matching, searching, and replacement.
pub mod regex;
/// HTTP server with routing, path params, query strings, static files.
pub mod server;
/// Time and duration utilities.
pub mod time;
