//! `http::*` dispatcher: routes `http::get`, `http::post`, `http::put_json`,
//! `http::delete`, `http::fetch`, `http::fetch_post` etc. to `crate::http::request`
//! and wraps the result as an `Ok(HttpResponse)` / `Err(String)` enum value.
//!
//! `http::fetch` and `http::fetch_post` return an `AsyncResult` future — the
//! HTTP call runs on a background thread and `.await` polls for completion.
//!
//! Registered in `stdlib::registry::MODULES`. When the `http` cargo feature
//! is disabled this dispatcher returns a clear error from every call.

use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

#[cfg(feature = "http")]
fn build_http_response(
    status: i64,
    resp_body: String,
    headers: std::collections::HashMap<String, String>,
) -> Value {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

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
    Value::ok(Value::Struct {
        name: "HttpResponse".to_string(),
        fields,
    })
}

/// Build a Value::Struct HttpResponse from raw HttpResultData.
pub(crate) fn build_response_from_raw(data: crate::types::HttpResultData) -> Value {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    let mut fields = HashMap::new();
    fields.insert("status".to_string(), Value::I64(data.status));
    fields.insert("body".to_string(), Value::String(data.body));
    let mut header_map: HashMap<Value, Value> = HashMap::new();
    for (k, v) in data.headers {
        header_map.insert(Value::String(k), Value::String(v));
    }
    fields.insert(
        "headers".to_string(),
        Value::HashMap(Rc::new(RefCell::new(header_map))),
    );
    Value::ok(Value::Struct {
        name: "HttpResponse".to_string(),
        fields,
    })
}

#[cfg(feature = "http")]
pub fn call(
    func_name: &str,
    args: &[Value],
    _span: &Span,
    _cb: crate::stdlib::registry::ClosureInvoker<'_>,
) -> Result<Value, FerriError> {
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
        "fetch" => Ok(fetch_get(args)),
        "fetch_post" => Ok(fetch_post(args)),
        other => Err(FerriError::Runtime {
            message: format!("unknown http function `http::{other}`"),
            line: 0,
            column: 0,
        }),
    }
}

#[cfg(not(feature = "http"))]
pub fn call(
    _func_name: &str,
    _args: &[Value],
    _span: &Span,
    _cb: crate::stdlib::registry::ClosureInvoker<'_>,
) -> Result<Value, FerriError> {
    Err(FerriError::Runtime {
        message: "`http::` is not available in this build (the `http` feature is disabled)".into(),
        line: 0,
        column: 0,
    })
}

/// Issue a synchronous HTTP request, returning `Ok(HttpResponse {...})` or
/// `Err("error message")`.
#[cfg(feature = "http")]
fn http_call(method: &str, args: &[Value], body: Option<String>) -> Result<Value, String> {
    let url = args.first().map(|v| v.to_string()).unwrap_or_default();
    let result = crate::http::request(method, &url, &[], body.as_deref());
    match result {
        Ok((status, resp_body, headers)) => Ok(build_http_response(status, resp_body, headers)),
        Err(e) => Ok(Value::err(Value::String(e))),
    }
}

/// Run an HTTP GET request on a background thread and return an AsyncResult
/// future that resolves to `HttpResponse`.
#[cfg(feature = "http")]
fn fetch_get(args: &[Value]) -> Value {
    let url = args.first().map(|v| v.to_string()).unwrap_or_default();
    let slot: std::sync::Arc<
        std::sync::Mutex<Option<Result<crate::types::HttpResultData, String>>>,
    > = std::sync::Arc::new(std::sync::Mutex::new(None));
    let slot_clone = std::sync::Arc::clone(&slot);

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(move || {
            let result = crate::http::request("GET", &url, &[], None);
            let data = match result {
                Ok((status, body, headers)) => Ok(crate::types::HttpResultData {
                    status,
                    body,
                    headers: headers.into_iter().collect(),
                }),
                Err(e) => Err(e),
            };
            *slot_clone.lock().unwrap() = Some(data);
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        let result = crate::http::request("GET", &url, &[], None);
        let data = match result {
            Ok((status, body, headers)) => Ok(crate::types::HttpResultData {
                status,
                body,
                headers: headers.into_iter().collect(),
            }),
            Err(e) => Err(e),
        };
        *slot.lock().unwrap() = Some(data);
    }

    Value::AsyncResult { result: slot }
}

/// Run an HTTP POST request on a background thread and return an AsyncResult
/// future that resolves to `HttpResponse`.
#[cfg(feature = "http")]
fn fetch_post(args: &[Value]) -> Value {
    let url = args.first().map(|v| v.to_string()).unwrap_or_default();
    let body = args.get(1).map(|v| v.to_string());
    let slot: std::sync::Arc<
        std::sync::Mutex<Option<Result<crate::types::HttpResultData, String>>>,
    > = std::sync::Arc::new(std::sync::Mutex::new(None));
    let slot_clone = std::sync::Arc::clone(&slot);

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(move || {
            let result = crate::http::request("POST", &url, &[], body.as_deref());
            let data = match result {
                Ok((status, resp_body, headers)) => Ok(crate::types::HttpResultData {
                    status,
                    body: resp_body,
                    headers: headers.into_iter().collect(),
                }),
                Err(e) => Err(e),
            };
            *slot_clone.lock().unwrap() = Some(data);
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        let result = crate::http::request("POST", &url, &[], body.as_deref());
        let data = match result {
            Ok((status, resp_body, headers)) => Ok(crate::types::HttpResultData {
                status,
                body: resp_body,
                headers: headers.into_iter().collect(),
            }),
            Err(e) => Err(e),
        };
        *slot.lock().unwrap() = Some(data);
    }

    Value::AsyncResult { result: slot }
}
