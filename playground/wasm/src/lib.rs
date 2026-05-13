use wasm_bindgen::prelude::*;

/// Run Oxide source code and return captured output.
/// Errors are returned as part of the output string.
#[wasm_bindgen]
pub fn run_oxide(source: &str) -> String {
    match run_inner(source) {
        Ok(output) => output,
        Err(e) => format!("Error: {e}"),
    }
}

fn run_inner(source: &str) -> Result<String, String> {
    let program = oxide_core::parser::parse(source).map_err(|e| e.to_string())?;
    oxide_core::type_checker::TypeChecker::new()
        .check_program(&program)
        .map_err(|e| e.to_string())?;
    let mut interp = oxide_core::interpreter::Interpreter::new_with_captured_output();
    interp
        .execute_program(&program)
        .map_err(|e| e.to_string())?;
    Ok(interp.captured_output().join(""))
}
