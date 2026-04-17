//! Standalone HTTP health-check invoked via `parkhub-server --health-check`.
//!
//! Uses raw TCP to avoid pulling in an HTTP client at this level —
//! required for distroless container builds where no shell / no external
//! probe binary is available, only the server binary itself.

/// Perform a synchronous HTTP health check against a running server.
///
/// Connects to `http://127.0.0.1:{port}/health` using a raw TCP connection
/// (no extra runtime or external binary required — works in distroless images).
/// Returns 0 if the server responds with HTTP 200, 1 otherwise.
pub(crate) fn perform_health_check(port: u16) -> i32 {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let addr = format!("127.0.0.1:{port}");
    let timeout = Duration::from_secs(4);

    // Parse the socket address — this is always valid since port is a u16,
    // but we handle the error gracefully rather than panicking.
    let Ok(socket_addr) = addr.parse() else {
        eprintln!("health-check: could not parse address {addr}");
        return 1;
    };

    let Ok(mut stream) = TcpStream::connect_timeout(&socket_addr, timeout) else {
        eprintln!("health-check: could not connect to {addr}");
        return 1;
    };

    let _ = stream.set_read_timeout(Some(timeout));
    let req = "GET /health HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
    if stream.write_all(req.as_bytes()).is_err() {
        eprintln!("health-check: failed to send request");
        return 1;
    }

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);

    // Accept any 2xx status on the first line
    if response.starts_with("HTTP/1.") && response.lines().next().is_some_and(|l| l.contains("200"))
    {
        0
    } else {
        eprintln!(
            "health-check: unexpected response: {}",
            response.lines().next().unwrap_or("(empty)")
        );
        1
    }
}
