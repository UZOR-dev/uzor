//! The transport-agnostic RPC core.
//!
//! Request: `{"id": <any>, "method": "<name>", "params": {...}}`.
//! Response: `{"id": <same>, "ok": true, "result": ...}` or
//! `{"id": <same>, "ok": false, "error": {"code": "...", "message": "..."}}`.
//! Codes and messages are plain English and stable; they are not
//! translated.

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use base64::Engine as _;
use serde_json::{json, Map, Value};
use uzor::a11y::{query::role_name, A11yNode, QueryError, Selector};
use uzor::input::{ModifierKeys, MouseButton};
use uzor::types::Rect;
use uzor_framework::automation::{AutoCmd, AutoError, AutoReply, AutoTree, AutomationClient, ShotSource};
use uzor_framework::types::ids::WindowId;
use uzor_framework::{ImeInput, InputEvent, KeyInput, KeyState, Point, PointerInput, WheelDelta, WheelInput};

use crate::keys;

/// Protocol revision reported by `ping` and the discovery file.
pub const PROTOCOL_VERSION: u32 = 1;

/// Every method `handle_json` understands.
pub const METHODS: &[&str] = &[
    "ping", "windows", "dump-tree", "click", "hover", "focus", "type", "key", "scroll", "screenshot", "state",
];

/// Events kept for `events_since` (oldest dropped first).
const EVENT_CAP: usize = 1024;
/// How long one host round trip may take.
const CALL_TIMEOUT: Duration = Duration::from_secs(10);

/// An RPC failure: a stable code and an English message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RpcError {
    /// Stable code.
    pub code: &'static str,
    /// Human-readable detail.
    pub message: String,
}

impl RpcError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
    fn bad(message: impl Into<String>) -> Self {
        Self::new("bad_request", message)
    }
}

impl From<AutoError> for RpcError {
    fn from(e: AutoError) -> Self {
        Self::new(e.code(), e.to_string())
    }
}

impl From<QueryError> for RpcError {
    fn from(e: QueryError) -> Self {
        let code = match e {
            QueryError::Parse(_) => "bad_selector",
            QueryError::NotFound => "not_found",
            QueryError::Ambiguous(_) => "ambiguous",
        };
        Self::new(code, e.to_string())
    }
}

type R<T> = Result<T, RpcError>;

struct Events {
    next: u64,
    log: VecDeque<Value>,
}

/// The RPC core: decodes requests, drives the app through its automation
/// client, encodes responses, and records an event per call.
pub struct Dispatcher {
    app: String,
    client: AutomationClient,
    events: Mutex<Events>,
    events_cv: Condvar,
    started: Instant,
}

impl Dispatcher {
    /// A dispatcher for `client`; `app` names the app in `ping`.
    pub fn new(app: &str, client: AutomationClient) -> Self {
        Self {
            app: app.to_string(),
            client,
            events: Mutex::new(Events { next: 1, log: VecDeque::new() }),
            events_cv: Condvar::new(),
            started: Instant::now(),
        }
    }

    /// One JSON request in, one JSON response out. Never panics on input.
    pub fn handle_json(&self, request: &str) -> String {
        let parsed: Result<Value, _> = serde_json::from_str(request);
        let (id, out, method) = match parsed {
            Err(e) => (Value::Null, Err(RpcError::bad(format!("request is not JSON: {e}"))), String::new()),
            Ok(v) => {
                let id = v.get("id").cloned().unwrap_or(Value::Null);
                match v.get("method").and_then(Value::as_str) {
                    None => (id, Err(RpcError::bad("missing \"method\"")), String::new()),
                    Some(m) => {
                        let params = v.get("params").cloned().unwrap_or(Value::Object(Map::new()));
                        if !params.is_object() {
                            (id, Err(RpcError::bad("\"params\" must be an object")), m.to_string())
                        } else {
                            (id, self.call(m, &params), m.to_string())
                        }
                    }
                }
            }
        };
        let ok = out.is_ok();
        let resp = match out {
            Ok(result) => json!({"id": id, "ok": true, "result": result}),
            Err(e) => json!({"id": id, "ok": false, "error": {"code": e.code, "message": e.message}}),
        };
        if !method.is_empty() {
            let mut ev = json!({"type": "rpc", "method": method, "ok": ok});
            if let Some(err) = resp.get("error").and_then(|e| e.get("code")) {
                ev["error"] = err.clone();
            }
            self.emit(ev);
        }
        resp.to_string()
    }

    /// Append an event (`seq` and `t_ms` are filled in).
    pub fn emit(&self, mut event: Value) {
        let mut ev = self.events.lock().unwrap_or_else(|p| p.into_inner());
        event["seq"] = json!(ev.next);
        event["t_ms"] = json!(self.started.elapsed().as_millis() as u64);
        ev.next += 1;
        ev.log.push_back(event);
        while ev.log.len() > EVENT_CAP {
            ev.log.pop_front();
        }
        drop(ev);
        self.events_cv.notify_all();
    }

    /// Events with `seq > since`, waiting up to `wait` for the first one.
    pub fn events_since(&self, since: u64, wait: Duration) -> Vec<Value> {
        let deadline = Instant::now() + wait;
        let mut ev = self.events.lock().unwrap_or_else(|p| p.into_inner());
        loop {
            let out: Vec<Value> = ev
                .log
                .iter()
                .filter(|e| e.get("seq").and_then(Value::as_u64).is_some_and(|s| s > since))
                .cloned()
                .collect();
            let now = Instant::now();
            if !out.is_empty() || now >= deadline {
                return out;
            }
            ev = self.events_cv.wait_timeout(ev, deadline - now).unwrap_or_else(|p| p.into_inner()).0;
        }
    }

    fn call(&self, method: &str, p: &Value) -> R<Value> {
        match method {
            "ping" => Ok(json!({
                "pong": true,
                "app": self.app,
                "pid": std::process::id(),
                "protocol": PROTOCOL_VERSION,
                "methods": METHODS,
            })),
            "windows" => self.windows(),
            "dump-tree" => self.dump_tree(p),
            "click" => self.click(p),
            "hover" => self.hover(p),
            "focus" => self.focus(p),
            "type" => self.type_text(p),
            "key" => self.key(p),
            "scroll" => self.scroll(p),
            "screenshot" => self.screenshot(p),
            "state" => self.state(),
            other => Err(RpcError::new("unknown_method", format!("unknown method '{other}'; try one of: {}", METHODS.join(", ")))),
        }
    }

    fn host(&self, cmd: AutoCmd) -> R<AutoReply> {
        Ok(self.client.call(cmd, CALL_TIMEOUT)?)
    }

    fn input(&self, win: Option<WindowId>, events: Vec<InputEvent>) -> R<()> {
        match self.host(AutoCmd::Input { win, events })? {
            AutoReply::InputDone { .. } => Ok(()),
            other => Err(RpcError::new("failed", format!("unexpected host reply {other:?}"))),
        }
    }

    fn tree(&self, win: Option<WindowId>) -> R<AutoTree> {
        match self.host(AutoCmd::Tree { win })? {
            AutoReply::Tree(t) => Ok(t),
            other => Err(RpcError::new("failed", format!("unexpected host reply {other:?}"))),
        }
    }

    fn windows(&self) -> R<Value> {
        match self.host(AutoCmd::Windows)? {
            AutoReply::Windows(ws) => Ok(Value::Array(
                ws.iter()
                    .map(|w| json!({"id": w.id.get(), "key": w.key, "viewport": rect_json(w.viewport), "dpr": w.dpr, "focused": w.focused}))
                    .collect(),
            )),
            other => Err(RpcError::new("failed", format!("unexpected host reply {other:?}"))),
        }
    }

    fn dump_tree(&self, p: &Value) -> R<Value> {
        let t = self.tree(window_param(p)?)?;
        let nodes: Vec<Value> = t.a11y.nodes().iter().map(node_json).collect();
        let mut out = json!({
            "window": t.window.get(),
            "focused": t.focused.as_ref().map(|w| w.as_str()),
            "hovered": t.hovered.as_ref().map(|w| w.as_str()),
            "nodes": nodes,
        });
        if p.get("widgets").and_then(Value::as_bool).unwrap_or(false) {
            out["widgets"] = Value::Array(
                t.widgets
                    .iter()
                    .map(|w| {
                        json!({"id": w.id.as_str(), "kind": format!("{:?}", w.kind), "rect": rect_json(w.rect), "parent": w.parent.as_ref().map(|p| p.as_str())})
                    })
                    .collect(),
            );
        }
        Ok(out)
    }

    /// Where a pointer command lands: a selector's centre or explicit x/y.
    fn target(&self, p: &Value) -> R<(Option<WindowId>, Option<String>, Point)> {
        let win = window_param(p)?;
        if let Some(sel) = p.get("selector").and_then(Value::as_str) {
            let t = self.tree(win)?;
            let (id, r) = resolve(&t, sel)?;
            return Ok((Some(t.window), Some(id), Point::new(r.x + r.width / 2.0, r.y + r.height / 2.0)));
        }
        match (p.get("x").and_then(Value::as_f64), p.get("y").and_then(Value::as_f64)) {
            (Some(x), Some(y)) => Ok((win, None, Point::new(x, y))),
            _ => Err(RpcError::bad("give \"selector\", or \"x\" and \"y\"")),
        }
    }

    fn hover(&self, p: &Value) -> R<Value> {
        let (win, id, pos) = self.target(p)?;
        self.input(win, vec![InputEvent::Pointer(PointerInput::Moved { pos, mods: ModifierKeys::default() })])?;
        self.input(win, vec![])?;
        Ok(json!({"id": id, "x": pos.x, "y": pos.y}))
    }

    fn click(&self, p: &Value) -> R<Value> {
        let (win, id, pos) = self.target(p)?;
        let button = match p.get("button").and_then(Value::as_str).unwrap_or("left") {
            "left" => MouseButton::Left,
            "right" => MouseButton::Right,
            "middle" => MouseButton::Middle,
            other => return Err(RpcError::bad(format!("unknown button '{other}'"))),
        };
        let count = p.get("count").and_then(Value::as_u64).unwrap_or(1).clamp(1, 3);
        let mods = ModifierKeys::default();
        self.input(win, vec![InputEvent::Pointer(PointerInput::Moved { pos, mods })])?;
        for _ in 0..count {
            self.input(win, vec![InputEvent::Pointer(PointerInput::Down { pos, button, mods })])?;
            self.input(win, vec![InputEvent::Pointer(PointerInput::Up { pos, button, mods })])?;
        }
        // The app reads clicks on its next compose; one more turn lets it act.
        self.input(win, vec![])?;
        Ok(json!({"id": id, "x": pos.x, "y": pos.y}))
    }

    fn focus(&self, p: &Value) -> R<Value> {
        if p.get("selector").and_then(Value::as_str).is_none() {
            return Err(RpcError::bad("give \"selector\""));
        }
        let (win, id, _) = self.target(p)?;
        self.click(p)?;
        let t = self.tree(win)?;
        let focused = t.focused.as_ref().map(|f| f.as_str()) == id.as_deref();
        Ok(json!({"id": id, "focused": focused}))
    }

    fn type_text(&self, p: &Value) -> R<Value> {
        let text = p.get("text").and_then(Value::as_str).ok_or_else(|| RpcError::bad("give \"text\""))?;
        let win = if p.get("selector").is_some() {
            let (win, _, _) = self.target(p)?;
            self.click(p)?;
            win
        } else {
            window_param(p)?
        };
        let mut events = Vec::new();
        if p.get("ime").and_then(Value::as_bool).unwrap_or(false) {
            events.push(InputEvent::Ime(ImeInput::Commit(text.to_string())));
        } else {
            for c in text.chars() {
                let code = keys::code_for_char(c);
                let t = (!matches!(c, '\n' | '\r' | '\t')).then(|| c.to_string());
                events.push(InputEvent::Key(KeyInput { code, text: t, state: KeyState::Down, mods: ModifierKeys::default() }));
                events.push(InputEvent::Key(KeyInput { code, text: None, state: KeyState::Up, mods: ModifierKeys::default() }));
            }
        }
        self.input(win, events)?;
        self.input(win, vec![])?;
        Ok(json!({"chars": text.chars().count()}))
    }

    fn key(&self, p: &Value) -> R<Value> {
        let spec = p.get("key").and_then(Value::as_str).ok_or_else(|| RpcError::bad("give \"key\", e.g. \"Enter\" or \"Ctrl+A\""))?;
        let chord = keys::parse(spec).map_err(RpcError::bad)?;
        let win = window_param(p)?;
        let repeat = p.get("repeat").and_then(Value::as_u64).unwrap_or(1).clamp(1, 100);
        let mut events = Vec::new();
        for _ in 0..repeat {
            events.push(InputEvent::Key(KeyInput { code: chord.code, text: chord.text.clone(), state: KeyState::Down, mods: chord.mods }));
            events.push(InputEvent::Key(KeyInput { code: chord.code, text: None, state: KeyState::Up, mods: chord.mods }));
        }
        self.input(win, events)?;
        self.input(win, vec![])?;
        Ok(json!({"key": spec}))
    }

    fn scroll(&self, p: &Value) -> R<Value> {
        let (win, id, pos) = self.target(p)?;
        let dx = p.get("dx").and_then(Value::as_f64).unwrap_or(0.0);
        let dy = p.get("dy").and_then(Value::as_f64).unwrap_or(0.0);
        let delta = match p.get("unit").and_then(Value::as_str).unwrap_or("px") {
            "px" | "pixels" => WheelDelta::Pixels { x: dx, y: dy },
            "lines" => WheelDelta::Lines { x: dx, y: dy },
            other => return Err(RpcError::bad(format!("unknown unit '{other}' (px or lines)"))),
        };
        let mods = ModifierKeys::default();
        self.input(win, vec![InputEvent::Pointer(PointerInput::Moved { pos, mods })])?;
        self.input(win, vec![InputEvent::Wheel(WheelInput { pos, delta, mods })])?;
        self.input(win, vec![])?;
        Ok(json!({"id": id, "x": pos.x, "y": pos.y}))
    }

    fn screenshot(&self, p: &Value) -> R<Value> {
        let source = match p.get("source").and_then(Value::as_str).unwrap_or("auto") {
            "auto" => ShotSource::Auto,
            "window" => ShotSource::Window,
            "offscreen" => ShotSource::Offscreen,
            other => return Err(RpcError::bad(format!("unknown source '{other}' (auto, window, offscreen)"))),
        };
        match self.host(AutoCmd::Screenshot { win: window_param(p)?, source })? {
            AutoReply::Shot(s) => Ok(json!({
                "window": s.window.get(),
                "width": s.width,
                "height": s.height,
                "source": s.source,
                "png_base64": base64::engine::general_purpose::STANDARD.encode(&s.png),
            })),
            other => Err(RpcError::new("failed", format!("unexpected host reply {other:?}"))),
        }
    }

    fn state(&self) -> R<Value> {
        match self.host(AutoCmd::AppState)? {
            AutoReply::State(None) => Ok(Value::Null),
            AutoReply::State(Some(s)) => serde_json::from_str(&s).map_err(|e| RpcError::new("failed", format!("app state is not JSON: {e}"))),
            other => Err(RpcError::new("failed", format!("unexpected host reply {other:?}"))),
        }
    }
}

fn window_param(p: &Value) -> R<Option<WindowId>> {
    match p.get("window") {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .map(|n| Some(WindowId(n)))
            .ok_or_else(|| RpcError::bad("\"window\" must be a window id number")),
    }
}

/// A selector to (id, rect): the accessibility tree first; a bare `#id`
/// also finds widgets that have no accessibility role.
fn resolve(t: &AutoTree, sel: &str) -> R<(String, Rect)> {
    let parsed = Selector::parse(sel)?;
    match parsed.find_one(&t.a11y) {
        Ok(n) => Ok((n.id.as_str().to_string(), n.bounds)),
        Err(QueryError::NotFound) => {
            let bare = sel.strip_prefix('#').filter(|s| !s.contains(['[', ':', ' ']));
            match bare.and_then(|id| t.widgets.iter().rev().find(|w| w.id.as_str() == id)) {
                Some(w) => Ok((w.id.as_str().to_string(), w.rect)),
                None => Err(QueryError::NotFound.into()),
            }
        }
        Err(e) => Err(e.into()),
    }
}

fn rect_json(r: Rect) -> Value {
    json!({"x": r.x, "y": r.y, "w": r.width, "h": r.height})
}

fn node_json(n: &A11yNode) -> Value {
    let mut v = json!({
        "id": n.id.as_str(),
        "role": role_name(n.role),
        "name": n.name,
        "bounds": rect_json(n.bounds),
        "focused": n.focused,
        "disabled": n.disabled,
        "masked": n.masked,
    });
    if let Some(k) = n.kind {
        v["kind"] = json!(format!("{k:?}"));
    }
    if let Some(val) = &n.value {
        v["value"] = json!(val);
    }
    if let Some(c) = n.checked {
        v["checked"] = json!(c);
    }
    if !n.children.is_empty() {
        v["children"] = Value::Array(n.children.iter().map(|c| json!(c.as_str())).collect());
    }
    v
}
