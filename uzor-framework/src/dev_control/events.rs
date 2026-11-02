//! The event log a transport streams (`GET /events` natively,
//! `__uzorDev.events(since)` in the web bridge): a bounded queue with
//! sequence numbers; native readers can long-poll it.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};

use crate::automation::now_ms;

/// Events kept (oldest dropped first).
pub const EVENT_CAP: usize = 1024;

struct Inner {
    next: u64,
    log: VecDeque<Value>,
}

/// A bounded, thread-safe event log.
pub struct EventLog {
    inner: Mutex<Inner>,
    #[cfg(not(target_arch = "wasm32"))]
    cv: std::sync::Condvar,
    started_ms: f64,
}

impl Default for EventLog {
    fn default() -> Self {
        Self::new()
    }
}

impl EventLog {
    /// An empty log; the first event gets `seq` 1.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner { next: 1, log: VecDeque::new() }),
            #[cfg(not(target_arch = "wasm32"))]
            cv: std::sync::Condvar::new(),
            started_ms: now_ms(),
        }
    }

    /// Append an event (`seq` and `t_ms` are filled in).
    pub fn emit(&self, mut event: Value) {
        let mut ev = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        event["seq"] = json!(ev.next);
        event["t_ms"] = json!((now_ms() - self.started_ms).max(0.0) as u64);
        ev.next += 1;
        ev.log.push_back(event);
        while ev.log.len() > EVENT_CAP {
            ev.log.pop_front();
        }
        drop(ev);
        #[cfg(not(target_arch = "wasm32"))]
        self.cv.notify_all();
    }

    /// Events with `seq > since`, waiting up to `wait` for the first one
    /// (no waiting on wasm: there is one thread).
    pub fn since(&self, since: u64, wait: Duration) -> Vec<Value> {
        let deadline = now_ms() + wait.as_secs_f64() * 1000.0;
        #[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
        let mut ev = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        loop {
            let out: Vec<Value> = ev
                .log
                .iter()
                .filter(|e| e.get("seq").and_then(Value::as_u64).is_some_and(|s| s > since))
                .cloned()
                .collect();
            let now = now_ms();
            if !out.is_empty() || now >= deadline || cfg!(target_arch = "wasm32") {
                return out;
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                ev = self.cv.wait_timeout(ev, Duration::from_secs_f64((deadline - now) / 1000.0)).unwrap_or_else(|p| p.into_inner()).0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seq_cap_and_wait() {
        let log = EventLog::new();
        assert!(log.since(0, Duration::from_millis(10)).is_empty());
        for i in 0..(EVENT_CAP + 5) {
            log.emit(json!({"i": i}));
        }
        let all = log.since(0, Duration::ZERO);
        assert_eq!(all.len(), EVENT_CAP);
        assert_eq!(all[0]["seq"], json!(6));
        assert_eq!(log.since(EVENT_CAP as u64 + 4, Duration::ZERO).len(), 1);
    }
}
