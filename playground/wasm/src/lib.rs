use wasm_bindgen::prelude::*;

/// Run Oxy source code and return captured output.
/// Errors are returned as part of the output string.
#[wasm_bindgen]
pub fn run_oxy(source: &str) -> String {
    match run_inner(source) {
        Ok(output) => output,
        Err(e) => format!("Error: {e}"),
    }
}

fn run_inner(source: &str) -> Result<String, String> {
    match oxy_core::vm::run_compiled_capturing(source) {
        Ok((_, output)) => Ok(output.join("")),
        Err(e) => Err(e.to_string()),
    }
}
