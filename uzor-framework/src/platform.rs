//! Platform facts the event loop cannot see, pushed by the thin native wrapper
//! (Swift / Kotlin, through `uzor-window-mobile`) from any thread: the safe area
//! where winit does not report it (Android `WindowInsets`) and the soft keyboard
//! height (iOS `keyboardWillChangeFrame`, Android `WindowInsets.ime`), both in
//! logical px. The native host picks them up on its next loop turn and echoes
//! `WindowInput::Insets` to every window.

use std::sync::Mutex;

use crate::types::window::Insets;

#[derive(Default)]
struct State {
    safe: Option<Insets>,
    keyboard: f64,
    dirty: bool,
    wake: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(State::default))
}

fn changed(s: &mut State) -> Option<std::sync::Arc<dyn Fn() + Send + Sync>> {
    s.dirty = true;
    s.wake.clone()
}

/// The platform's safe area (overrides what the host derives itself; on iOS winit
/// already reports it, on Android only the wrapper knows it).
pub fn set_safe_area(safe: Insets) {
    let wake = with(|s| {
        if s.safe == Some(safe) {
            return None;
        }
        s.safe = Some(safe);
        changed(s)
    });
    if let Some(w) = wake {
        w();
    }
}

/// The soft keyboard's height over the bottom of the window (0 when hidden).
pub fn set_keyboard(height: f64) {
    let height = height.max(0.0);
    let wake = with(|s| {
        if s.keyboard == height {
            return None;
        }
        s.keyboard = height;
        changed(s)
    });
    if let Some(w) = wake {
        w();
    }
}

/// The last values the wrapper pushed: (safe area if it set one, keyboard height).
pub fn current() -> (Option<Insets>, f64) {
    with(|s| (s.safe, s.keyboard))
}

/// New values since the last call (the host's loop turn).
#[allow(dead_code)]
pub(crate) fn take_changed() -> Option<(Option<Insets>, f64)> {
    with(|s| std::mem::take(&mut s.dirty).then_some((s.safe, s.keyboard)))
}

/// Wake the host's loop (from any thread): something it polls changed (a tray
/// menu pick, a tray widget update).
#[allow(dead_code)]
pub(crate) fn wake() {
    if let Some(w) = with(|s| s.wake.clone()) {
        w();
    }
}

/// The host's loop wake, called after every change.
#[allow(dead_code)]
pub(crate) fn set_wake(f: impl Fn() + Send + Sync + 'static) {
    with(|s| s.wake = Some(std::sync::Arc::new(f)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_are_taken_once_and_wake_the_loop() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static WOKE: AtomicUsize = AtomicUsize::new(0);
        set_wake(|| {
            WOKE.fetch_add(1, Ordering::Relaxed);
        });
        let _ = take_changed();
        set_keyboard(300.0);
        set_keyboard(300.0);
        assert_eq!(WOKE.load(Ordering::Relaxed), 1, "same value does not wake again");
        assert_eq!(take_changed().map(|c| c.1), Some(300.0));
        assert_eq!(take_changed(), None);
        set_safe_area(Insets::new(24.0, 0.0, 48.0, 0.0));
        assert_eq!(take_changed(), Some((Some(Insets::new(24.0, 0.0, 48.0, 0.0)), 300.0)));
        set_keyboard(-5.0);
        assert_eq!(current().1, 0.0);
    }
}
