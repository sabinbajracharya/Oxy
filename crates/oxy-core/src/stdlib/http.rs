//! `http::*` dispatcher: routes `http::get`, `http::post`, `http::put_json`,
//! `http::delete`, etc. to `crate::http::request` and wraps the result as an
//! `Ok(HttpResponse)` / `Err(String)` enum value.
//!
//! Registered in `stdlib::registry::MODULES`. When the `http` cargo feature
//! is disabled this dispatcher returns a clear error from every call.

use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

#[cfg(feature = "http")]
pub fn call(func_name: &str, args: &[Value], _span: &Span) -> Result<Value, FerriError> {
    let body_arg = || args.get(1).map(|v| v.to_string());
    let lift = |r: Result<Value, String>| {
        r.map_err(|e| FerriError::Runtime {
            message: e,
            line: 0,
            column: 0,
        })
    };
    match func_name {
        "get" | "get_json" => lift(http_call("GET", args, None)),
        "post" | "post_json" => lift(http_call("POST", args, body_arg())),
        "put_json" => lift(http_call("PUT", args, body_arg())),
        "delete" => lift(http_call("DELETE", args, None)),
        other => Err(FerriError::Runtime {
            message: format!("unknown http function `http::{other}`"),
            line: 0,
            column: 0,
        }),
    }
}

#[cfg(not(feature = "http"))]
pub fn call(_func_name: &str, _args: &[Value], _span: &Span) -> Result<Value, FerriError> {
    Err(FerriError::Runtime {
        message: "`http::` is not available in this build (the `http` feature is disabled)".into(),
        line: 0,
        column: 0,
    })
}

/// Issue an HTTP request and wrap the response in an `HttpResponse` struct.
#[cfg(feature = "http")]
fn http_call(method: &str, args: &[Value], body: Option<String>) -> Result<Value, String> {
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
