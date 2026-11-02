//! Dev control in the browser (feature `dev-control`): the same RPC as the
//! native loopback server, reached from page JavaScript instead of a socket.
//!
//! ```js
//! // only on a page opened with ?dev=1, and only in a build with the feature
//! const r = await window.__uzorDev.call({method: "dump-tree"});
//! const ev = window.__uzorDev.events(0);   // like GET /events?since=0
//! ```
//!
//! Nothing listens on the network: the bridge is a JS object on `window`,
//! so it is as remote as whatever already drives the page (a CDP
//! `Runtime.evaluate` from a test, the console). Without `?dev=1` in the
//! page URL nothing is installed. Production builds leave the feature off,
//! so they carry none of this code.
//!
//! Calls run as tasks the RAF driver polls every frame; the driver keeps
//! frames coming while any task, reply or frame wait is pending. Jobs are
//! served like the native host does: input goes into the next tick and is
//! answered after its paint, queries are answered after the paint, and
//! screenshots are painted offscreen (tiny-skia) at the page's DPR.

use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Wake};
use std::time::Duration;

use serde_json::Value;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

use crate::automation::{
    answer_plain, channel, resolve_window, tap_input, AutoCmd, AutoError, AutoJob, AutoReply, AutoReplyTx,
    AutomationPort, FrameWaits, ShotSource,
};
use crate::dev_control::Dispatcher;
use crate::runtime::Runtime;
use crate::types::bus::InputEnvelope;
use crate::types::ids::WindowId;
use crate::{App, Spec};

/// Turn the browser dev bridge on for [`run_web_with`](super::run_web_with).
///
/// It is installed only when the page URL carries `dev=1` in its query
/// (`?dev=1`); then `window.__uzorDev` and every alias answer calls.
#[derive(Debug, Clone)]
pub struct WebDevControl {
    /// App name reported by `ping`.
    pub app: String,
    /// More global names for the same object (an app's own `__hqDev`, …).
    pub aliases: Vec<String>,
}

impl WebDevControl {
    /// The bridge for `app`, as `window.__uzorDev`.
    pub fn new(app: &str) -> Self {
        Self { app: app.to_string(), aliases: Vec::new() }
    }

    /// Also expose it as `window.<name>`.
    pub fn alias(mut self, name: &str) -> Self {
        self.aliases.push(name.to_string());
        self
    }
}

/// The page URL asks for dev control (`dev=1` in the query string).
pub fn dev_requested() -> bool {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .is_some_and(|q| query_has_dev(&q))
}

fn query_has_dev(search: &str) -> bool {
    search.trim_start_matches('?').split('&').any(|kv| kv == "dev=1")
}

/// `Send` wrapper for the single-threaded RAF arm (see `raf::Arm`).
struct Local(Rc<dyn Fn()>);
unsafe impl Send for Local {}
unsafe impl Sync for Local {}

impl Local {
    fn call(&self) {
        (self.0)()
    }
}

struct NoopWake;
impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

type Task = Pin<Box<dyn Future<Output = ()>>>;

/// The driver's half: port bookkeeping and the call tasks.
pub(super) struct Bridge {
    port: AutomationPort,
    tasks: Rc<RefCell<Vec<Task>>>,
    after_paint: Vec<(AutoReplyTx, WindowId)>,
    queries: Vec<AutoJob>,
    waits: FrameWaits,
    background: HashMap<WindowId, u32>,
}

/// Install `window.__uzorDev` (+ aliases) when the page asked for it.
/// `wake` schedules a frame.
pub(super) fn install(cfg: &WebDevControl, wake: Rc<dyn Fn()>) -> Option<Bridge> {
    if !dev_requested() {
        return None;
    }
    let window = web_sys::window()?;
    let (client, port) = channel();
    {
        let arm = Local(Rc::clone(&wake));
        port.set_waker(move || arm.call());
    }
    let disp = Rc::new(Dispatcher::new(&cfg.app, client));
    let tasks: Rc<RefCell<Vec<Task>>> = Rc::new(RefCell::new(Vec::new()));

    let call = {
        let disp = Rc::clone(&disp);
        let tasks = Rc::clone(&tasks);
        Closure::wrap(Box::new(move |req: JsValue| -> js_sys::Promise {
            let text = match req.as_string() {
                Some(s) => s,
                None => js_sys::JSON::stringify(&req).map(String::from).unwrap_or_default(),
            };
            let mut slot = None;
            let promise = js_sys::Promise::new(&mut |resolve, _reject| slot = Some(resolve));
            if let Some(resolve) = slot {
                let disp = Rc::clone(&disp);
                tasks.borrow_mut().push(Box::pin(async move {
                    let out = disp.handle_json_async(&text).await;
                    let v = js_sys::JSON::parse(&out).unwrap_or_else(|_| JsValue::from_str(&out));
                    let _ = resolve.call1(&JsValue::NULL, &v);
                }));
            }
            wake();
            promise
        }) as Box<dyn FnMut(JsValue) -> js_sys::Promise>)
    };
    let events = {
        let disp = Rc::clone(&disp);
        Closure::wrap(Box::new(move |since: JsValue| -> JsValue {
            let since = since.as_f64().filter(|s| *s >= 0.0).unwrap_or(0.0) as u64;
            let list = Value::Array(disp.events_since(since, Duration::ZERO)).to_string();
            js_sys::JSON::parse(&list).unwrap_or(JsValue::NULL)
        }) as Box<dyn FnMut(JsValue) -> JsValue>)
    };
    let obj = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&obj, &"call".into(), call.as_ref().unchecked_ref());
    let _ = js_sys::Reflect::set(&obj, &"events".into(), events.as_ref().unchecked_ref());
    let _ = js_sys::Reflect::set(&obj, &"app".into(), &JsValue::from_str(&cfg.app));
    // Page lifetime, like the driver.
    call.forget();
    events.forget();
    for name in std::iter::once("__uzorDev").chain(cfg.aliases.iter().map(String::as_str)) {
        let _ = js_sys::Reflect::set(&window, &JsValue::from_str(name), &obj);
    }
    log::info!("{}: dev control bridge on window.__uzorDev (?dev=1)", cfg.app);
    Some(Bridge {
        port,
        tasks,
        after_paint: Vec::new(),
        queries: Vec::new(),
        waits: FrameWaits::default(),
        background: HashMap::new(),
    })
}

impl Bridge {
    /// Run every call task until it waits.
    pub fn poll(&mut self) {
        let waker = std::task::Waker::from(Arc::new(NoopWake));
        let mut cx = Context::from_waker(&waker);
        let mut list = std::mem::take(&mut *self.tasks.borrow_mut());
        list.retain_mut(|t| t.as_mut().poll(&mut cx) == Poll::Pending);
        // Calls made while polling landed in the shared list.
        let mut shared = self.tasks.borrow_mut();
        list.append(&mut shared);
        *shared = list;
    }

    /// Record input the page delivered (for `record`).
    pub fn tap<S: Spec, A: App<Spec = S>>(&self, rt: &Runtime<S, A>, env: &InputEnvelope) {
        tap_input(&self.port, rt, env.window, &env.event);
    }

    /// Take queued jobs: input goes into this frame's tick; replies wait for
    /// the paint.
    pub fn intake<S: Spec, A: App<Spec = S>>(&mut self, rt: &mut Runtime<S, A>, now: crate::types::ids::Seconds) {
        for (cmd, reply) in self.port.drain() {
            match cmd {
                AutoCmd::Input { win, events } => match resolve_window(rt, win) {
                    Some(w) => {
                        for event in events {
                            tap_input(&self.port, rt, w, &event);
                            rt.push_input(InputEnvelope { window: w, t: now, event });
                        }
                        self.after_paint.push((reply, w));
                    }
                    None => {
                        let _ = reply.send(AutoReply::Error(AutoError::NoWindow));
                    }
                },
                other => {
                    if let Some(job) = self.waits.take(other, reply) {
                        self.queries.push(job);
                    }
                }
            }
        }
    }

    /// A frame was painted for `win` on `background`.
    pub fn painted(&mut self, win: WindowId, background: u32) {
        self.waits.painted();
        self.background.insert(win, background);
    }

    /// Answer what waited for this frame's paint.
    pub fn answer<S: Spec, A: App<Spec = S>>(&mut self, rt: &mut Runtime<S, A>, dpr: f64) {
        let frame = self.waits.frames;
        for (reply, window) in std::mem::take(&mut self.after_paint) {
            let _ = reply.send(AutoReply::InputDone { window, frame });
        }
        let bg = self.background.clone();
        let dpr_of = |_: WindowId| dpr;
        let bg_of = |w: WindowId| bg.get(&w).copied().unwrap_or(0xffffffff);
        for (cmd, reply) in std::mem::take(&mut self.queries) {
            let answer = match cmd {
                // A canvas cannot be read back reliably (WebGL, tainting):
                // screenshots are painted offscreen.
                AutoCmd::Screenshot { source: ShotSource::Window, .. } => {
                    AutoReply::Error(AutoError::Unsupported("window readback in the browser (use source=offscreen)"))
                }
                AutoCmd::Screenshot { win, .. } => {
                    let c = AutoCmd::Screenshot { win, source: ShotSource::Offscreen };
                    answer_plain(rt, &c, &dpr_of, &bg_of).unwrap_or(AutoReply::Error(AutoError::Unsupported("screenshot")))
                }
                other => answer_plain(rt, &other, &dpr_of, &bg_of).unwrap_or(AutoReply::Error(AutoError::Unsupported("command"))),
            };
            let _ = reply.send(answer);
        }
    }

    /// Work is pending: keep frames coming.
    pub fn busy(&mut self) -> bool {
        !self.tasks.borrow().is_empty() || !self.after_paint.is_empty() || !self.queries.is_empty() || self.waits.anyone_waiting()
    }
}

#[cfg(test)]
mod tests {
    use super::query_has_dev;

    #[test]
    fn only_dev_1_turns_it_on() {
        assert!(query_has_dev("?dev=1"));
        assert!(query_has_dev("?x=2&dev=1"));
        assert!(!query_has_dev(""));
        assert!(!query_has_dev("?dev=0"));
        assert!(!query_has_dev("?dev=10"));
        assert!(!query_has_dev("?nodev=1"));
    }
}
