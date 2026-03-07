//! HTTP client module wrapping `ureq` for the Oxide interpreter.

use std::collections::HashMap;

/// Make an HTTP request and return (status_code, body, headers).
///
/// HTTP error responses (4xx, 5xx) are returned as successful results
/// with the status code indicating the error — they are not Rust errors.
pub fn request(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<&str>,
) -> Result<(i64, String, HashMap<String, String>), String> {
    let agent = ureq::agent();

    let mut req = match method.to_uppercase().as_str() {
        "GET" => agent.get(url),
        "POST" => agent.post(url),
        "PUT" => agent.put(url),
        "DELETE" => agent.delete(url),
        "PATCH" => agent.request("PATCH", url),
        other => return Err(format!("unsupported HTTP method: {other}")),
    };

    for (key, value) in headers {
        req = req.set(key, value);
    }

    let response = if let Some(body_str) = body {
        req.send_string(body_str)
    } else {
        req.call()
    };

    match response {
        Ok(resp) => read_response(resp),
        Err(ureq::Error::Status(code, resp)) => {
            let mut resp_headers = HashMap::new();
            for name in resp.headers_names() {
                if let Some(val) = resp.header(&name) {
                    resp_headers.insert(name, val.to_string());
                }
            }
            let body_text = resp.into_string().unwrap_or_default();
            Ok((code as i64, body_text, resp_headers))
        }
        Err(ureq::Error::Transport(t)) => Err(format!("HTTP transport error: {t}")),
    }
}

fn read_response(resp: ureq::Response) -> Result<(i64, String, HashMap<String, String>), String> {
    let status = resp.status() as i64;
    let mut resp_headers = HashMap::new();
    for name in resp.headers_names() {
        if let Some(val) = resp.header(&name) {
            resp_headers.insert(name, val.to_string());
        }
    }
    let body_text = resp
        .into_string()
        .map_err(|e| format!("failed to read response body: {e}"))?;
    Ok((status, body_text, resp_headers))
}
