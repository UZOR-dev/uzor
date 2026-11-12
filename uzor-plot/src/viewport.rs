//! Horizontal viewport over evenly spaced slots (bars, candles, categories):
//! slot ↔ x mapping, visible window, pan, zoom around a point with spacing
//! limits, fit helpers, and kinetic (momentum) panning.
//!
//! Slot `i` is centred at `(i - start) × spacing + spacing / 2`. `start` is
//! fractional and may run past either end of the data.

use core::ops::Range;

/// A horizontal slot viewport.
///
/// ```
/// use uzor_plot::viewport::SlotViewport;
/// let mut v = SlotViewport::new(800.0).with_spacing(10.0).with_count(500);
/// assert_eq!(v.visible_slots(), 80);
/// v.scroll_to_end();
/// assert_eq!(v.start, 420.0);
/// assert_eq!(v.slot_to_x(420), 5.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct SlotViewport {
    /// First visible slot (fractional; negative or past the end is allowed).
    pub start: f64,
    /// Pixels per slot. Default 8.
    pub spacing: f64,
    /// Body width as a fraction of the spacing. Default 0.8.
    pub body_ratio: f64,
    /// Viewport width in pixels.
    pub width: f64,
    /// Number of data slots.
    pub count: usize,
    /// Smallest spacing zoom allows. Default 0.7.
    pub min_spacing: f64,
    /// Largest spacing as a fraction of the width. Default 0.5.
    pub max_spacing_ratio: f64,
}

impl Default for SlotViewport {
    fn default() -> Self {
        Self {
            start: 0.0,
            spacing: 8.0,
            body_ratio: 0.8,
            width: 0.0,
            count: 0,
            min_spacing: 0.7,
            max_spacing_ratio: 0.5,
        }
    }
}

impl SlotViewport {
    /// A viewport `width` px wide, otherwise default.
    pub fn new(width: f64) -> Self {
        Self { width, ..Default::default() }
    }
    /// Set the slot spacing.
    pub fn with_spacing(mut self, spacing: f64) -> Self {
        self.spacing = spacing;
        self
    }
    /// Set the data slot count.
    pub fn with_count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }
    /// Set the first visible slot.
    pub fn with_start(mut self, start: f64) -> Self {
        self.start = start;
        self
    }
    /// Set the body/spacing ratio.
    pub fn with_body_ratio(mut self, ratio: f64) -> Self {
        self.body_ratio = ratio;
        self
    }
    /// Set the zoom limits.
    pub fn with_spacing_limits(mut self, min_spacing: f64, max_spacing_ratio: f64) -> Self {
        self.min_spacing = min_spacing;
        self.max_spacing_ratio = max_spacing_ratio;
        self
    }

    /// Largest spacing zoom allows (`width × max_spacing_ratio`).
    #[inline]
    pub fn max_spacing(&self) -> f64 {
        self.width * self.max_spacing_ratio
    }

    /// Whole slots that fit across the width (at least 1).
    #[inline]
    pub fn visible_slots(&self) -> usize {
        ((self.width / self.spacing) as usize).max(1)
    }

    /// Slots the width holds at the smallest spacing; 0 before layout.
    #[inline]
    pub fn max_fit_slots(&self) -> usize {
        if self.width <= 0.0 {
            return 0;
        }
        (self.width / self.min_spacing).ceil() as usize
    }

    /// `start` as a data index clamped to `[0, count - 1]`.
    #[inline]
    pub fn start_index(&self) -> usize {
        if self.start < 0.0 {
            0
        } else {
            (self.start as usize).min(self.count.saturating_sub(1))
        }
    }

    /// Data indices to draw, one extra slot each side for partly visible
    /// slots, clamped to the data.
    #[inline]
    pub fn visible_range(&self) -> Range<usize> {
        let s = self.start_index();
        let start = if s > 0 { s - 1 } else { 0 };
        let visible = self.width / self.spacing;
        let end = ((self.start + visible).ceil() as usize + 1).min(self.count);
        start..end
    }

    /// Centre x of slot `index`.
    #[inline]
    pub fn slot_to_x(&self, index: usize) -> f64 {
        self.pos_to_x(index as f64)
    }

    /// Centre x of fractional slot position `pos`.
    #[inline]
    pub fn pos_to_x(&self, pos: f64) -> f64 {
        let rel = pos - self.start;
        rel * self.spacing + self.spacing / 2.0
    }

    /// Data slot under `x`, if `x` is inside the width and the data.
    #[inline]
    pub fn x_to_slot(&self, x: f64) -> Option<usize> {
        if x < 0.0 || x > self.width {
            return None;
        }
        let i = (self.start + x / self.spacing) as i64;
        if i >= 0 && (i as usize) < self.count {
            Some(i as usize)
        } else {
            None
        }
    }

    /// Fractional slot position at `x` (slot left edges are integers).
    #[inline]
    pub fn x_to_pos(&self, x: f64) -> f64 {
        self.start + x / self.spacing
    }

    /// Body width in pixels.
    #[inline]
    pub fn body_width(&self) -> f64 {
        self.spacing * self.body_ratio
    }

    /// Pan by a pixel drag delta (dragging right reveals earlier slots).
    pub fn pan_px(&mut self, dx: f64) {
        if self.spacing > 0.0 {
            self.start -= dx / self.spacing;
        }
    }

    /// Pan by a slot delta.
    pub fn pan_slots(&mut self, delta: f64) {
        self.start -= delta;
    }

    /// Zoom by `factor` (> 1 zooms in) keeping the slot under `x` fixed,
    /// within the built-in spacing limits. A zero-width viewport keeps the
    /// smallest spacing instead of failing.
    pub fn zoom_at(&mut self, x: f64, factor: f64) {
        let min = self.min_spacing;
        let max = self.max_spacing().max(min);
        self.zoom_at_within(x, factor, min, max);
    }

    /// [`Self::zoom_at`] with explicit spacing limits (`min <= max`).
    pub fn zoom_at_within(&mut self, x: f64, factor: f64, min: f64, max: f64) {
        let old = self.spacing;
        self.spacing = (self.spacing * factor).clamp(min, max);
        let under = self.start + x / old;
        self.start = under - x / self.spacing;
    }

    /// Show the last slots at the right edge.
    pub fn scroll_to_end(&mut self) {
        self.start = self.count.saturating_sub(self.visible_slots()) as f64;
    }

    /// Do not scroll past the last full window of data.
    pub fn clamp_start(&mut self) {
        let max_start = self.count.saturating_sub(self.visible_slots()) as f64;
        if self.start > max_start {
            self.start = max_start;
        }
    }

    /// Fit every slot in the width within the built-in limits.
    pub fn fit_all(&mut self) {
        let min = self.min_spacing;
        let max = self.max_spacing().max(min);
        self.fit_all_within(min, max);
    }

    /// Fit every slot in the width within explicit limits.
    pub fn fit_all_within(&mut self, min: f64, max: f64) {
        if self.count > 0 {
            self.spacing = (self.width / self.count as f64).clamp(min, max);
            self.start = 0.0;
        }
    }

    /// Show about `data_slots` recent slots with `margin_slots` empty slots
    /// after the last one, within explicit limits (a `max` below `min` is
    /// raised to `min`).
    pub fn fit_recent(&mut self, data_slots: usize, margin_slots: usize, min: f64, max: f64) {
        let total = data_slots + margin_slots;
        let spacing = self.width / total as f64;
        self.spacing = spacing.clamp(min, max.max(min));
        let right_edge = self.count.saturating_sub(1) as f64 + margin_slots as f64;
        self.start = right_edge - self.width / self.spacing;
    }

    /// [`Self::fit_recent`] with 95 data slots, 5 margin slots and the built-in limits.
    pub fn reset_to_default(&mut self) {
        let (min, max) = (self.min_spacing, self.max_spacing());
        self.fit_recent(95, 5, min, max);
    }

    /// Keep the right edge in place when an inset of `px` opens (`true`) or
    /// closes on the right. Call before changing [`Self::width`].
    pub fn shift_for_right_inset(&mut self, opening: bool, px: f64) {
        if self.spacing > 0.0 && px > 0.0 {
            let d = px / self.spacing;
            if opening {
                self.start += d;
            } else {
                self.start -= d;
            }
        }
    }
}

/// Fractional slot position of `key` in items sorted by key.
///
/// An exact hit is its index; a key between two items interpolates by key;
/// a key before the first or after the last extrapolates with the average
/// key step (whole series before, last step after), or `default_step` when
/// there is one item.
pub fn fractional_position<T>(items: &[T], key: i64, key_of: impl Fn(&T) -> i64, default_step: f64) -> f64 {
    if items.is_empty() {
        return 0.0;
    }
    match items.binary_search_by_key(&key, &key_of) {
        Ok(i) => i as f64,
        Err(0) => {
            let first = key_of(&items[0]);
            let dt = (key - first) as f64;
            let step = if items.len() >= 2 {
                (key_of(&items[items.len() - 1]) - first) as f64 / (items.len() - 1).max(1) as f64
            } else {
                default_step
            };
            if step > 0.0 {
                dt / step
            } else {
                0.0
            }
        }
        Err(i) if i >= items.len() => {
            let last = key_of(&items[items.len() - 1]);
            let dt = (key - last) as f64;
            let step = if items.len() >= 2 {
                (last - key_of(&items[items.len() - 2])) as f64
            } else {
                default_step
            };
            (items.len() - 1) as f64 + if step > 0.0 { dt / step } else { 0.0 }
        }
        Err(i) => {
            let prev = key_of(&items[i - 1]) as f64;
            let next = key_of(&items[i]) as f64;
            let span = next - prev;
            let frac = if span > 0.0 { (key as f64 - prev) / span } else { 0.0 };
            (i - 1) as f64 + frac
        }
    }
}

/// Momentum panning with exponential decay.
///
/// Velocity is in pixels per millisecond; times are milliseconds.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Kinetic {
    /// Current velocity, px/ms.
    pub velocity: f64,
    /// Time of the last update, ms.
    pub last_time: f64,
    /// Whether the motion is running.
    pub active: bool,
    /// Velocity kept per millisecond. Default 0.997.
    pub damping: f64,
    /// Speed below which the motion stops. Default 0.001.
    pub min_velocity: f64,
    /// Drag speed → start velocity factor. Default 0.3.
    pub friction: f64,
}

impl Default for Kinetic {
    fn default() -> Self {
        Self { velocity: 0.0, last_time: 0.0, active: false, damping: 0.997, min_velocity: 0.001, friction: 0.3 }
    }
}

impl Kinetic {
    /// Start with `velocity` at time `now`; too slow a start stays idle.
    pub fn start(&mut self, velocity: f64, now: f64) {
        self.velocity = velocity;
        self.last_time = now;
        self.active = velocity.abs() > self.min_velocity;
    }
    /// Stop now.
    pub fn stop(&mut self) {
        self.active = false;
        self.velocity = 0.0;
    }
    /// Whether the motion is running.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.active
    }
    /// Advance to `now`; returns the slot delta to move (pixel travel ÷
    /// `spacing`), or `None` once stopped.
    pub fn update(&mut self, now: f64, spacing: f64) -> Option<f64> {
        if !self.active {
            return None;
        }
        let dt = now - self.last_time;
        self.last_time = now;
        self.velocity *= self.damping.powf(dt);
        let delta = self.velocity * dt / spacing;
        if self.velocity.abs() < self.min_velocity {
            self.active = false;
            self.velocity = 0.0;
            return None;
        }
        Some(delta)
    }
    /// Start velocity for a drag of `dx` px over `dt` ms (0 when `dt <= 0`).
    pub fn velocity_from_drag(&self, dx: f64, dt: f64) -> f64 {
        if dt > 0.0 {
            (dx / dt) * self.friction
        } else {
            0.0
        }
    }
}
