//! Dev control end to end over loopback (feature `dev-control`): a headless
//! app, the real server, HTTP calls.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine as _;
use serde_json::{json, Value};
use uzor::layout::docking::DockPanel;
use uzor::types::Rect;
use uzor_framework::dev_control::devtools::server::{http_call, Server};
use uzor_framework::dev_control::devtools::token::Token;
use uzor_framework::dev_control::Dispatcher;
use uzor_framework::automation::{channel, AutomationPort};
use uzor_framework::handle::{App, InitCx, IntentCx, OverlayCx, PanelCx};
use uzor_framework::Handle;
use uzor_framework::types::command::{AppCommand, DockTarget, LayoutCmd};
use uzor_framework::types::intent::Intent;
use uzor_framework::types::overlay_model::OverlayModel;
use uzor_framework::types::spec::{PanelHome, Spec};
use uzor_framework::types::ids::WindowId;
use uzor_framework::types::window::SizePx;
use uzor_framework::{HeadlessHost, RuntimeConfig};

#[derive(Clone, Debug)]
struct P;
impl DockPanel for P {
    fn title(&self) -> &str {
        "rpc"
    }
    fn type_id(&self) -> &'static str {
        "rpc"
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Ov {}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Act {}
struct TS;
impl Spec for TS {
    type Panel = P;
    type Overlay = Ov;
    type Action = Act;
    fn decode_panel(_h: PanelHome, _t: &str) -> Option<P> {
        Some(P)
    }
}

#[derive(Default)]
struct Form {
    clicks: u32,
    /// The app handle, for tests that run the app on another thread.
    handle_out: Arc<Mutex<Option<Handle<TS>>>>,
}
impl App for Form {
    type Spec = TS;
    fn init(&mut self, cx: &mut InitCx<'_, TS>) {
        *self.handle_out.lock().unwrap() = Some(cx.handle());
    }
    fn overlay_model(&self, _w: WindowId, id: Ov) -> OverlayModel {
        match id {}
    }
    fn panel(&mut self, cx: &mut PanelCx<'_, TS>) {
        use uzor_framework::widgets::{kit, lm};
        lm::button("go", Rect::new(20.0, 20.0, 120.0, 32.0)).text("Go").bind_count(&mut self.clicks).build(cx);
        kit::field("nick", Rect::new(20.0, 70.0, 200.0, 28.0)).name("Nick").build(cx);
        kit::field("token", Rect::new(20.0, 110.0, 200.0, 28.0)).name("Token").masked(true).build(cx);
    }
    fn overlay_body(&mut self, _cx: &mut OverlayCx<'_, TS>) {}
    fn intent(&mut self, _i: Intent<TS>, _cx: &mut IntentCx<'_, TS>) {}
    fn automation_state(&self) -> Option<String> {
        Some(format!("{{\"clicks\":{}}}", self.clicks))
    }
}

const TOKEN: [u8; 32] = [7; 32];

/// Serve a headless app on this thread while `body` runs on another.
fn with_app(body: impl FnOnce(u16) + Send + 'static) {
    let (mut host, handle) = HeadlessHost::new(Form::default(), RuntimeConfig::default());
    let win = host.open_window(SizePx::new(400, 300), 1.0);
    handle.dispatch(AppCommand::Layout(LayoutCmd::OpenPanel { win, panel: P, at: DockTarget::Root })).unwrap();
    host.advance(0.017);
    host.advance(0.017);
    let (client, port): (_, AutomationPort) = channel();
    let server = Server::start(Arc::new(Dispatcher::new("test", client)), Token::from_bytes(TOKEN)).unwrap();
    let p = server.port();
    let t = std::thread::spawn(move || body(p));
    while !t.is_finished() {
        host.serve_automation(&port, 0.017);
        std::thread::sleep(Duration::from_millis(1));
    }
    t.join().unwrap();
}

fn hex() -> String {
    Token::from_bytes(TOKEN).to_hex()
}

fn rpc(port: u16, method: &str, params: Value) -> Value {
    let auth = format!("Bearer {}", hex());
    let host = format!("127.0.0.1:{port}");
    let body = json!({"id": 9, "method": method, "params": params}).to_string();
    let (status, text) = http_call(port, "POST", "/rpc", &[("Host", &host), ("Authorization", &auth)], &body).unwrap();
    assert_eq!(status, 200, "{text}");
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["id"], 9);
    v
}

#[test]
fn access_checks() {
    with_app(|port| {
        let host = format!("127.0.0.1:{port}");
        let good = format!("Bearer {}", hex());
        let bad = format!("Bearer {}", "00".repeat(32));
        let body = r#"{"method":"ping"}"#;
        let call = |h: &[(&str, &str)]| http_call(port, "POST", "/rpc", h, body).unwrap().0;
        assert_eq!(call(&[("Host", &host)]), 401, "no bearer");
        assert_eq!(call(&[("Host", &host), ("Authorization", &bad)]), 401, "wrong bearer");
        assert_eq!(call(&[("Host", &host), ("Authorization", &hex())]), 401, "not bearer scheme");
        assert_eq!(call(&[("Host", &host), ("Authorization", &good), ("Origin", "http://evil.example")]), 403, "origin");
        assert_eq!(call(&[("Host", &host), ("Origin", "null")]), 403, "origin before auth");
        assert_eq!(call(&[("Host", "evil.example"), ("Authorization", &good)]), 421, "rebinding host");
        assert_eq!(call(&[("Authorization", &good)]), 421, "no host");
        let local = format!("localhost:{port}");
        assert_eq!(call(&[("Host", &local), ("Authorization", &good)]), 200);
        assert_eq!(http_call(port, "GET", "/nope", &[("Host", &host), ("Authorization", &good)], "").unwrap().0, 404);
        assert_eq!(http_call(port, "GET", "/rpc", &[("Host", &host), ("Authorization", &good)], "").unwrap().0, 405);
        // A 401 body says nothing about the token.
        let (_, text) = http_call(port, "POST", "/rpc", &[("Host", &host), ("Authorization", &bad)], body).unwrap();
        assert!(!text.contains(&hex()));
    });
}

#[test]
fn drive_the_app() {
    with_app(|port| {
        let ping = rpc(port, "ping", json!({}));
        assert_eq!(ping["result"]["protocol"], 1);

        let wins = rpc(port, "windows", json!({}));
        assert_eq!(wins["result"].as_array().unwrap().len(), 1);

        let tree = rpc(port, "dump-tree", json!({}));
        let nodes = tree["result"]["nodes"].as_array().unwrap();
        assert!(nodes.iter().any(|n| n["id"] == "go" && n["role"] == "button"), "{tree}");

        let c = rpc(port, "click", json!({"selector": "#go"}));
        assert_eq!(c["ok"], true, "{c}");
        assert_eq!(rpc(port, "state", json!({}))["result"]["clicks"], 1);
        let c = rpc(port, "click", json!({"selector": "button[id^=g]"}));
        assert_eq!(c["ok"], true, "{c} {}", rpc(port, "dump-tree", json!({})));
        assert_eq!(rpc(port, "state", json!({}))["result"]["clicks"], 2);

        assert_eq!(rpc(port, "type", json!({"selector": "textinput[name=Nick]", "text": "lavr"}))["ok"], true);
        assert_eq!(rpc(port, "type", json!({"selector": "#token", "text": "s3cret"}))["ok"], true);
        let tree = rpc(port, "dump-tree", json!({})).to_string();
        assert!(tree.contains("\"lavr\""), "{tree}");
        assert!(!tree.contains("s3cret"), "masked value leaked: {tree}");
        assert!(tree.contains("••••••"));

        let f = rpc(port, "focus", json!({"selector": "#nick"}));
        assert_eq!(f["result"]["focused"], true, "{f}");
        assert_eq!(rpc(port, "key", json!({"key": "Ctrl+A"}))["ok"], true);
        assert_eq!(rpc(port, "key", json!({"key": "Backspace"}))["ok"], true);
        assert!(!rpc(port, "dump-tree", json!({})).to_string().contains("\"lavr\""));

        assert_eq!(rpc(port, "hover", json!({"x": 5, "y": 5}))["ok"], true);
        assert_eq!(rpc(port, "scroll", json!({"x": 5, "y": 5, "dy": -40}))["ok"], true);

        let shot = rpc(port, "screenshot", json!({}));
        let png = base64::engine::general_purpose::STANDARD.decode(shot["result"]["png_base64"].as_str().unwrap()).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(shot["result"]["width"], 400);

        // Errors: stable codes.
        let e = |m: &str, p: Value| rpc(port, m, p)["error"]["code"].as_str().unwrap().to_string();
        assert_eq!(e("click", json!({"selector": "#missing"})), "not_found");
        assert_eq!(e("click", json!({"selector": "[["})), "bad_selector");
        assert_eq!(e("click", json!({})), "bad_request");
        assert_eq!(e("nope", json!({})), "unknown_method");
        assert_eq!(e("dump-tree", json!({"window": 99})), "no_window");
        assert_eq!(e("key", json!({"key": "Hyper+Q"})), "bad_request");

        // Events record every call.
        let auth = format!("Bearer {}", hex());
        let host = format!("127.0.0.1:{port}");
        let (s, ev) = http_call(port, "GET", "/events?since=0&wait_ms=10", &[("Host", &host), ("Authorization", &auth)], "").unwrap();
        assert_eq!(s, 200);
        let ev: Value = serde_json::from_str(&ev).unwrap();
        let ev = ev.as_array().unwrap();
        assert!(ev.len() > 10);
        assert_eq!(ev[0]["method"], "ping");
        let last = ev.last().unwrap()["seq"].as_u64().unwrap();
        let (_, none) =
            http_call(port, "GET", &format!("/events?since={last}&wait_ms=20"), &[("Host", &host), ("Authorization", &auth)], "").unwrap();
        assert_eq!(none, "[]");
    });
}

#[test]
fn malformed_requests() {
    let (client, _port) = channel();
    let d = Dispatcher::new("t", client);
    let v: Value = serde_json::from_str(&d.handle_json("not json")).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
    let v: Value = serde_json::from_str(&d.handle_json(r#"{"id":"x"}"#)).unwrap();
    assert_eq!(v["id"], "x");
    assert_eq!(v["error"]["code"], "bad_request");
    let v: Value = serde_json::from_str(&d.handle_json(r#"{"method":"ping","params":[1]}"#)).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
}

/// `dev_control::start`: the discovery file (0600 in a 0700 directory) has
/// the port and the token, and goes away with the endpoint.
#[cfg(unix)]
#[test]
fn start_publishes_discovery() {
    use std::os::unix::fs::PermissionsExt;
    let rt = std::env::temp_dir().join(format!("uzor-devctl-test-{}", std::process::id()));
    std::fs::create_dir_all(&rt).unwrap();
    std::env::set_var("XDG_RUNTIME_DIR", &rt);
    let (client, _port) = channel();
    let d = uzor_framework::dev_control::start("uzor-test", "t", client).unwrap();
    let file = d.discovery_path().to_path_buf();
    assert!(file.starts_with(rt.join("uzor-test")));
    assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(std::fs::metadata(file.parent().unwrap()).unwrap().permissions().mode() & 0o777, 0o700);
    let info: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(info["port"].as_u64(), Some(u64::from(d.port())));
    let token = info["token"].as_str().unwrap().to_string();
    assert_eq!(token.len(), 64);
    let host = format!("127.0.0.1:{}", d.port());
    let auth = format!("Bearer {token}");
    let (status, text) = http_call(d.port(), "POST", "/rpc", &[("Host", &host), ("Authorization", &auth)], r#"{"id":1,"method":"ping"}"#).unwrap();
    assert_eq!(status, 200, "{text}");
    assert!(text.contains(r#""app":"t""#), "{text}");
    drop(d);
    assert!(!file.exists());
    let _ = std::fs::remove_dir_all(&rt);
}

fn err_code(port: u16, m: &str, p: Value) -> String {
    let v = rpc(port, m, p);
    v["error"]["code"].as_str().unwrap_or_else(|| panic!("{m} did not fail: {v}")).to_string()
}

fn events(port: u16) -> Vec<Value> {
    let auth = format!("Bearer {}", hex());
    let host = format!("127.0.0.1:{port}");
    let (_, ev) = http_call(port, "GET", "/events?since=0&wait_ms=10", &[("Host", &host), ("Authorization", &auth)], "").unwrap();
    serde_json::from_str::<Value>(&ev).unwrap().as_array().unwrap().clone()
}

#[test]
fn wait_for_and_settle() {
    with_app(|port| {
        let w = rpc(port, "wait-for", json!({"target": "#go"}));
        assert_eq!(w["result"]["id"], "go", "{w}");
        assert_eq!(rpc(port, "wait-for", json!({"target": "#nope", "state": "gone"}))["ok"], true);
        assert_eq!(err_code(port, "wait-for", json!({"target": "#nope", "timeout_ms": 150})), "timeout");
        assert_eq!(err_code(port, "wait-for", json!({"target": "[["})), "bad_selector");
        assert_eq!(err_code(port, "wait-for", json!({})), "bad_request");
        assert_eq!(err_code(port, "wait-for", json!({"target": "#go", "state": "odd"})), "bad_request");
        assert_eq!(rpc(port, "wait-for", json!({"cond": {"clicks": 0}}))["ok"], true);
        assert_eq!(err_code(port, "wait-for", json!({"cond": {"clicks": 5}, "timeout_ms": 100})), "timeout");
        rpc(port, "click", json!({"selector": "#go"}));
        assert_eq!(rpc(port, "wait-for", json!({"cond": {"clicks": 1}}))["ok"], true);
        let s = rpc(port, "settle", json!({"idle_ms": 80}));
        assert_eq!(s["ok"], true, "{s}");
        assert!(s["result"]["waited_ms"].as_u64().unwrap() >= 80);
        assert!(s["result"]["checks"].as_u64().unwrap() >= 2, "{s}");
        assert_eq!(rpc(port, "type", json!({"selector": "#nick", "text": "ab"}))["ok"], true);
        let v = rpc(port, "wait-for", json!({"target": "#nick", "value": "ab", "state": "focused"}));
        assert_eq!(v["ok"], true, "{v}");
        assert_eq!(err_code(port, "settle", json!({"idle_ms": "x"})), "bad_request");
    });
}

#[test]
fn record_and_replay() {
    with_app(|port| {
        assert_eq!(rpc(port, "record", json!({}))["result"]["recording"], false);
        assert_eq!(rpc(port, "record", json!({"on": true}))["result"]["recording"], true);
        rpc(port, "click", json!({"selector": "#go"}));
        rpc(port, "type", json!({"selector": "#nick", "text": "abc"}));
        rpc(port, "type", json!({"selector": "#token", "text": "s3cret"}));
        rpc(port, "key", json!({"key": "Enter"}));
        rpc(port, "scroll", json!({"selector": "#go", "dy": -40}));
        rpc(port, "click", json!({"x": 390, "y": 290}));
        let off = rpc(port, "record", json!({"on": false}));
        let jsonl = off["result"]["jsonl"].as_str().unwrap().to_string();
        assert!(!jsonl.contains("s3cret"), "secret recorded: {jsonl}");
        let lines: Vec<Value> = jsonl.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        let methods: Vec<&str> = lines.iter().map(|l| l["method"].as_str().unwrap()).collect();
        assert_eq!(methods, ["click", "click", "type", "click", "type", "key", "scroll", "click"], "{jsonl}");
        assert_eq!(lines[0]["params"]["selector"], "#go");
        assert_eq!(lines[1]["params"]["selector"], "#nick");
        assert_eq!(lines[2]["params"]["text"], "abc");
        assert_eq!(lines[2]["params"]["into"], "nick");
        assert_eq!(lines[4]["params"]["text"], "<masked>");
        assert_eq!(lines[4]["params"]["into"], "token");
        assert_eq!(lines[5]["params"]["key"], "Enter");
        assert_eq!(lines[6]["params"]["selector"], "#go");
        assert_eq!(lines[6]["params"]["dy"], -40.0);
        assert!(lines[7]["params"]["selector"].is_null() && lines[7]["params"]["x"] == 390.0, "{}", lines[7]);
        let ts: Vec<u64> = lines.iter().map(|l| l["t_ms"].as_u64().unwrap()).collect();
        assert!(ts.windows(2).all(|w| w[0] <= w[1]));

        // The log streams the lines too, never the secret.
        let ev = events(port);
        assert!(ev.iter().any(|e| e["type"] == "record" && e["event"]["method"] == "type"));
        assert!(!Value::Array(ev.clone()).to_string().contains("s3cret"));

        // Replay needs the secret, refuses what it cannot replay.
        assert_eq!(err_code(port, "replay", json!({"events": jsonl.clone(), "speed": 0})), "bad_request");
        assert_eq!(err_code(port, "replay", json!({"events": jsonl.clone(), "speed": 0, "secrets": {"other": "x"}})), "bad_request");
        assert_eq!(err_code(port, "replay", json!({"events": [{"method": "screenshot"}]})), "bad_request");
        assert_eq!(err_code(port, "replay", json!({"events": "not json"})), "bad_request");
        let before = rpc(port, "state", json!({}))["result"]["clicks"].as_u64().unwrap();
        let r = rpc(port, "replay", json!({"events": jsonl.clone(), "speed": 0, "secrets": {"token": "zz"}}));
        assert_eq!(r["ok"], true, "{r}");
        assert_eq!(r["result"]["replayed"], 8);
        assert_eq!(rpc(port, "state", json!({}))["result"]["clicks"].as_u64().unwrap(), before + 1);
        let tree = rpc(port, "dump-tree", json!({})).to_string();
        assert!(tree.contains("abcabc"), "{tree}");
        // A list of secrets works in order; a speed replays with pauses.
        let r = rpc(port, "replay", json!({"events": lines, "speed": 4, "secrets": ["q"]}));
        assert_eq!(r["ok"], true, "{r}");
        let ev = events(port);
        assert!(ev.iter().any(|e| e["type"] == "replay" && e["ok"] == true));
        assert!(!Value::Array(ev).to_string().contains("\"zz\""));
        // A step whose selector never shows up fails with its index.
        let bad = json!([{"method": "click", "params": {"selector": "#gone"}}]);
        let v = rpc(port, "replay", json!({"events": bad, "speed": 0}));
        assert_eq!(v["error"]["code"], "timeout", "{v}");
        assert!(v["error"]["message"].as_str().unwrap().contains("event 0"));
    });
}

/// `run_headless`: a live loop with no window, driven over the same RPC.
#[test]
fn live_headless_app() {
    use uzor_framework::types::window::WindowSpec;
    let (client, aport) = channel();
    let slot: Arc<Mutex<Option<Handle<TS>>>> = Arc::default();
    let form = Form { clicks: 0, handle_out: Arc::clone(&slot) };
    let mut cfg = RuntimeConfig::default();
    cfg.windows.push(WindowSpec::new("main", "live", SizePx::new(320, 200)));
    let app = std::thread::spawn(move || uzor_framework::host::run_headless(form, cfg, 1.0, aport));
    let server = Server::start(Arc::new(Dispatcher::new("live", client)), Token::from_bytes(TOKEN)).unwrap();
    let port = server.port();
    let w = rpc(port, "windows", json!({}));
    let win = w["result"][0]["id"].as_u64().unwrap() as u32;
    assert_eq!(w["result"][0]["viewport"]["w"], 320.0);
    let handle = slot.lock().unwrap().clone().unwrap();
    handle.dispatch(AppCommand::Layout(LayoutCmd::OpenPanel { win: WindowId(win), panel: P, at: DockTarget::Root })).unwrap();
    // The dispatch wakes the loop; the panel shows up within frames.
    let v = rpc(port, "wait-for", json!({"target": "#go", "timeout_ms": 3000}));
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(rpc(port, "click", json!({"selector": "#go"}))["ok"], true);
    assert_eq!(rpc(port, "wait-for", json!({"cond": {"clicks": 1}, "timeout_ms": 2000}))["ok"], true);
    let st = rpc(port, "settle", json!({"idle_ms": 100}));
    assert_eq!(st["ok"], true, "{st}");
    let shot = rpc(port, "screenshot", json!({}));
    assert_eq!(shot["result"]["source"], "offscreen");
    assert_eq!(shot["result"]["width"], 320);
    handle.dispatch(AppCommand::Shutdown).unwrap();
    let t0 = std::time::Instant::now();
    while !app.is_finished() {
        assert!(t0.elapsed() < Duration::from_secs(5), "headless loop did not exit");
        std::thread::sleep(Duration::from_millis(10));
    }
    app.join().unwrap();
}
