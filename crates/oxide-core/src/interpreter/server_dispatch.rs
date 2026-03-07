//! HTTP server dispatch for the Oxide interpreter.
//!
//! Handles `Server::new()`, route registration methods (`.get()`, `.post()`,
//! `.put()`, `.delete()`, `.patch()`), `.static_files()`, `.listen()`,
//! and `Response::text()`, `Response::json()`, `Response::html()`,
//! `Response::status()`.

use std::collections::HashMap;

use crate::errors::{check_arg_count, expect_string, FerriError};
use crate::lexer::Span;
use crate::stdlib::server::{self, Method, Route};
use crate::types::Value;

use super::Interpreter;

impl Interpreter {
    /// Handle `Server::new()` and `Response::*()` path calls.
    pub(crate) fn call_server_path(
        &mut self,
        type_name: &str,
        method_name: &str,
        args: &[Value],
        span: &Span,
    ) -> Result<Value, FerriError> {
        match type_name {
            "Server" => match method_name {
                "new" => {
                    check_arg_count("Server::new", 0, args, span)?;
                    self.server_id_counter += 1;
                    let id = format!("__server_{}", self.server_id_counter);
                    self.server_routes.insert(id.clone(), Vec::new());
                    let mut fields = HashMap::new();
                    fields.insert("__id".to_string(), Value::String(id));
                    Ok(Value::Struct {
                        name: "Server".to_string(),
                        fields,
                    })
                }
                _ => Err(FerriError::Runtime {
                    message: format!("unknown Server method `{method_name}`"),
                    line: span.line,
                    column: span.column,
                }),
            },
            "Response" => match method_name {
                "text" => {
                    check_arg_count("Response::text", 1, args, span)?;
                    let text = expect_string(&args[0], "Response::text()", span)?;
                    server::response_text(text, span)
                }
                "json" => {
                    check_arg_count("Response::json", 1, args, span)?;
                    let body = expect_string(&args[0], "Response::json()", span)?;
                    server::response_json(body, span)
                }
                "html" => {
                    check_arg_count("Response::html", 1, args, span)?;
                    let body = expect_string(&args[0], "Response::html()", span)?;
                    server::response_html(body, span)
                }
                "status" => {
                    check_arg_count("Response::status", 2, args, span)?;
                    let status =
                        crate::errors::expect_integer(&args[0], "Response::status()", span)? as u16;
                    let body = expect_string(&args[1], "Response::status()", span)?;
                    server::response_with_status(status, body, span)
                }
                _ => Err(FerriError::Runtime {
                    message: format!("unknown Response method `{method_name}`"),
                    line: span.line,
                    column: span.column,
                }),
            },
            _ => Err(FerriError::Runtime {
                message: format!("unknown type `{type_name}`"),
                line: span.line,
                column: span.column,
            }),
        }
    }

    /// Handle method calls on Server values: .get(), .post(), .listen(), etc.
    pub(crate) fn call_server_method(
        &mut self,
        receiver: &Value,
        method: &str,
        args: Vec<Value>,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let server_id = self.get_server_id(receiver, span)?;

        match method {
            "get" | "post" | "put" | "delete" | "patch" => {
                self.register_route(&server_id, method, &args, span)?;
                Ok(receiver.clone())
            }
            "static_files" => {
                check_arg_count("static_files", 1, &args, span)?;
                let dir = expect_string(&args[0], "static_files()", span)?;
                self.server_static_dirs.insert(server_id, dir.to_string());
                Ok(receiver.clone())
            }
            "listen" => {
                check_arg_count("listen", 1, &args, span)?;
                let addr = expect_string(&args[0], "listen()", span)?;
                self.start_server(&server_id, addr, span)
            }
            _ => Err(FerriError::Runtime {
                message: format!("unknown Server method `{method}`"),
                line: span.line,
                column: span.column,
            }),
        }
    }

    /// Extract the server ID from a Server struct value.
    fn get_server_id(&self, value: &Value, span: &Span) -> Result<String, FerriError> {
        if let Value::Struct { name, fields } = value {
            if name == "Server" {
                if let Some(Value::String(id)) = fields.get("__id") {
                    return Ok(id.clone());
                }
            }
        }
        Err(FerriError::Runtime {
            message: "expected a Server value".to_string(),
            line: span.line,
            column: span.column,
        })
    }

    /// Register a route handler for a server.
    fn register_route(
        &mut self,
        server_id: &str,
        method_str: &str,
        args: &[Value],
        span: &Span,
    ) -> Result<(), FerriError> {
        if args.len() != 2 {
            return Err(FerriError::Runtime {
                message: format!("{method_str}() takes 2 arguments (path, handler)"),
                line: span.line,
                column: span.column,
            });
        }
        let path = expect_string(&args[0], &format!("{method_str}()"), span)?;
        let handler = args[1].clone();

        let method = match method_str {
            "get" => Method::Get,
            "post" => Method::Post,
            "put" => Method::Put,
            "delete" => Method::Delete,
            "patch" => Method::Patch,
            _ => unreachable!(),
        };

        let segments = server::parse_route_pattern(path);
        let route = Route {
            method,
            pattern: path.to_string(),
            segments,
            handler,
        };

        if let Some(routes) = self.server_routes.get_mut(server_id) {
            routes.push(route);
        }
        Ok(())
    }

    /// Start the HTTP server and listen for connections.
    fn start_server(
        &mut self,
        server_id: &str,
        addr: &str,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let listener = TcpListener::bind(addr).map_err(|e| FerriError::Runtime {
            message: format!("failed to bind to {addr}: {e}"),
            line: span.line,
            column: span.column,
        })?;

        self.write_output(&format!("🚀 Oxide server listening on http://{addr}\n"));

        for stream in listener.incoming() {
            let stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };

            let raw = match server::read_request(&stream) {
                Ok(r) => r,
                Err(_) => continue,
            };

            let mut req = match server::parse_request(&raw) {
                Ok(r) => r,
                Err(_) => {
                    let resp = server::HttpResponse::new(400, "Bad Request".to_string());
                    let _ = write_response(&stream, &resp);
                    continue;
                }
            };

            // Try to match a route
            let routes = self
                .server_routes
                .get(server_id)
                .cloned()
                .unwrap_or_default();

            let mut matched = false;
            for route in &routes {
                if route.method != req.method {
                    continue;
                }
                if let Some(params) = server::match_route(&route.segments, &req.path) {
                    req.params = params;
                    let req_value = server::request_to_value(&req);

                    // Call the handler closure
                    let result =
                        self.call_function(&route.handler, &[req_value], span.line, span.column);

                    let resp = match result {
                        Ok(val) => server::value_to_response(&val),
                        Err(e) => {
                            server::HttpResponse::new(500, format!("Internal Server Error: {e}"))
                        }
                    };

                    let _ = write_response(&stream, &resp);
                    matched = true;

                    // Log the request
                    self.write_output(&format!(
                        "{} {} → {}\n",
                        req.method.as_str(),
                        req.path,
                        resp.status
                    ));
                    break;
                }
            }

            if !matched {
                // Try static files
                if let Some(static_dir) = self.server_static_dirs.get(server_id).cloned() {
                    if let Some(resp) = server::serve_static_file(&static_dir, &req.path) {
                        let _ = write_response(&stream, &resp);
                        self.write_output(&format!(
                            "{} {} → {} (static)\n",
                            req.method.as_str(),
                            req.path,
                            resp.status
                        ));
                        continue;
                    }
                }

                // 404
                let resp = server::HttpResponse::new(404, "Not Found".to_string());
                let _ = write_response(&stream, &resp);
                self.write_output(&format!("{} {} → 404\n", req.method.as_str(), req.path));
            }
        }

        Ok(Value::Unit)
    }
}

/// Write an HTTP response to a TCP stream.
fn write_response(
    stream: &std::net::TcpStream,
    resp: &server::HttpResponse,
) -> std::io::Result<()> {
    use std::io::Write;
    let mut stream = stream;
    stream.write_all(resp.to_http_string().as_bytes())?;
    stream.flush()
}

use std::net::TcpListener;
