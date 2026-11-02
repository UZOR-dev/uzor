//! `record`: turns the input a host pushes (the user's or injected) into
//! replayable RPC-level lines — `click`, `type`, `key`, `scroll` — with
//! selectors in preference to coordinates, and every value typed into a
//! masked field written as `"<masked>"`.
//!
//! Line format (one JSON object per line, JSONL):
//! `{"t_ms": 120, "method": "click", "params": {"selector": "#go"}}`.
//! `t_ms` counts from `record on`. Each line also goes to the event log as
//! `{"type": "record", "line": <index>, "event": <line>}`; a line that grows
//! (consecutive typing into one field, consecutive wheel turns) is emitted
//! again with the same index.

use std::sync::{Arc, Mutex};

use serde_json::{json, Map, Value};
use uzor::a11y::{QueryError, Selector};
use uzor::input::MouseButton;
use uzor::types::Rect;

use super::events::EventLog;
use super::keys;
use crate::automation::{now_ms, AutoTree, TapEvent};
use crate::types::bus::{ImeInput, InputEvent, KeyState, PointerInput, WheelDelta};
use crate::types::ids::WindowId;
use crate::Point;

/// What a masked value is recorded as.
pub const MASKED: &str = "<masked>";

/// Typing or wheel turns closer than this extend the previous line.
const MERGE_MS: f64 = 1500.0;
/// A press and release further apart than this are a drag (not recorded).
const CLICK_SLOP: f64 = 6.0;

#[derive(Default)]
struct State {
    on: bool,
    t0: f64,
    lines: Vec<Value>,
    /// Last line's kind and when it was last extended.
    tail: Option<(Tail, f64)>,
    down: Option<(WindowId, Point, MouseButton, Value)>,
}

#[derive(Clone, PartialEq)]
enum Tail {
    Type { into: Option<String>, masked: bool },
    Scroll { target: Value, unit: &'static str },
    Other,
}

/// The recorder; the host's input tap feeds it.
pub(crate) struct Recorder {
    st: Mutex<State>,
    events: Arc<EventLog>,
}

impl Recorder {
    pub fn new(events: Arc<EventLog>) -> Self {
        Self { st: Mutex::new(State::default()), events }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.st.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Start a fresh recording.
    pub fn start(&self) {
        let mut st = self.lock();
        *st = State { on: true, t0: now_ms(), ..State::default() };
    }

    /// Stop; the lines recorded.
    pub fn stop(&self) -> Vec<Value> {
        let mut st = self.lock();
        st.on = false;
        st.down = None;
        st.tail = None;
        st.lines.clone()
    }

    /// Recording, and lines so far.
    pub fn status(&self) -> (bool, usize) {
        let st = self.lock();
        (st.on, st.lines.len())
    }

    /// One tapped input event.
    pub fn on_tap(&self, ev: TapEvent) {
        let Some(tree) = ev.tree.as_ref() else { return };
        let mut st = self.lock();
        if !st.on {
            return;
        }
        match &ev.event {
            InputEvent::Pointer(PointerInput::Down { pos, button, .. }) => {
                st.down = Some((ev.window, *pos, *button, target_at(tree, *pos)));
            }
            InputEvent::Pointer(PointerInput::Up { pos, button, .. }) => {
                if let Some((w, p0, b0, target)) = st.down.take() {
                    let near = (pos.x - p0.x).hypot(pos.y - p0.y) <= CLICK_SLOP;
                    if w == ev.window && b0 == *button && near {
                        let mut params = target;
                        match button {
                            MouseButton::Left => {}
                            MouseButton::Right => params["button"] = json!("right"),
                            MouseButton::Middle => params["button"] = json!("middle"),
                        }
                        self.push(&mut st, "click", params, Tail::Other);
                    }
                }
            }
            InputEvent::Wheel(w) => {
                let (dx, dy, unit) = match w.delta {
                    WheelDelta::Pixels { x, y } => (x, y, "px"),
                    WheelDelta::Lines { x, y } => (x, y, "lines"),
                };
                let target = target_at(tree, w.pos);
                let tail = Tail::Scroll { target: target.clone(), unit };
                if let Some(line) = self.extend(&mut st, &tail) {
                    let p = &mut line["params"];
                    p["dx"] = json!(p["dx"].as_f64().unwrap_or(0.0) + dx);
                    p["dy"] = json!(p["dy"].as_f64().unwrap_or(0.0) + dy);
                    self.reemit(&st);
                } else {
                    let mut params = target;
                    params["dx"] = json!(dx);
                    params["dy"] = json!(dy);
                    if unit != "px" {
                        params["unit"] = json!(unit);
                    }
                    self.push(&mut st, "scroll", params, tail);
                }
            }
            InputEvent::Key(k) if k.state == KeyState::Down => {
                let shortcut = k.mods.ctrl || k.mods.alt || k.mods.meta;
                let text = k.text.as_deref().filter(|t| !shortcut && !t.is_empty() && !t.chars().any(char::is_control));
                match text {
                    Some(t) => self.typed(&mut st, tree, t, false),
                    None => {
                        if let Some(name) = keys::chord_name(k.code, k.mods) {
                            self.push(&mut st, "key", json!({"key": name}), Tail::Other);
                        }
                    }
                }
            }
            InputEvent::Ime(ImeInput::Commit(t)) if !t.is_empty() => self.typed(&mut st, tree, t, true),
            _ => {}
        }
    }

    fn typed(&self, st: &mut State, tree: &AutoTree, text: &str, ime: bool) {
        let into = tree.focused.as_ref().map(|f| f.as_str().to_string());
        let masked = into
            .as_deref()
            .is_some_and(|id| tree.a11y.nodes().iter().any(|n| n.id.as_str() == id && n.masked));
        let tail = Tail::Type { into: into.clone(), masked };
        if !ime {
            if let Some(line) = self.extend(st, &tail) {
                if !masked {
                    let p = &mut line["params"];
                    let t = format!("{}{text}", p["text"].as_str().unwrap_or(""));
                    p["text"] = json!(t);
                    self.reemit(st);
                }
                return;
            }
        }
        // A typing run records what the field holds after it, and replays
        // with `clear` so a field that already holds something (a remembered
        // nick, a draft) ends up the same as when it was recorded.
        let prefix = if ime || masked {
            String::new()
        } else {
            into.as_deref()
                .and_then(|id| tree.a11y.nodes().iter().find(|n| n.id.as_str() == id))
                .and_then(|n| n.value.clone())
                .unwrap_or_default()
        };
        let mut params = Map::new();
        params.insert("text".into(), json!(if masked { MASKED.to_string() } else { format!("{prefix}{text}") }));
        if !ime && into.is_some() {
            params.insert("clear".into(), json!(true));
        }
        if masked {
            params.insert("masked".into(), json!(true));
        }
        if ime {
            params.insert("ime".into(), json!(true));
        }
        if let Some(id) = into {
            params.insert("into".into(), json!(format!("#{id}")));
        }
        self.push(st, "type", Value::Object(params), if ime { Tail::Other } else { tail });
    }

    /// The last line, when `tail` continues it recently enough.
    fn extend<'a>(&self, st: &'a mut State, tail: &Tail) -> Option<&'a mut Value> {
        let now = now_ms();
        match &mut st.tail {
            Some((t, at)) if t == tail && now - *at <= MERGE_MS => {
                *at = now;
                st.lines.last_mut()
            }
            _ => None,
        }
    }

    fn push(&self, st: &mut State, method: &str, params: Value, tail: Tail) {
        let t_ms = (now_ms() - st.t0).max(0.0).round() as u64;
        st.lines.push(json!({"t_ms": t_ms, "method": method, "params": params}));
        st.tail = Some((tail, now_ms()));
        self.reemit(st);
    }

    fn reemit(&self, st: &State) {
        if let Some(line) = st.lines.last() {
            self.events.emit(json!({"type": "record", "line": st.lines.len() - 1, "event": line}));
        }
    }
}

/// `{"selector": …}` for the widget under `pos` when a selector finds it
/// again, else `{"x": …, "y": …}`.
pub(crate) fn target_at(t: &AutoTree, pos: Point) -> Value {
    let contains = |r: &Rect| pos.x >= r.x && pos.y >= r.y && pos.x <= r.x + r.width && pos.y <= r.y + r.height;
    let area = |r: &Rect| r.width.max(0.0) * r.height.max(0.0);
    let mut cands: Vec<(f64, String)> = t
        .a11y
        .nodes()
        .iter()
        .filter(|n| !n.id.as_str().is_empty() && contains(&n.bounds))
        .map(|n| (area(&n.bounds), n.id.as_str().to_string()))
        .collect();
    cands.extend(
        t.widgets
            .iter()
            .filter(|w| !w.id.as_str().is_empty() && contains(&w.rect))
            .map(|w| (area(&w.rect), w.id.as_str().to_string())),
    );
    cands.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (_, id) in cands {
        if id.contains(['[', ':', ' ', ']', '#']) {
            continue;
        }
        let sel = format!("#{id}");
        if resolve(t, &sel).is_ok_and(|(_, r)| contains(&r)) {
            return json!({"selector": sel});
        }
    }
    json!({"x": pos.x, "y": pos.y})
}

/// A selector to (id, rect): the accessibility tree first; a bare `#id`
/// also finds widgets that have no accessibility role.
pub(crate) fn resolve(t: &AutoTree, sel: &str) -> Result<(String, Rect), QueryError> {
    let parsed = Selector::parse(sel)?;
    match parsed.find_one(&t.a11y) {
        Ok(n) => Ok((n.id.as_str().to_string(), n.bounds)),
        Err(QueryError::NotFound) => {
            let bare = sel.strip_prefix('#').filter(|s| !s.contains(['[', ':', ' ']));
            match bare.and_then(|id| t.widgets.iter().rev().find(|w| w.id.as_str() == id)) {
                Some(w) => Ok((w.id.as_str().to_string(), w.rect)),
                None => Err(QueryError::NotFound),
            }
        }
        Err(e) => Err(e),
    }
}
