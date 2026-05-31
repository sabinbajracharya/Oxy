//! Native test suite — exercises the register-IR + Cranelift JIT backend
//! through `run_compiled_capturing`.
//!
//! The ~400 tests are split by topic into the `vm_tests/` submodules below;
//! the shared imports and `run_and_capture` / `run_and_get_value` helpers live
//! here and reach the submodules via their `use super::*`.

use oxy_core::types::*;
use oxy_core::vm::{run_compiled, run_compiled_capturing};

fn run_and_capture(src: &str) -> Vec<String> {
    let (_, output) = run_compiled_capturing(src).unwrap();
    output
}

fn run_and_get_value(src: &str) -> Value {
    let (val, _) = run_compiled_capturing(src).unwrap();
    val
}

mod basics;
mod closures;
mod collections;
mod control_flow;
mod diagnostics;
mod error_handling;
mod functions;
mod modules;
mod patterns;
mod reference_syntax;
mod stdlib;
mod strings;
mod structs_enums;
mod traits_generics;
