//! The transport-agnostic RPC core.
//!
//! Request: `{"id": <any>, "method": "<name>", "params": {...}}`.
//! Response: `{"id": <same>, "ok": true, "result": ...}` or
//! `{"id": <same>, "ok": false, "error": {"code": "...", "message": "..."}}`.
//! Codes and messages are plain English and stable; they are not
//! translated.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine as _;
use serde_json::{json, Map, Value};
use uzor::a11y::{query::role_name, A11yNode, QueryError, Selector};
use uzor::input::{ModifierKeys, MouseButton};
use uzor::types::Rect;
use crate::automation::{now_ms, AutoCmd, AutoError, AutoReply, AutoTree, AutomationClient, Delay, ShotSource};
use crate::types::ids::WindowId;
use crate::{ImeInput, InputEvent, KeyInput, KeyState, Point, PointerInput, WheelDelta, WheelInput};

use super::events::EventLog;
use super::keys;
use super::recorder::{self, Recorder, MASKED};

/// Protocol revision reported by `ping` and the discovery file.
pub const PROTOCOL_VERSION: u32 = 1;

/// The methods this core answers.
pub const METHODS: &[&str] = &[
    "ping", "windows", "dump-tree", "click", "hover", "focus", "type", "key", "scroll", "screenshot", "state",
    "wait-for", "settle", "record", "replay",
];

/// Methods a replay may run.
const REPLAYABLE: &[&str] = &["click", "hover", "focus", "type", "key", "scroll", "wait-for", "settle"];

/// How long one host round trip may take.
const CALL_TIMEOUT: Duration = Duration::from_secs(10);
/// How long `focus` / `type` wait for the clicked widget to take the focus.
const FOCUS_WAIT: Duration = Duration::from_secs(2);
/// Default `timeout_ms` of `wait-for` and `settle`.
const WAIT_DEFAULT_MS: f64 = 5000.0;
/// Longest `timeout_ms` accepted.
const WAIT_MAX_MS: f64 = 120_000.0;
/// `wait-for` re-checks at least this often even when nothing is painted
/// (a condition on the app's state report can turn true without a frame).
const RECHECK_MS: f64 = 250.0;
/// How long a replayed pointer step waits for its selector to appear.
const REPLAY_FIND_MS: f64 = 5000.0;

type Fut<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

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

/// The RPC core: decodes requests, drives the app through its automation
/// client, encodes responses, and records an event per call.
///
/// It is async inside so the same core runs where nothing may block (the
/// web bridge resolves it once per animation frame); natively
/// [`Dispatcher::handle_json`] blocks on it.
pub struct Dispatcher {
    app: String,
    client: AutomationClient,
    events: Arc<EventLog>,
    rec: Arc<Recorder>,
}

impl Dispatcher {
    /// A dispatcher for `client`; `app` names the app in `ping`.
    pub fn new(app: &str, client: AutomationClient) -> Self {
        let events = Arc::new(EventLog::new());
        Self { app: app.to_string(), client, rec: Arc::new(Recorder::new(Arc::clone(&events))), events }
    }

    /// One JSON request in, one JSON response out. Never panics on input.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn handle_json(&self, request: &str) -> String {
        crate::automation::block_on(self.handle_json_async(request))
    }

    /// [`Dispatcher::handle_json`] as a future (the web bridge awaits it).
    pub async fn handle_json_async(&self, request: &str) -> String {
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
                            let out = self.call(m, &params).await;
                            (id, out, m.to_string())
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
    pub fn emit(&self, event: Value) {
        self.events.emit(event);
    }

    /// Events with `seq > since`, waiting up to `wait` for the first one
    /// (never waits on wasm).
    pub fn events_since(&self, since: u64, wait: Duration) -> Vec<Value> {
        self.events.since(since, wait)
    }

    fn call<'a>(&'a self, method: &'a str, p: &'a Value) -> Fut<'a, R<Value>> {
        Box::pin(async move {
            match method {
                "ping" => Ok(json!({
                    "pong": true,
                    "app": self.app,
                    "pid": pid(),
                    "protocol": PROTOCOL_VERSION,
                    "methods": METHODS,
                })),
                "windows" => self.windows().await,
                "dump-tree" => self.dump_tree(p).await,
                "click" => self.click(p).await,
                "hover" => self.hover(p).await,
                "focus" => self.focus(p).await,
                "type" => self.type_text(p).await,
                "key" => self.key(p).await,
                "scroll" => self.scroll(p).await,
                "screenshot" => self.screenshot(p).await,
                "state" => self.state().await,
                "wait-for" => self.wait_for(p).await,
                "settle" => self.settle(p).await,
                "record" => self.record(p),
                "replay" => self.replay(p).await,
                other => Err(RpcError::new("unknown_method", format!("unknown method '{other}'; try one of: {}", METHODS.join(", ")))),
            }
        })
    }

    async fn host(&self, cmd: AutoCmd) -> R<AutoReply> {
        Ok(self.client.call_async(cmd, CALL_TIMEOUT).await?)
    }

    /// The host's painted-frame counter.
    async fn frames(&self) -> R<u64> {
        match self.host(AutoCmd::Frames).await? {
            AutoReply::Frame { frame } => Ok(frame),
            other => Err(unexpected(other)),
        }
    }

    /// Wait up to `max_ms` for a frame painted after `after`: its number,
    /// or `None` when none was painted in time.
    async fn next_frame(&self, after: u64, max_ms: f64) -> R<Option<u64>> {
        let wait = Duration::from_secs_f64(max_ms.max(1.0) / 1000.0);
        match self.client.call_async(AutoCmd::Frame { after }, wait).await {
            Ok(AutoReply::Frame { frame }) => Ok(Some(frame)),
            Ok(other) => Err(unexpected(other)),
            Err(AutoError::Timeout) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn input(&self, win: Option<WindowId>, events: Vec<InputEvent>) -> R<()> {
        match self.host(AutoCmd::Input { win, events }).await? {
            AutoReply::InputDone { .. } => Ok(()),
            other => Err(unexpected(other)),
        }
    }

    async fn tree(&self, win: Option<WindowId>) -> R<AutoTree> {
        match self.host(AutoCmd::Tree { win }).await? {
            AutoReply::Tree(t) => Ok(t),
            other => Err(unexpected(other)),
        }
    }

    async fn windows(&self) -> R<Value> {
        match self.host(AutoCmd::Windows).await? {
            AutoReply::Windows(ws) => Ok(Value::Array(
                ws.iter()
                    .map(|w| json!({"id": w.id.get(), "key": w.key, "viewport": rect_json(w.viewport), "dpr": w.dpr, "focused": w.focused}))
                    .collect(),
            )),
            other => Err(unexpected(other)),
        }
    }

    async fn dump_tree(&self, p: &Value) -> R<Value> {
        let t = self.tree(window_param(p)?).await?;
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
    async fn target(&self, p: &Value) -> R<(Option<WindowId>, Option<String>, Point)> {
        let win = window_param(p)?;
        if let Some(sel) = p.get("selector").and_then(Value::as_str) {
            let t = self.tree(win).await?;
            let (id, r) = resolve(&t, sel)?;
            return Ok((Some(t.window), Some(id), Point::new(r.x + r.width / 2.0, r.y + r.height / 2.0)));
        }
        match (p.get("x").and_then(Value::as_f64), p.get("y").and_then(Value::as_f64)) {
            (Some(x), Some(y)) => Ok((win, None, Point::new(x, y))),
            _ => Err(RpcError::bad("give \"selector\", or \"x\" and \"y\"")),
        }
    }

    async fn hover(&self, p: &Value) -> R<Value> {
        let (win, id, pos) = self.target(p).await?;
        self.input(win, vec![InputEvent::Pointer(PointerInput::Moved { pos, mods: ModifierKeys::default() })]).await?;
        self.input(win, vec![]).await?;
        Ok(json!({"id": id, "x": pos.x, "y": pos.y}))
    }

    async fn click(&self, p: &Value) -> R<Value> {
        let (win, id, pos) = self.target(p).await?;
        let button = match p.get("button").and_then(Value::as_str).unwrap_or("left") {
            "left" => MouseButton::Left,
            "right" => MouseButton::Right,
            "middle" => MouseButton::Middle,
            other => return Err(RpcError::bad(format!("unknown button '{other}'"))),
        };
        let count = p.get("count").and_then(Value::as_u64).unwrap_or(1).clamp(1, 3);
        let mods = ModifierKeys::default();
        self.input(win, vec![InputEvent::Pointer(PointerInput::Moved { pos, mods })]).await?;
        for _ in 0..count {
            self.input(win, vec![InputEvent::Pointer(PointerInput::Down { pos, button, mods })]).await?;
            self.input(win, vec![InputEvent::Pointer(PointerInput::Up { pos, button, mods })]).await?;
        }
        // The app reads clicks on its next compose; one more turn lets it act.
        self.input(win, vec![]).await?;
        Ok(json!({"id": id, "x": pos.x, "y": pos.y}))
    }

    async fn focus(&self, p: &Value) -> R<Value> {
        if p.get("selector").and_then(Value::as_str).is_none() {
            return Err(RpcError::bad("give \"selector\""));
        }
        let (win, id) = self.click_and_focus(p).await?;
        Ok(json!({"id": id, "window": win.map(|w| w.get()), "focused": true}))
    }

    /// Click the selector's widget and wait until it holds the focus. Apps may move the focus
    /// a frame or two after the click (a focus command), so this polls with empty input turns.
    async fn click_and_focus(&self, p: &Value) -> R<(Option<WindowId>, String)> {
        let (win, id, _) = self.target(p).await?;
        let id = id.unwrap_or_default();
        self.click(p).await?;
        let deadline = now_ms() + FOCUS_WAIT.as_secs_f64() * 1000.0;
        loop {
            let t = self.tree(win).await?;
            if t.focused.as_ref().map(|f| f.as_str()) == Some(id.as_str()) {
                return Ok((win, id));
            }
            if now_ms() >= deadline {
                return Err(RpcError::new("not_focused", format!("'{id}' did not take the focus after a click")));
            }
            Delay::ms(16.0).await;
            self.input(win, vec![]).await?;
        }
    }

    async fn type_text(&self, p: &Value) -> R<Value> {
        let text = p.get("text").and_then(Value::as_str).ok_or_else(|| RpcError::bad("give \"text\""))?;
        let win = if p.get("selector").is_some() {
            self.click_and_focus(p).await?.0
        } else {
            window_param(p)?
        };
        let mut events = Vec::new();
        if p.get("clear").and_then(Value::as_bool).unwrap_or(false) {
            // Replace what the field holds: select all, delete, then type.
            for spec in ["Ctrl+A", "Backspace"] {
                let chord = keys::parse(spec).map_err(RpcError::bad)?;
                events.push(InputEvent::Key(KeyInput { code: chord.code, text: chord.text.clone(), state: KeyState::Down, mods: chord.mods }));
                events.push(InputEvent::Key(KeyInput { code: chord.code, text: None, state: KeyState::Up, mods: chord.mods }));
            }
        }
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
        self.input(win, events).await?;
        self.input(win, vec![]).await?;
        Ok(json!({"chars": text.chars().count()}))
    }

    async fn key(&self, p: &Value) -> R<Value> {
        let spec = p.get("key").and_then(Value::as_str).ok_or_else(|| RpcError::bad("give \"key\", e.g. \"Enter\" or \"Ctrl+A\""))?;
        let chord = keys::parse(spec).map_err(RpcError::bad)?;
        let win = window_param(p)?;
        let repeat = p.get("repeat").and_then(Value::as_u64).unwrap_or(1).clamp(1, 100);
        let mut events = Vec::new();
        for _ in 0..repeat {
            events.push(InputEvent::Key(KeyInput { code: chord.code, text: chord.text.clone(), state: KeyState::Down, mods: chord.mods }));
            events.push(InputEvent::Key(KeyInput { code: chord.code, text: None, state: KeyState::Up, mods: chord.mods }));
        }
        self.input(win, events).await?;
        self.input(win, vec![]).await?;
        Ok(json!({"key": spec}))
    }

    async fn scroll(&self, p: &Value) -> R<Value> {
        let (win, id, pos) = self.target(p).await?;
        let dx = p.get("dx").and_then(Value::as_f64).unwrap_or(0.0);
        let dy = p.get("dy").and_then(Value::as_f64).unwrap_or(0.0);
        let delta = match p.get("unit").and_then(Value::as_str).unwrap_or("px") {
            "px" | "pixels" => WheelDelta::Pixels { x: dx, y: dy },
            "lines" => WheelDelta::Lines { x: dx, y: dy },
            other => return Err(RpcError::bad(format!("unknown unit '{other}' (px or lines)"))),
        };
        let mods = ModifierKeys::default();
        self.input(win, vec![InputEvent::Pointer(PointerInput::Moved { pos, mods })]).await?;
        self.input(win, vec![InputEvent::Wheel(WheelInput { pos, delta, mods })]).await?;
        self.input(win, vec![]).await?;
        Ok(json!({"id": id, "x": pos.x, "y": pos.y}))
    }

    async fn screenshot(&self, p: &Value) -> R<Value> {
        let source = match p.get("source").and_then(Value::as_str).unwrap_or("auto") {
            "auto" => ShotSource::Auto,
            "window" => ShotSource::Window,
            "offscreen" => ShotSource::Offscreen,
            other => return Err(RpcError::bad(format!("unknown source '{other}' (auto, window, offscreen)"))),
        };
        match self.host(AutoCmd::Screenshot { win: window_param(p)?, source }).await? {
            AutoReply::Shot(s) => Ok(json!({
                "window": s.window.get(),
                "width": s.width,
                "height": s.height,
                "source": s.source,
                "png_base64": base64::engine::general_purpose::STANDARD.encode(&s.png),
            })),
            other => Err(unexpected(other)),
        }
    }

    async fn state(&self) -> R<Value> {
        match self.host(AutoCmd::AppState).await? {
            AutoReply::State(None) => Ok(Value::Null),
            AutoReply::State(Some(s)) => serde_json::from_str(&s).map_err(|e| RpcError::new("failed", format!("app state is not JSON: {e}"))),
            other => Err(unexpected(other)),
        }
    }

    /// One check of a `wait-for` condition: `Some(detail)` when met.
    async fn check(&self, p: &Value, win: Option<WindowId>) -> R<Option<Value>> {
        if let Some(sel) = p.get("target").and_then(Value::as_str) {
            Selector::parse(sel)?;
            let t = self.tree(win).await?;
            let found = match resolve(&t, sel) {
                Ok((id, _)) => Some(id),
                Err(e) if e.code == "ambiguous" => Some(String::new()),
                Err(e) if e.code == "not_found" => None,
                Err(e) => return Err(e),
            };
            let node = |id: &str| t.a11y.nodes().iter().find(|n| n.id.as_str() == id).cloned();
            let met = match p.get("state").and_then(Value::as_str).unwrap_or("present") {
                "present" => found.is_some(),
                "gone" => found.is_none(),
                "focused" => found.as_deref().is_some_and(|id| !id.is_empty() && t.focused.as_ref().map(|f| f.as_str()) == Some(id)),
                "enabled" => found.as_deref().and_then(node).is_some_and(|n| !n.disabled),
                other => return Err(RpcError::bad(format!("unknown state '{other}' (present, gone, focused, enabled)"))),
            };
            let value_ok = match p.get("value").and_then(Value::as_str) {
                None => true,
                Some(want) => found.as_deref().and_then(node).is_some_and(|n| n.value.as_deref() == Some(want)),
            };
            return Ok((met && value_ok).then(|| json!({"id": found.filter(|s| !s.is_empty())})));
        }
        if let Some(cond) = p.get("cond") {
            if !cond.is_object() {
                return Err(RpcError::bad("\"cond\" must be an object matched against the app's state report"));
            }
            let state = self.state().await?;
            return Ok(json_subset(cond, &state).then(|| json!({})));
        }
        Err(RpcError::bad("give \"target\" (a selector) or \"cond\" (an object the state report must contain)"))
    }

    /// Wait until a selector matches (or `state`: gone / focused / enabled,
    /// optional `value`) or the state report contains `cond`; checked once
    /// per painted frame (and every 250 ms while nothing paints).
    async fn wait_for(&self, p: &Value) -> R<Value> {
        let win = window_param(p)?;
        let limit = timeout_ms(p)?;
        let t0 = now_ms();
        let mut checks = 0u64;
        loop {
            let seen = self.frames().await?;
            checks += 1;
            if let Some(mut detail) = self.check(p, win).await? {
                detail["checks"] = json!(checks);
                detail["frame"] = json!(seen);
                detail["waited_ms"] = json!((now_ms() - t0).round() as u64);
                return Ok(detail);
            }
            let left = t0 + limit - now_ms();
            if left <= 0.0 {
                return Err(RpcError::new("timeout", format!("condition not met in {} ms ({checks} checks)", limit as u64)));
            }
            self.next_frame(seen, left.min(RECHECK_MS)).await?;
        }
    }

    /// What the UI shows, as one number: the accessibility tree (names,
    /// values, bounds, states), every widget rect, focus, hover and the
    /// app's state report. Pixels-only animation does not move it.
    async fn signature(&self, win: Option<WindowId>) -> R<u64> {
        use std::hash::{Hash, Hasher};
        let t = self.tree(win).await?;
        let state = match self.host(AutoCmd::AppState).await? {
            AutoReply::State(s) => s,
            other => return Err(unexpected(other)),
        };
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for n in t.a11y.nodes() {
            node_json(n).to_string().hash(&mut h);
        }
        for w in &t.widgets {
            (w.id.as_str(), w.rect.x.to_bits(), w.rect.y.to_bits(), w.rect.width.to_bits(), w.rect.height.to_bits()).hash(&mut h);
        }
        (t.focused.as_ref().map(|f| f.as_str().to_string()), t.hovered.as_ref().map(|f| f.as_str().to_string()), state).hash(&mut h);
        Ok(h.finish())
    }

    /// Wait until the UI has not changed for `idle_ms` (default 300):
    /// the [signature](Self::signature) is compared once per painted frame
    /// (and every 250 ms while nothing paints).
    async fn settle(&self, p: &Value) -> R<Value> {
        let idle = match p.get("idle_ms") {
            None | Some(Value::Null) => 300.0,
            Some(v) => v.as_f64().filter(|t| *t >= 0.0 && *t <= WAIT_MAX_MS).ok_or_else(|| RpcError::bad("\"idle_ms\" must be a number of milliseconds"))?,
        };
        let win = window_param(p)?;
        let limit = timeout_ms(p)?.max(idle);
        let t0 = now_ms();
        let first = self.frames().await?;
        let mut seen = first;
        let mut sig = self.signature(win).await?;
        let mut quiet_since = now_ms();
        let (mut checks, mut changes) = (1u64, 0u64);
        loop {
            let now = now_ms();
            let quiet_left = quiet_since + idle - now;
            if quiet_left <= 0.0 {
                return Ok(json!({
                    "frame": seen,
                    "frames": seen - first,
                    "checks": checks,
                    "changes": changes,
                    "waited_ms": (now - t0).round() as u64,
                }));
            }
            let left = t0 + limit - now;
            if left <= 0.0 {
                return Err(RpcError::new(
                    "timeout",
                    format!("the UI kept changing for {} ms ({changes} changes over {} frames)", limit as u64, seen - first),
                ));
            }
            if let Some(f) = self.next_frame(seen, quiet_left.min(left).min(RECHECK_MS)).await? {
                seen = f;
            }
            checks += 1;
            let now_sig = self.signature(win).await?;
            if now_sig != sig {
                sig = now_sig;
                changes += 1;
                quiet_since = now_ms();
            }
        }
    }

    /// `record {"on": true}` starts a fresh recording of every input the app
    /// gets; `{"on": false}` stops and returns it as JSONL; no params: status.
    fn record(&self, p: &Value) -> R<Value> {
        let on = match p.get("on") {
            None | Some(Value::Null) => None,
            Some(Value::Bool(b)) => Some(*b),
            Some(Value::String(s)) if s == "on" => Some(true),
            Some(Value::String(s)) if s == "off" => Some(false),
            Some(_) => return Err(RpcError::bad("\"on\" must be true or false")),
        };
        match on {
            None => {
                let (recording, n) = self.rec.status();
                Ok(json!({"recording": recording, "events": n}))
            }
            Some(true) => {
                self.rec.start();
                let rec = Arc::clone(&self.rec);
                self.client.set_tap(Some(Box::new(move |ev| rec.on_tap(ev))));
                self.emit(json!({"type": "record-state", "recording": true}));
                Ok(json!({"recording": true}))
            }
            Some(false) => {
                self.client.set_tap(None);
                let lines = self.rec.stop();
                self.emit(json!({"type": "record-state", "recording": false, "events": lines.len()}));
                let jsonl: String = lines.iter().map(|l| format!("{l}\n")).collect();
                Ok(json!({"recording": false, "events": lines.len(), "jsonl": jsonl}))
            }
        }
    }

    /// Replay recorded lines (`events`: JSONL text or an array) at `speed`
    /// (1 = as recorded, 2 = twice as fast, 0 = no pauses). Masked values
    /// come from `secrets`: an object by field id, or a list in order.
    async fn replay(&self, p: &Value) -> R<Value> {
        let lines = replay_lines(p.get("events").ok_or_else(|| RpcError::bad("give \"events\" (JSONL text or an array)"))?)?;
        let speed = match p.get("speed") {
            None | Some(Value::Null) => 1.0,
            Some(v) => v.as_f64().filter(|s| *s >= 0.0 && s.is_finite()).ok_or_else(|| RpcError::bad("\"speed\" must be a number >= 0"))?,
        };
        // Every masked value must have a secret before anything runs.
        let secrets = p.get("secrets");
        let mut list_i = 0usize;
        let mut filled: Vec<Value> = Vec::with_capacity(lines.len());
        let mut missing: Vec<String> = Vec::new();
        let mut masked_n = 0usize;
        for (_, method, params) in &lines {
            let mut params = params.clone();
            let masked = *method == "type" && (params.get("masked").and_then(Value::as_bool) == Some(true) || params.get("text").and_then(Value::as_str) == Some(MASKED));
            if masked {
                masked_n += 1;
                let into = params.get("into").and_then(Value::as_str).unwrap_or("").to_string();
                let bare = into.strip_prefix('#').unwrap_or(&into).to_string();
                let secret = match secrets {
                    Some(Value::Object(m)) => m.get(&bare).or_else(|| m.get(&format!("#{bare}"))).and_then(Value::as_str).map(str::to_string),
                    Some(Value::Array(a)) => {
                        let s = a.get(list_i).and_then(Value::as_str).map(str::to_string);
                        list_i += 1;
                        s
                    }
                    _ => None,
                };
                match secret {
                    Some(s) => {
                        params["text"] = json!(s);
                    }
                    None => missing.push(if into.is_empty() { format!("#{masked_n}") } else { into }),
                }
                if let Some(o) = params.as_object_mut() {
                    o.remove("masked");
                }
            }
            if let Some(o) = params.as_object_mut() {
                o.remove("into");
            }
            filled.push(params);
        }
        if masked_n > 0 && secrets.is_none() {
            return Err(RpcError::bad(format!("replay needs \"secrets\": {masked_n} masked value(s) (an object by field id, or a list in order)")));
        }
        if !missing.is_empty() {
            return Err(RpcError::bad(format!("\"secrets\" has no value for: {}", missing.join(", "))));
        }
        let t0 = now_ms();
        let first_t = lines.first().map_or(0.0, |l| l.0);
        for (i, ((t, method, _), params)) in lines.iter().zip(filled.iter()).enumerate() {
            if speed > 0.0 {
                let at = t0 + (t - first_t) / speed;
                let wait = at - now_ms();
                if wait > 0.0 {
                    Delay::ms(wait).await;
                }
            }
            if let Some(sel) = params.get("selector").and_then(Value::as_str) {
                let w = json!({"target": sel, "timeout_ms": REPLAY_FIND_MS, "window": params.get("window").cloned().unwrap_or(Value::Null)});
                if let Err(e) = self.wait_for(&w).await {
                    self.emit(json!({"type": "replay", "index": i, "method": method, "ok": false, "error": e.code}));
                    return Err(RpcError::new(e.code, format!("event {i} ({method}): {sel} did not appear: {}", e.message)));
                }
            }
            let out = self.call(method, params).await;
            let mut ev = json!({"type": "replay", "index": i, "method": method, "ok": out.is_ok()});
            if let Err(e) = &out {
                ev["error"] = json!(e.code);
            }
            self.emit(ev);
            if let Err(e) = out {
                return Err(RpcError::new(e.code, format!("event {i} ({method}): {}", e.message)));
            }
        }
        Ok(json!({"replayed": lines.len(), "speed": speed, "ms": (now_ms() - t0).round() as u64}))
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

fn resolve(t: &AutoTree, sel: &str) -> R<(String, Rect)> {
    Ok(recorder::resolve(t, sel)?)
}

fn unexpected(other: AutoReply) -> RpcError {
    RpcError::new("failed", format!("unexpected host reply {other:?}"))
}

fn pid() -> Value {
    #[cfg(target_arch = "wasm32")]
    {
        Value::Null
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        json!(std::process::id())
    }
}

/// `timeout_ms` (default 5 s, at most 120 s).
fn timeout_ms(p: &Value) -> R<f64> {
    match p.get("timeout_ms") {
        None | Some(Value::Null) => Ok(WAIT_DEFAULT_MS),
        Some(v) => v
            .as_f64()
            .filter(|t| *t >= 0.0 && *t <= WAIT_MAX_MS)
            .ok_or_else(|| RpcError::bad(format!("\"timeout_ms\" must be 0..{WAIT_MAX_MS}"))),
    }
}

/// `want` is a subset of `have`: objects by key (recursively), the rest by equality.
fn json_subset(want: &Value, have: &Value) -> bool {
    match (want, have) {
        (Value::Object(w), Value::Object(h)) => w.iter().all(|(k, wv)| h.get(k).is_some_and(|hv| json_subset(wv, hv))),
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        _ => want == have,
    }
}

/// Parse replay input into (t_ms, method, params), checking each method.
fn replay_lines(events: &Value) -> R<Vec<(f64, &'static str, Value)>> {
    let raw: Vec<Value> = match events {
        Value::String(text) => text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .enumerate()
            .map(|(i, l)| serde_json::from_str(l).map_err(|e| RpcError::bad(format!("events line {}: {e}", i + 1))))
            .collect::<R<_>>()?,
        Value::Array(a) => a.clone(),
        _ => return Err(RpcError::bad("\"events\" must be JSONL text or an array")),
    };
    let mut out = Vec::with_capacity(raw.len());
    let mut last_t = 0.0f64;
    for (i, ev) in raw.iter().enumerate() {
        let m = ev.get("method").and_then(Value::as_str).ok_or_else(|| RpcError::bad(format!("event {i}: missing \"method\"")))?;
        let method = *REPLAYABLE
            .iter()
            .find(|r| **r == m)
            .ok_or_else(|| RpcError::bad(format!("event {i}: '{m}' cannot be replayed (only {})", REPLAYABLE.join(", "))))?;
        let params = ev.get("params").cloned().unwrap_or(Value::Object(Map::new()));
        if !params.is_object() {
            return Err(RpcError::bad(format!("event {i}: \"params\" must be an object")));
        }
        let t = ev.get("t_ms").and_then(Value::as_f64).unwrap_or(last_t).max(last_t);
        last_t = t;
        out.push((t, method, params));
    }
    Ok(out)
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

#[cfg(not(target_arch = "wasm32"))]
impl uzor_devtools::Rpc for Dispatcher {
    fn handle_json(&self, request: &str) -> String {
        Dispatcher::handle_json(self, request)
    }
    fn events_since(&self, since: u64, wait: Duration) -> Vec<Value> {
        Dispatcher::events_since(self, since, wait)
    }
}
