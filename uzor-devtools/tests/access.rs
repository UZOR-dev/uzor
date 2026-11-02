//! The transport alone, with a stand-in RPC core: access checks, routing,
//! request limits and the event route.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use uzor_devtools::events::EventLog;
use uzor_devtools::server::{http_call, Server};
use uzor_devtools::token::Token;
use uzor_devtools::Rpc;

struct Echo(EventLog);
impl Rpc for Echo {
    fn handle_json(&self, request: &str) -> String {
        self.0.emit(json!({"type": "rpc"}));
        json!({"ok": true, "result": request}).to_string()
    }
    fn events_since(&self, since: u64, wait: Duration) -> Vec<Value> {
        self.0.since(since, wait)
    }
}

const TOKEN: [u8; 32] = [3; 32];

fn server() -> Server {
    Server::start(Arc::new(Echo(EventLog::new())), Token::from_bytes(TOKEN)).unwrap()
}

#[test]
fn access_checks() {
    let s = server();
    let port = s.port();
    let host = format!("127.0.0.1:{port}");
    let hex = Token::from_bytes(TOKEN).to_hex();
    let good = format!("Bearer {hex}");
    let bad = format!("Bearer {}", "00".repeat(32));
    let body = r#"{"method":"ping"}"#;
    let call = |h: &[(&str, &str)]| http_call(port, "POST", "/rpc", h, body).unwrap().0;
    assert_eq!(call(&[("Host", &host)]), 401, "no bearer");
    assert_eq!(call(&[("Host", &host), ("Authorization", &bad)]), 401, "wrong bearer");
    assert_eq!(call(&[("Host", &host), ("Authorization", &hex)]), 401, "not bearer scheme");
    assert_eq!(call(&[("Host", &host), ("Authorization", &good), ("Origin", "http://evil.example")]), 403, "origin");
    assert_eq!(call(&[("Host", &host), ("Origin", "null")]), 403, "origin before auth");
    assert_eq!(call(&[("Host", "evil.example"), ("Authorization", &good)]), 421, "rebinding host");
    assert_eq!(call(&[("Authorization", &good)]), 421, "no host");
    assert_eq!(call(&[("Host", &format!("localhost:{port}")), ("Authorization", &good)]), 200);
    assert_eq!(http_call(port, "GET", "/nope", &[("Host", &host), ("Authorization", &good)], "").unwrap().0, 404);
    assert_eq!(http_call(port, "GET", "/rpc", &[("Host", &host), ("Authorization", &good)], "").unwrap().0, 405);
    let (_, text) = http_call(port, "POST", "/rpc", &[("Host", &host), ("Authorization", &bad)], body).unwrap();
    assert!(!text.contains(&hex));
    let (status, text) = http_call(port, "GET", "/events?since=0&wait_ms=10", &[("Host", &host), ("Authorization", &good)], "").unwrap();
    assert_eq!(status, 200);
    let ev: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(ev.as_array().unwrap().len(), 1, "{text}");
}
