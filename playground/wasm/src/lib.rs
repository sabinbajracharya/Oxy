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

/// Run #[test] functions in source and return JSON results.
/// Returns: [{"name":"test_foo","passed":true,"error":null}, ...]
/// On infrastructure error: {"error":"message"}
#[wasm_bindgen]
pub fn run_tests_oxy(source: &str) -> String {
    match oxy_core::vm::run_tests("lesson", source) {
        Ok(results) => {
            let items: Vec<String> = results
                .iter()
                .map(|r| {
                    let err = match &r.error {
                        Some(e) => format!("\"{}\"", e.replace('"', "\\\"").replace('\n', "\\n")),
                        None => "null".to_string(),
                    };
                    format!(
                        "{{\"name\":\"{}\",\"passed\":{},\"error\":{}}}",
                        r.name, r.passed, err
                    )
                })
                .collect();
            format!("[{}]", items.join(","))
        }
        Err(e) => format!("{{\"error\":\"{}\"}}", e.to_string().replace('"', "\\\"").replace('\n', "\\n")),
    }
}
