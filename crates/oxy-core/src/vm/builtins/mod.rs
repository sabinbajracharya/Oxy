//! Built-in method dispatch for Oxy types.
//!
//! Each type has its own module with a single entry point.
//! Both the tree-walking interpreter and the bytecode VM route
//! method calls through these functions.

pub mod binary_heap;
pub mod btreemap;
pub mod btreeset;
pub mod hashmap;
pub mod hashset;
pub mod iterator;
pub mod numeric;
pub mod option;
pub mod result;
pub mod string;
pub mod vec;
pub mod vec_deque;
