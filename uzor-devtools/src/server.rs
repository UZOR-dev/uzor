//! The loopback HTTP/1.1 transport (std only).
//!
//! * binds `127.0.0.1:0` — nothing outside the machine can connect;
//! * `Host` must be `127.0.0.1:<port>` or `localhost:<port>` (DNS rebinding);
//! * any `Origin` header is refused with 403 (no browser page may call it);
//! * `Authorization: Bearer <token>` is required (401), compared in constant time.
//!
//! Routes: `POST /rpc` (body: one request, see [`crate::protocol`]) and
//! `GET /events?since=<seq>&wait_ms=<ms>` (a JSON array, long-polled).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::protocol::Dispatcher;
use crate::token::Token;

const MAX_HEADER: usize = 16 * 1024;
const MAX_BODY: usize = 1024 * 1024;
const MAX_WAIT_MS: u64 = 30_000;

/// A running server; dropping it stops accepting.
pub struct Server {
    port: u16,
    stop: Arc<AtomicBool>,
    accept: Option<JoinHandle<()>>,
}

impl Server {
    /// Bind `127.0.0.1:0` and serve `dispatcher` behind `token`.
    pub fn start(dispatcher: Arc<Dispatcher>, token: Token) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = Arc::clone(&stop);
        let token = Arc::new(token);
        let accept = std::thread::Builder::new().name("uzor-devtools".into()).spawn(move || {
            for conn in listener.incoming() {
                if stop2.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(stream) = conn else { continue };
                let d = Arc::clone(&dispatcher);
                let t = Arc::clone(&token);
                let _ = std::thread::Builder::new()
                    .name("uzor-devtools-conn".into())
                    .spawn(move || serve(stream, &d, &t, port));
            }
        })?;
        Ok(Self { port, stop, accept: Some(accept) })
    }

    /// The bound port.
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Wake the accept loop so it sees the flag.
        let _ = TcpStream::connect_timeout(&([127, 0, 0, 1], self.port).into(), Duration::from_millis(200));
        if let Some(h) = self.accept.take() {
            let _ = h.join();
        }
    }
}

/// A parsed request.
#[derive(Debug, Default)]
pub struct Request {
    /// Method (`GET`, `POST`).
    pub method: String,
    /// Path with query.
    pub target: String,
    /// Lower-cased header names with values.
    pub headers: Vec<(String, String)>,
    /// Body bytes.
    pub body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
}

fn read_request(stream: &TcpStream) -> Result<Request, (u16, &'static str)> {
    let mut r = BufReader::new(stream);
    let mut req = Request::default();
    let mut line = String::new();
    let mut total = 0usize;
    let mut first = true;
    loop {
        line.clear();
        let n = r.read_line(&mut line).map_err(|_| (400, "bad_request"))?;
        if n == 0 {
            return Err((400, "bad_request"));
        }
        total += n;
        if total > MAX_HEADER {
            return Err((431, "headers_too_large"));
        }
        let l = line.trim_end_matches(['\r', '\n']);
        if first {
            let mut parts = l.split(' ');
            req.method = parts.next().unwrap_or_default().to_string();
            req.target = parts.next().unwrap_or_default().to_string();
            first = false;
            continue;
        }
        if l.is_empty() {
            break;
        }
        if let Some((k, v)) = l.split_once(':') {
            req.headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
    }
    let len: usize = match req.header("content-length") {
        Some(v) => v.parse().map_err(|_| (400, "bad_request"))?,
        None => 0,
    };
    if len > MAX_BODY {
        return Err((413, "body_too_large"));
    }
    req.body = vec![0; len];
    r.read_exact(&mut req.body).map_err(|_| (400, "bad_request"))?;
    Ok(req)
}

/// The access decision for a request, before routing. `Ok` = allowed.
pub fn check_access(req: &Request, token: &Token, port: u16) -> Result<(), (u16, &'static str)> {
    let host_ok = req
        .header("host")
        .is_some_and(|h| h == format!("127.0.0.1:{port}") || h == format!("localhost:{port}"));
    if !host_ok {
        return Err((421, "bad_host"));
    }
    if req.header("origin").is_some() {
        return Err((403, "origin_forbidden"));
    }
    let presented = req.header("authorization").and_then(|a| {
        let (scheme, rest) = a.split_once(' ')?;
        scheme.eq_ignore_ascii_case("bearer").then_some(rest)
    });
    match presented {
        Some(t) if token.verify(t) => Ok(()),
        _ => Err((401, "unauthorized")),
    }
}

fn serve(stream: TcpStream, d: &Dispatcher, token: &Token, port: u16) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
    let (status, body) = match read_request(&stream) {
        Err((code, what)) => (code, error_body(what)),
        Ok(req) => match check_access(&req, token, port) {
            Err((code, what)) => (code, error_body(what)),
            Ok(()) => route(&req, d),
        },
    };
    let mut s = stream;
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        421 => "Misdirected Request",
        431 => "Request Header Fields Too Large",
        _ => "Error",
    };
    let mut head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n",
        body.len()
    );
    if status == 401 {
        head.push_str("WWW-Authenticate: Bearer\r\n");
    }
    head.push_str("\r\n");
    let _ = s.write_all(head.as_bytes());
    let _ = s.write_all(body.as_bytes());
    let _ = s.flush();
    let _ = s.shutdown(Shutdown::Both);
}

fn route(req: &Request, d: &Dispatcher) -> (u16, String) {
    let (path, query) = req.target.split_once('?').unwrap_or((req.target.as_str(), ""));
    match (req.method.as_str(), path) {
        ("POST", "/rpc") => match std::str::from_utf8(&req.body) {
            Ok(s) => (200, d.handle_json(s)),
            Err(_) => (400, error_body("bad_request")),
        },
        ("GET", "/events") => {
            let arg = |k: &str| query.split('&').find_map(|kv| kv.strip_prefix(k)?.strip_prefix('=')?.parse::<u64>().ok());
            let since = arg("since").unwrap_or(0);
            let wait = arg("wait_ms").unwrap_or(0).min(MAX_WAIT_MS);
            (200, serde_json::Value::Array(d.events_since(since, Duration::from_millis(wait))).to_string())
        }
        (_, "/rpc") | (_, "/events") => (405, error_body("method_not_allowed")),
        _ => (404, error_body("not_found")),
    }
}

fn error_body(code: &str) -> String {
    serde_json::json!({"ok": false, "error": {"code": code, "message": code.replace('_', " ")}}).to_string()
}

/// A minimal blocking client for the CLI and tests: one request per connection.
pub fn http_call(port: u16, method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> std::io::Result<(u16, String)> {
    let mut s = TcpStream::connect(("127.0.0.1", port))?;
    s.set_read_timeout(Some(Duration::from_secs(60)))?;
    let mut req = format!("{method} {path} HTTP/1.1\r\nContent-Length: {}\r\nConnection: close\r\n", body.len());
    for (k, v) in headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str("\r\n");
    s.write_all(req.as_bytes())?;
    s.write_all(body.as_bytes())?;
    let mut raw = Vec::new();
    s.read_to_end(&mut raw)?;
    let text = String::from_utf8_lossy(&raw).to_string();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((text.as_str(), ""));
    let status = head.split(' ').nth(1).and_then(|c| c.parse().ok()).unwrap_or(0);
    Ok((status, body.to_string()))
}
