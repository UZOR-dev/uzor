//! Haptic feedback. The app asks; the native wrapper performs it
//! (`UIImpactFeedbackGenerator` / `UINotificationFeedbackGenerator` on iOS,
//! `View.performHapticFeedback` on Android). Until a wrapper registers a performer
//! the call is a no-op (desktop, tests, a host without the bridge).

use std::sync::RwLock;

/// The kinds both platforms have.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Haptic {
    /// A selection moved (picker tick, segmented control).
    Selection = 0,
    /// A light impact (toggle).
    Light = 1,
    /// A medium impact (snap, drop).
    Medium = 2,
    /// A heavy impact.
    Heavy = 3,
    /// Something succeeded.
    Success = 4,
    /// A warning.
    Warning = 5,
    /// Something failed.
    Error = 6,
}

impl Haptic {
    /// The code the C callback receives.
    pub fn code(self) -> i32 {
        self as i32
    }
}

type Performer = Box<dyn Fn(Haptic) + Send + Sync>;

static PERFORMER: RwLock<Option<Performer>> = RwLock::new(None);

/// Perform `kind` through the registered performer (no-op without one).
pub fn haptic(kind: Haptic) {
    if let Some(p) = PERFORMER.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
        p(kind);
    }
}

/// Register the performer (Rust side; a wrapper uses [`uzor_mobile_set_haptics`]).
pub fn set_performer(f: impl Fn(Haptic) + Send + Sync + 'static) {
    *PERFORMER.write().unwrap_or_else(|e| e.into_inner()) = Some(Box::new(f));
}

/// Whether a performer is registered.
pub fn available() -> bool {
    PERFORMER.read().unwrap_or_else(|e| e.into_inner()).is_some()
}

/// C: `void uzor_mobile_set_haptics(void (*perform)(int32_t kind))`; `kind` is
/// [`Haptic::code`]. Swift passes a `@convention(c)` closure.
#[no_mangle]
pub extern "C" fn uzor_mobile_set_haptics(perform: Option<extern "C" fn(i32)>) {
    match perform {
        Some(f) => set_performer(move |k| f(k.code())),
        None => *PERFORMER.write().unwrap_or_else(|e| e.into_inner()) = None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI32, Ordering};

    static LAST: AtomicI32 = AtomicI32::new(-1);
    extern "C" fn record(k: i32) {
        LAST.store(k, Ordering::Relaxed);
    }

    #[test]
    fn no_performer_is_a_no_op_then_the_c_callback_runs() {
        uzor_mobile_set_haptics(None);
        assert!(!available());
        haptic(Haptic::Success);
        assert_eq!(LAST.load(Ordering::Relaxed), -1);
        uzor_mobile_set_haptics(Some(record));
        haptic(Haptic::Warning);
        assert_eq!(LAST.load(Ordering::Relaxed), 5);
    }
}
