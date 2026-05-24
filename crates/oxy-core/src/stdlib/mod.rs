//! Built-in standard library modules for the Oxy language.
//!
//! Provides math, random number generation, time utilities, file system
//! operations, environment access, process control, regex, networking,
//! HTTP server, and SQLite database.

/// SQLite database operations (open, query, execute).
#[cfg(feature = "db")]
pub mod db;
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
#[cfg(feature = "server")]
pub mod server;
/// Time and duration utilities.
pub mod time;

/// Single-source-of-truth registry mapping built-in paths to their handlers.
/// Both the compiler whitelist and the VM dispatch read from this.
pub mod registry;

/// HTTP helper: call `crate::http::request` and wrap the result as an
/// `Ok(HttpResponse)` / `Err(String)` enum value. Used by the `http`
/// module dispatcher in `registry`.
#[cfg(feature = "http")]
pub(crate) fn http_call(
    method: &str,
    args: &[crate::types::Value],
    body: Option<String>,
) -> Result<crate::types::Value, String> {
    use crate::types::Value;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    let url = args.first().map(|v| v.to_string()).unwrap_or_default();
    let result = crate::http::request(method, &url, &[], body.as_deref());
    match result {
        Ok((status, resp_body, headers)) => {
            let mut fields = HashMap::new();
            fields.insert("status".to_string(), Value::I64(status));
            fields.insert("body".to_string(), Value::String(resp_body));
            let mut header_map: HashMap<Value, Value> = HashMap::new();
            for (k, v) in &headers {
                header_map.insert(Value::String(k.clone()), Value::String(v.clone()));
            }
            fields.insert(
                "headers".to_string(),
                Value::HashMap(Rc::new(RefCell::new(header_map))),
            );
            Ok(Value::ok(Value::Struct {
                name: "HttpResponse".to_string(),
                fields,
            }))
        }
        Err(e) => Ok(Value::err(Value::String(e))),
    }
}
