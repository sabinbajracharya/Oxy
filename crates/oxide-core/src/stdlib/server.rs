//! HTTP server module for Oxide.
//!
//! Provides a simple, Express-like HTTP server with route registration,
//! path parameters, query string parsing, JSON body parsing, headers,
//! and static file serving.
//!
//! Designed as a self-contained module with minimal coupling to the
//! interpreter — only takes/returns `Value` and `FerriError`.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};

use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

/// HTTP methods supported by the server.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

impl Method {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            "PUT" => Some(Self::Put),
            "DELETE" => Some(Self::Delete),
            "PATCH" => Some(Self::Patch),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
        }
    }
}

/// A registered route with its pattern and handler.
#[derive(Debug, Clone)]
pub struct Route {
    pub method: Method,
    pub pattern: String,
    pub segments: Vec<RouteSegment>,
    pub handler: Value,
}

/// A segment of a route pattern.
#[derive(Debug, Clone)]
pub enum RouteSegment {
    Literal(String),
    Param(String),
    Wildcard,
}

/// A parsed HTTP request.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: Method,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub params: HashMap<String, String>,
}

/// An HTTP response to send back.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpResponse {
    pub fn new(status: u16, body: String) -> Self {
        let status_text = match status {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            301 => "Moved Permanently",
            302 => "Found",
            304 => "Not Modified",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            500 => "Internal Server Error",
            _ => "OK",
        }
        .to_string();
        Self {
            status,
            status_text,
            headers: HashMap::new(),
            body,
        }
    }

    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Serialize this response into an HTTP/1.1 response string.
    pub fn to_http_string(&self) -> String {
        let mut resp = format!("HTTP/1.1 {} {}\r\n", self.status, self.status_text);
        let mut has_content_type = false;
        let mut has_content_length = false;
        for (k, v) in &self.headers {
            resp.push_str(&format!("{k}: {v}\r\n"));
            if k.to_lowercase() == "content-type" {
                has_content_type = true;
            }
            if k.to_lowercase() == "content-length" {
                has_content_length = true;
            }
        }
        if !has_content_type {
            resp.push_str("Content-Type: text/plain; charset=utf-8\r\n");
        }
        if !has_content_length {
            resp.push_str(&format!("Content-Length: {}\r\n", self.body.len()));
        }
        resp.push_str("Connection: close\r\n");
        resp.push_str("\r\n");
        resp.push_str(&self.body);
        resp
    }
}

/// Parse a route pattern into segments.
pub fn parse_route_pattern(pattern: &str) -> Vec<RouteSegment> {
    pattern
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|seg| {
            if let Some(name) = seg.strip_prefix(':') {
                RouteSegment::Param(name.to_string())
            } else if seg == "*" {
                RouteSegment::Wildcard
            } else {
                RouteSegment::Literal(seg.to_string())
            }
        })
        .collect()
}

/// Try to match a request path against a route pattern.
/// Returns extracted path parameters if matched.
pub fn match_route(route_segments: &[RouteSegment], path: &str) -> Option<HashMap<String, String>> {
    let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // Check for wildcard at end
    let has_wildcard = route_segments
        .last()
        .is_some_and(|s| matches!(s, RouteSegment::Wildcard));

    if !has_wildcard && route_segments.len() != path_segments.len() {
        return None;
    }
    if has_wildcard && path_segments.len() < route_segments.len() - 1 {
        return None;
    }

    let mut params = HashMap::new();
    for (i, seg) in route_segments.iter().enumerate() {
        match seg {
            RouteSegment::Literal(expected) => {
                if i >= path_segments.len() || path_segments[i] != expected.as_str() {
                    return None;
                }
            }
            RouteSegment::Param(name) => {
                if i >= path_segments.len() {
                    return None;
                }
                params.insert(name.clone(), path_segments[i].to_string());
            }
            RouteSegment::Wildcard => {
                // Matches rest of path
                return Some(params);
            }
        }
    }
    Some(params)
}

/// Parse a raw HTTP request from a TCP stream.
pub fn parse_request(raw: &str) -> Result<HttpRequest, String> {
    let mut lines = raw.lines();

    let request_line = lines.next().ok_or("empty request")?;
    let parts: Vec<&str> = request_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err("malformed request line".to_string());
    }

    let method = Method::parse(parts[0]).ok_or_else(|| format!("unknown method: {}", parts[0]))?;

    let full_path = parts[1];
    let (path, query) = if let Some(idx) = full_path.find('?') {
        let path = &full_path[..idx];
        let query_str = &full_path[idx + 1..];
        (path.to_string(), parse_query_string(query_str))
    } else {
        (full_path.to_string(), HashMap::new())
    };

    let mut headers = HashMap::new();
    for line in lines.by_ref() {
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_lowercase(), value.trim().to_string());
        }
    }

    let body: String = lines.collect::<Vec<&str>>().join("\n");

    Ok(HttpRequest {
        method,
        path,
        query,
        headers,
        body,
        params: HashMap::new(),
    })
}

/// Parse a query string into key-value pairs.
pub fn parse_query_string(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter(|s| !s.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (url_decode(key), url_decode(value))
        })
        .collect()
}

/// Basic URL decoding (percent-encoded strings).
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert an HttpRequest into a Oxide `Value::Struct` for the handler.
pub fn request_to_value(req: &HttpRequest) -> Value {
    let mut fields = HashMap::new();
    fields.insert(
        "method".to_string(),
        Value::String(req.method.as_str().to_string()),
    );
    fields.insert("path".to_string(), Value::String(req.path.clone()));
    fields.insert("body".to_string(), Value::String(req.body.clone()));

    // Query as HashMap
    let query_map: HashMap<String, Value> = req
        .query
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    fields.insert("query".to_string(), Value::HashMap(query_map));

    // Headers as HashMap
    let header_map: HashMap<String, Value> = req
        .headers
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    fields.insert("headers".to_string(), Value::HashMap(header_map));

    // Params as HashMap
    let param_map: HashMap<String, Value> = req
        .params
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    fields.insert("params".to_string(), Value::HashMap(param_map));

    Value::Struct {
        name: "Request".to_string(),
        fields,
    }
}

/// Convert a Oxide `Value` (returned by handler) into an HttpResponse.
pub fn value_to_response(val: &Value) -> HttpResponse {
    match val {
        // If handler returns a struct with status/body/headers, use it
        Value::Struct { name, fields } if name == "Response" => {
            let status = fields
                .get("status")
                .and_then(|v| {
                    if let Value::Integer(n) = v {
                        Some(*n as u16)
                    } else {
                        None
                    }
                })
                .unwrap_or(200);
            let body = fields
                .get("body")
                .map(|v| format!("{v}"))
                .unwrap_or_default();
            let mut resp = HttpResponse::new(status, body);
            if let Some(Value::String(ct)) = fields.get("content_type") {
                resp = resp.with_header("Content-Type", ct);
            }
            if let Some(Value::HashMap(hdrs)) = fields.get("headers") {
                for (k, v) in hdrs {
                    resp = resp.with_header(k, &format!("{v}"));
                }
            }
            resp
        }
        // If handler returns a string, wrap as 200 text response
        Value::String(s) => HttpResponse::new(200, s.clone()),
        // Anything else, convert to string
        other => HttpResponse::new(200, format!("{other}")),
    }
}

/// Try to serve a static file from the given directory.
pub fn serve_static_file(static_dir: &str, path: &str) -> Option<HttpResponse> {
    // Prevent directory traversal
    if path.contains("..") {
        return Some(HttpResponse::new(403, "Forbidden".to_string()));
    }

    let file_path = if path == "/" {
        format!("{static_dir}/index.html")
    } else {
        format!("{static_dir}{path}")
    };

    match std::fs::read_to_string(&file_path) {
        Ok(content) => {
            let content_type = guess_content_type(&file_path);
            Some(HttpResponse::new(200, content).with_header("Content-Type", content_type))
        }
        Err(_) => None,
    }
}

/// Guess Content-Type from file extension.
fn guess_content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("txt") => "text/plain; charset=utf-8",
        Some("xml") => "application/xml; charset=utf-8",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

/// Read a full HTTP request from a TCP stream.
pub fn read_request(stream: &std::net::TcpStream) -> Result<String, String> {
    let mut reader = BufReader::new(stream);
    let mut request = String::new();
    let mut content_length: usize = 0;

    // Read headers
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if line.trim().is_empty() {
                    request.push_str(&line);
                    break;
                }
                if line.to_lowercase().starts_with("content-length:") {
                    if let Some(len_str) = line.split(':').nth(1) {
                        content_length = len_str.trim().parse().unwrap_or(0);
                    }
                }
                request.push_str(&line);
            }
            Err(e) => return Err(format!("read error: {e}")),
        }
    }

    // Read body if Content-Length specified
    if content_length > 0 {
        let mut body = vec![0u8; content_length];
        reader
            .read_exact(&mut body)
            .map_err(|e| format!("body read error: {e}"))?;
        request.push_str(&String::from_utf8_lossy(&body));
    }

    Ok(request)
}

/// Create a Response::text() value.
pub fn response_text(text: &str, _span: &Span) -> Result<Value, FerriError> {
    let mut fields = HashMap::new();
    fields.insert("status".to_string(), Value::Integer(200));
    fields.insert("body".to_string(), Value::String(text.to_string()));
    fields.insert(
        "content_type".to_string(),
        Value::String("text/plain; charset=utf-8".to_string()),
    );
    Ok(Value::Struct {
        name: "Response".to_string(),
        fields,
    })
}

/// Create a Response::json() value.
pub fn response_json(body: &str, _span: &Span) -> Result<Value, FerriError> {
    let mut fields = HashMap::new();
    fields.insert("status".to_string(), Value::Integer(200));
    fields.insert("body".to_string(), Value::String(body.to_string()));
    fields.insert(
        "content_type".to_string(),
        Value::String("application/json; charset=utf-8".to_string()),
    );
    Ok(Value::Struct {
        name: "Response".to_string(),
        fields,
    })
}

/// Create a Response::html() value.
pub fn response_html(body: &str, _span: &Span) -> Result<Value, FerriError> {
    let mut fields = HashMap::new();
    fields.insert("status".to_string(), Value::Integer(200));
    fields.insert("body".to_string(), Value::String(body.to_string()));
    fields.insert(
        "content_type".to_string(),
        Value::String("text/html; charset=utf-8".to_string()),
    );
    Ok(Value::Struct {
        name: "Response".to_string(),
        fields,
    })
}

/// Create a Response::status() value.
pub fn response_with_status(status: u16, body: &str, _span: &Span) -> Result<Value, FerriError> {
    let mut fields = HashMap::new();
    fields.insert("status".to_string(), Value::Integer(status as i64));
    fields.insert("body".to_string(), Value::String(body.to_string()));
    fields.insert(
        "content_type".to_string(),
        Value::String("text/plain; charset=utf-8".to_string()),
    );
    Ok(Value::Struct {
        name: "Response".to_string(),
        fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_route_pattern() {
        let segs = parse_route_pattern("/users/:id/posts");
        assert_eq!(segs.len(), 3);
        assert!(matches!(&segs[0], RouteSegment::Literal(s) if s == "users"));
        assert!(matches!(&segs[1], RouteSegment::Param(s) if s == "id"));
        assert!(matches!(&segs[2], RouteSegment::Literal(s) if s == "posts"));
    }

    #[test]
    fn test_match_route_exact() {
        let segs = parse_route_pattern("/hello");
        let params = match_route(&segs, "/hello");
        assert!(params.is_some());
        assert!(params.unwrap().is_empty());
    }

    #[test]
    fn test_match_route_with_param() {
        let segs = parse_route_pattern("/users/:id");
        let params = match_route(&segs, "/users/42").unwrap();
        assert_eq!(params.get("id").unwrap(), "42");
    }

    #[test]
    fn test_match_route_no_match() {
        let segs = parse_route_pattern("/users/:id");
        assert!(match_route(&segs, "/posts/1").is_none());
    }

    #[test]
    fn test_match_route_wildcard() {
        let segs = parse_route_pattern("/static/*");
        assert!(match_route(&segs, "/static/css/style.css").is_some());
        assert!(match_route(&segs, "/static/").is_some());
    }

    #[test]
    fn test_parse_query_string() {
        let q = parse_query_string("name=oxide&version=1.0&empty=");
        assert_eq!(q.get("name").unwrap(), "oxide");
        assert_eq!(q.get("version").unwrap(), "1.0");
        assert_eq!(q.get("empty").unwrap(), "");
    }

    #[test]
    fn test_parse_request() {
        let raw =
            "GET /users/42?page=1 HTTP/1.1\r\nHost: localhost\r\nContent-Type: text/plain\r\n\r\n";
        let req = parse_request(raw).unwrap();
        assert_eq!(req.method, Method::Get);
        assert_eq!(req.path, "/users/42");
        assert_eq!(req.query.get("page").unwrap(), "1");
        assert_eq!(req.headers.get("host").unwrap(), "localhost");
    }

    #[test]
    fn test_parse_request_with_body() {
        let raw =
            "POST /api/data HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"key\":\"value\"}";
        let req = parse_request(raw).unwrap();
        assert_eq!(req.method, Method::Post);
        assert_eq!(req.body, "{\"key\":\"value\"}");
    }

    #[test]
    fn test_response_to_http_string() {
        let resp = HttpResponse::new(200, "hello".to_string());
        let http = resp.to_http_string();
        assert!(http.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(http.contains("Content-Length: 5"));
        assert!(http.ends_with("hello"));
    }

    #[test]
    fn test_url_decode() {
        assert_eq!(url_decode("hello%20world"), "hello world");
        assert_eq!(url_decode("a+b"), "a b");
        assert_eq!(url_decode("test%21"), "test!");
    }

    #[test]
    fn test_guess_content_type() {
        assert_eq!(guess_content_type("style.css"), "text/css; charset=utf-8");
        assert_eq!(
            guess_content_type("app.js"),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(guess_content_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(
            guess_content_type("data.json"),
            "application/json; charset=utf-8"
        );
    }

    #[test]
    fn test_value_to_response_string() {
        let val = Value::String("hello".to_string());
        let resp = value_to_response(&val);
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, "hello");
    }
}
