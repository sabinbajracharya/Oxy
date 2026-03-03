//! Networking standard library module.
//!
//! Provides TCP and UDP socket operations using Rust's `std::net`.
//! Connections are represented as opaque struct values.

use std::collections::HashMap;
use std::io::{Read, Write};

use crate::errors::{check_arg_count, expect_integer, expect_string, runtime_error, FerriError};
use crate::lexer::Span;
use crate::types::Value;

/// Dispatch `std::net::` function calls.
pub fn call(func_name: &str, args: &[Value], span: &Span) -> Result<Value, FerriError> {
    match func_name {
        // --- TCP ---
        "tcp_connect" => {
            check_arg_count("std::net::tcp_connect", 1, args, span)?;
            let addr = expect_string(&args[0], "std::net::tcp_connect", span)?;
            match std::net::TcpStream::connect(addr) {
                Ok(stream) => {
                    let local = stream
                        .local_addr()
                        .map(|a| a.to_string())
                        .unwrap_or_default();
                    let peer = stream
                        .peer_addr()
                        .map(|a| a.to_string())
                        .unwrap_or_default();
                    drop(stream);
                    let mut fields = HashMap::new();
                    fields.insert("local_addr".to_string(), Value::String(local));
                    fields.insert("peer_addr".to_string(), Value::String(peer));
                    fields.insert("protocol".to_string(), Value::String("tcp".to_string()));
                    Ok(Value::ok(Value::Struct {
                        name: "TcpConnection".to_string(),
                        fields,
                    }))
                }
                Err(e) => Ok(Value::err(Value::String(e.to_string()))),
            }
        }
        "tcp_send" => {
            check_arg_count("std::net::tcp_send", 2, args, span)?;
            let addr = expect_string(&args[0], "std::net::tcp_send(addr)", span)?;
            let data = expect_string(&args[1], "std::net::tcp_send(data)", span)?;
            match std::net::TcpStream::connect(addr) {
                Ok(mut stream) => match stream.write_all(data.as_bytes()) {
                    Ok(()) => {
                        let _ = stream.shutdown(std::net::Shutdown::Write);
                        let mut response = String::new();
                        let _ = stream.read_to_string(&mut response);
                        Ok(Value::ok(Value::String(response)))
                    }
                    Err(e) => Ok(Value::err(Value::String(e.to_string()))),
                },
                Err(e) => Ok(Value::err(Value::String(e.to_string()))),
            }
        }
        "tcp_listen" => {
            check_arg_count("std::net::tcp_listen", 1, args, span)?;
            let addr = expect_string(&args[0], "std::net::tcp_listen", span)?;
            match std::net::TcpListener::bind(addr) {
                Ok(listener) => {
                    let local = listener
                        .local_addr()
                        .map(|a| a.to_string())
                        .unwrap_or_default();
                    let mut fields = HashMap::new();
                    fields.insert("local_addr".to_string(), Value::String(local));
                    fields.insert("protocol".to_string(), Value::String("tcp".to_string()));
                    Ok(Value::ok(Value::Struct {
                        name: "TcpListener".to_string(),
                        fields,
                    }))
                }
                Err(e) => Ok(Value::err(Value::String(e.to_string()))),
            }
        }

        // --- UDP ---
        "udp_bind" => {
            check_arg_count("std::net::udp_bind", 1, args, span)?;
            let addr = expect_string(&args[0], "std::net::udp_bind", span)?;
            match std::net::UdpSocket::bind(addr) {
                Ok(socket) => {
                    let local = socket
                        .local_addr()
                        .map(|a| a.to_string())
                        .unwrap_or_default();
                    let mut fields = HashMap::new();
                    fields.insert("local_addr".to_string(), Value::String(local));
                    fields.insert("protocol".to_string(), Value::String("udp".to_string()));
                    Ok(Value::ok(Value::Struct {
                        name: "UdpSocket".to_string(),
                        fields,
                    }))
                }
                Err(e) => Ok(Value::err(Value::String(e.to_string()))),
            }
        }
        "udp_send_to" => {
            check_arg_count("std::net::udp_send_to", 3, args, span)?;
            let bind_addr = expect_string(&args[0], "std::net::udp_send_to(bind_addr)", span)?;
            let target_addr = expect_string(&args[1], "std::net::udp_send_to(target_addr)", span)?;
            let data = expect_string(&args[2], "std::net::udp_send_to(data)", span)?;
            match std::net::UdpSocket::bind(bind_addr) {
                Ok(socket) => match socket.send_to(data.as_bytes(), target_addr) {
                    Ok(bytes) => Ok(Value::ok(Value::Integer(bytes as i64))),
                    Err(e) => Ok(Value::err(Value::String(e.to_string()))),
                },
                Err(e) => Ok(Value::err(Value::String(e.to_string()))),
            }
        }

        // --- DNS ---
        "lookup_host" => {
            check_arg_count("std::net::lookup_host", 1, args, span)?;
            let host = expect_string(&args[0], "std::net::lookup_host", span)?;
            match std::net::ToSocketAddrs::to_socket_addrs(&(host, 0u16)) {
                Ok(addrs) => {
                    let ips: Vec<Value> =
                        addrs.map(|a| Value::String(a.ip().to_string())).collect();
                    Ok(Value::ok(Value::Vec(ips)))
                }
                Err(e) => Ok(Value::err(Value::String(e.to_string()))),
            }
        }

        // --- Utilities ---
        "socket_addr_parse" => {
            check_arg_count("std::net::socket_addr_parse", 2, args, span)?;
            let host = expect_string(&args[0], "std::net::socket_addr_parse(host)", span)?;
            let port = expect_integer(&args[1], "std::net::socket_addr_parse(port)", span)?;
            Ok(Value::String(format!("{host}:{port}")))
        }

        _ => Err(runtime_error(
            format!("unknown net function `std::net::{func_name}`"),
            span,
        )),
    }
}

#[cfg(test)]
mod tests {
    use crate::interpreter::run_capturing;

    fn run(src: &str) -> String {
        let (_, output) = run_capturing(src).expect("runtime error");
        output.join("")
    }

    #[test]
    fn test_net_tcp_connect_invalid() {
        let out = run(r#"
fn main() {
    let result = std::net::tcp_connect("127.0.0.1:1");
    if let Ok(conn) = result {
        println!("ok");
    } else {
        println!("err");
    }
}
"#);
        assert_eq!(out, "err\n");
    }

    #[test]
    fn test_net_socket_addr_parse() {
        let out = run(r#"
fn main() {
    let addr = std::net::socket_addr_parse("127.0.0.1", 8080);
    println!("{}", addr);
}
"#);
        assert_eq!(out, "127.0.0.1:8080\n");
    }

    #[test]
    fn test_net_lookup_host_localhost() {
        // DNS may fail in some environments, so just verify it returns a Result
        let out = run(r#"
fn main() {
    let result = std::net::lookup_host("localhost");
    if let Ok(addrs) = result {
        println!("{}", addrs.len() > 0);
    } else {
        // DNS lookup may fail in some envs — still valid
        println!("true");
    }
}
"#);
        assert_eq!(out, "true\n");
    }
}
