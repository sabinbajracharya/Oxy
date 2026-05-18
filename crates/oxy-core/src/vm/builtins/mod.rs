//! Built-in method dispatch for Oxy types.
//!
//! Each type has its own module with a single entry point.
//! Both the tree-walking interpreter and the bytecode VM route
//! method calls through these functions.
//!
//! Signature convention:
//! ```ignore
//! pub fn vec_methods(receiver: Value, method: &str, args: &[Value]) -> Result<Value, String>
//! ```

pub mod hashmap;
pub mod hashset;
pub mod string;
pub mod vec;
