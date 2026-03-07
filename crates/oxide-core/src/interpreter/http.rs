//! HTTP client functions and response/builder method dispatch.
//!
//! Provides the `http::` pseudo-module (get, post, put, delete, get_json,
//! post_json, put_json, patch_json, request) and methods on HttpResponse
//! and HttpRequestBuilder structs.

use std::collections::HashMap;

use crate::errors::check_arg_count;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

use super::Interpreter;

impl Interpreter {
    /// Dispatch an `http::function()` call.
    pub(crate) fn call_http_function(
        &self,
        func_name: &str,
        args: &[Value],
        span: &Span,
    ) -> Result<Value, FerriError> {
        match func_name {
            "get" => {
                check_arg_count("http::get", 1, args, span)?;
                let url = format!("{}", args[0]);
                self.http_do_request("GET", &url, &[], None)
            }
            "post" => {
                check_arg_count("http::post", 2, args, span)?;
                let url = format!("{}", args[0]);
                let body = format!("{}", args[1]);
                self.http_do_request("POST", &url, &[], Some(&body))
            }
            "put" => {
                check_arg_count("http::put", 2, args, span)?;
                let url = format!("{}", args[0]);
                let body = format!("{}", args[1]);
                self.http_do_request("PUT", &url, &[], Some(&body))
            }
            "delete" => {
                check_arg_count("http::delete", 1, args, span)?;
                let url = format!("{}", args[0]);
                self.http_do_request("DELETE", &url, &[], None)
            }
            "get_json" => {
                check_arg_count("http::get_json", 1, args, span)?;
                let url = format!("{}", args[0]);
                match crate::http::request("GET", &url, &[], None) {
                    Ok((_, body, _)) => match crate::json::deserialize(&body) {
                        Ok(val) => Ok(Value::ok(val)),
                        Err(e) => Ok(Value::err(Value::String(format!("JSON parse error: {e}")))),
                    },
                    Err(e) => Ok(Value::err(Value::String(e))),
                }
            }
            "post_json" => self.http_json_body("POST", args, span),
            "put_json" => self.http_json_body("PUT", args, span),
            "patch_json" => self.http_json_body("PATCH", args, span),
            "request" => {
                check_arg_count("http::request", 2, args, span)?;
                let method = format!("{}", args[0]);
                let url = format!("{}", args[1]);
                let mut fields = HashMap::new();
                fields.insert("method".to_string(), Value::String(method));
                fields.insert("url".to_string(), Value::String(url));
                fields.insert("headers".to_string(), Value::HashMap(HashMap::new()));
                fields.insert("body".to_string(), Value::String(String::new()));
                Ok(Value::Struct {
                    name: "HttpRequestBuilder".to_string(),
                    fields,
                })
            }
            _ => Err(FerriError::Runtime {
                message: format!("unknown http function: {func_name}"),
                line: span.line,
                column: span.column,
            }),
        }
    }

    /// Execute an HTTP request and wrap the response in a Result.
    fn http_do_request(
        &self,
        method: &str,
        url: &str,
        headers: &[(String, String)],
        body: Option<&str>,
    ) -> Result<Value, FerriError> {
        match crate::http::request(method, url, headers, body) {
            Ok((status, body, resp_headers)) => {
                let headers_map: HashMap<String, Value> = resp_headers
                    .into_iter()
                    .map(|(k, v)| (k, Value::String(v)))
                    .collect();
                let mut fields = HashMap::new();
                fields.insert("status".to_string(), Value::Integer(status));
                fields.insert("body".to_string(), Value::String(body));
                fields.insert("headers".to_string(), Value::HashMap(headers_map));
                Ok(Value::ok(Value::Struct {
                    name: "HttpResponse".to_string(),
                    fields,
                }))
            }
            Err(e) => Ok(Value::err(Value::String(e))),
        }
    }

    /// Send a JSON body via HTTP (used by post_json, put_json, patch_json).
    fn http_json_body(
        &self,
        method: &str,
        args: &[Value],
        span: &Span,
    ) -> Result<Value, FerriError> {
        check_arg_count(
            &format!("http::{}_json", method.to_lowercase()),
            2,
            args,
            span,
        )?;
        let url = format!("{}", args[0]);
        let json_body = match crate::json::serialize(&args[1]) {
            Ok(j) => j,
            Err(e) => {
                return Ok(Value::err(Value::String(format!(
                    "JSON serialize error: {e}"
                ))))
            }
        };
        let headers = vec![("Content-Type".to_string(), "application/json".to_string())];
        self.http_do_request(method, &url, &headers, Some(&json_body))
    }

    /// Handle method calls on HttpResponse struct.
    pub(crate) fn call_http_response_method(
        &self,
        fields: &HashMap<String, Value>,
        method: &str,
        span: &Span,
    ) -> Result<Value, FerriError> {
        match method {
            "json" => {
                let body = match fields.get("body") {
                    Some(Value::String(s)) => s.clone(),
                    _ => String::new(),
                };
                match crate::json::deserialize(&body) {
                    Ok(val) => Ok(Value::ok(val)),
                    Err(e) => Ok(Value::err(Value::String(e))),
                }
            }
            "text" => {
                let body = match fields.get("body") {
                    Some(Value::String(s)) => s.clone(),
                    _ => String::new(),
                };
                Ok(Value::String(body))
            }
            "status_ok" => {
                let status = match fields.get("status") {
                    Some(Value::Integer(n)) => *n,
                    _ => 0,
                };
                Ok(Value::Bool((200..300).contains(&status)))
            }
            _ => Err(FerriError::Runtime {
                message: format!("no method `{method}` on HttpResponse"),
                line: span.line,
                column: span.column,
            }),
        }
    }

    /// Handle method calls on HttpRequestBuilder struct.
    pub(crate) fn call_http_builder_method(
        &self,
        mut fields: HashMap<String, Value>,
        method: &str,
        args: &[Value],
        span: &Span,
    ) -> Result<Value, FerriError> {
        match method {
            "header" => {
                check_arg_count("HttpRequestBuilder::header", 2, args, span)?;
                let key = format!("{}", args[0]);
                let val = format!("{}", args[1]);
                if let Some(Value::HashMap(ref mut h)) = fields.get_mut("headers") {
                    h.insert(key, Value::String(val));
                }
                Ok(Value::Struct {
                    name: "HttpRequestBuilder".to_string(),
                    fields,
                })
            }
            "body" => {
                check_arg_count("HttpRequestBuilder::body", 1, args, span)?;
                let body_str = format!("{}", args[0]);
                fields.insert("body".to_string(), Value::String(body_str));
                Ok(Value::Struct {
                    name: "HttpRequestBuilder".to_string(),
                    fields,
                })
            }
            "json" => {
                check_arg_count("HttpRequestBuilder::json", 1, args, span)?;
                let json_body = match crate::json::serialize(&args[0]) {
                    Ok(j) => j,
                    Err(e) => {
                        return Err(FerriError::Runtime {
                            message: format!("JSON serialize error: {e}"),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                fields.insert("body".to_string(), Value::String(json_body));
                if let Some(Value::HashMap(ref mut h)) = fields.get_mut("headers") {
                    h.insert(
                        "Content-Type".to_string(),
                        Value::String("application/json".to_string()),
                    );
                }
                Ok(Value::Struct {
                    name: "HttpRequestBuilder".to_string(),
                    fields,
                })
            }
            "send" => {
                let method_str = match fields.get("method") {
                    Some(Value::String(s)) => s.clone(),
                    _ => "GET".to_string(),
                };
                let url = match fields.get("url") {
                    Some(Value::String(s)) => s.clone(),
                    _ => String::new(),
                };
                let body = match fields.get("body") {
                    Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
                    _ => None,
                };
                let mut header_pairs = Vec::new();
                if let Some(Value::HashMap(h)) = fields.get("headers") {
                    for (k, v) in h {
                        header_pairs.push((k.clone(), format!("{v}")));
                    }
                }
                self.http_do_request(&method_str, &url, &header_pairs, body.as_deref())
            }
            _ => Err(FerriError::Runtime {
                message: format!("no method `{method}` on HttpRequestBuilder"),
                line: span.line,
                column: span.column,
            }),
        }
    }
}
