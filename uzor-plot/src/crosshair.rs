//! Crosshair model: cursor position in pixels, slot and value, magnet
//! snapping to nearby values, and tooltip placement. Drawing is left to
//! the caller.

/// How the crosshair follows the cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum CrosshairMode {
    /// Follow the cursor exactly.
    #[default]
    Free,
    /// Snap to the nearest candidate within a wide tolerance
    /// (40% of the slot spacing, clamped to 20–80 px). For candles, pass the
    /// body (`[open, close]`).
    StrongMagnet,
    /// Snap to the nearest candidate within a narrow tolerance
    /// (25% of the slot spacing, clamped to 10–50 px). For candles, pass
    /// `[open, high, low, close]`.
    WeakMagnet,
    /// Not shown.
    Hidden,
}

impl CrosshairMode {
    /// Whether this mode snaps.
    #[inline]
    pub fn is_magnet(self) -> bool {
        matches!(self, Self::StrongMagnet | Self::WeakMagnet)
    }

    /// Snap distance in pixels for slots `spacing` px apart; `None` when the
    /// mode does not snap.
    pub fn snap_tolerance(self, spacing: f64) -> Option<f64> {
        match self {
            Self::StrongMagnet => Some((spacing * 0.4).clamp(20.0, 80.0)),
            Self::WeakMagnet => Some((spacing * 0.25).clamp(10.0, 50.0)),
            Self::Free | Self::Hidden => None,
        }
    }
}

/// The candidate whose coordinate is nearest the cursor's, if within
/// `tolerance_px`; otherwise the cursor value. Returns `(value, coord)`.
/// Ties keep the earlier candidate.
pub fn snap_to_nearest(
    candidates: &[f64],
    cursor_value: f64,
    to_coord: impl Fn(f64) -> f64,
    tolerance_px: f64,
) -> (f64, f64) {
    let cursor = to_coord(cursor_value);
    let mut nearest = cursor_value;
    let mut best = f64::INFINITY;
    for &v in candidates {
        let d = (to_coord(v) - cursor).abs();
        if d < best {
            best = d;
            nearest = v;
        }
    }
    if best <= tolerance_px {
        (nearest, to_coord(nearest))
    } else {
        (cursor_value, cursor)
    }
}

/// Crosshair state.
///
/// ```
/// use uzor_plot::crosshair::{Crosshair, CrosshairMode};
/// let mut c = Crosshair::new().with_mode(CrosshairMode::WeakMagnet);
/// c.update(120.0, 48.0, Some(3), 3.4, 101.2, true);
/// // Candle at 100..104, y = 500 - value * 4.5 (so 101.2 sits at 44.6 px).
/// let to_y = |v: f64| 500.0 - v * 4.5;
/// let (v, y) = c.snap(&[100.0, 104.0, 99.0, 101.0], to_y, 40.0);
/// c.set_snapped(v, y);
/// assert_eq!(c.effective_value(false), 101.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Crosshair {
    /// User toggle. Default on.
    pub enabled: bool,
    /// Cursor is over the plot.
    pub visible: bool,
    /// Cursor x, px.
    pub x: f64,
    /// Cursor y, px (possibly clamped, see [`Self::update_clamped`]).
    pub y: f64,
    /// Data slot under the cursor, `None` outside the data.
    pub slot: Option<usize>,
    /// Fractional slot position under the cursor (valid past the data).
    pub pos: f64,
    /// Value at the cursor y.
    pub value: f64,
    /// Follow mode.
    pub mode: CrosshairMode,
    /// Snapped y (equals `y` until [`Self::set_snapped`]).
    pub snapped_y: f64,
    /// Snapped value (equals `value` until [`Self::set_snapped`]).
    pub snapped_value: f64,
}

impl Default for Crosshair {
    fn default() -> Self {
        Self {
            enabled: true,
            visible: false,
            x: 0.0,
            y: 0.0,
            slot: None,
            pos: 0.0,
            value: 0.0,
            mode: CrosshairMode::default(),
            snapped_y: 0.0,
            snapped_value: 0.0,
        }
    }
}

impl Crosshair {
    /// Enabled, hidden, free mode.
    pub fn new() -> Self {
        Self::default()
    }
    /// Set the mode.
    pub fn with_mode(mut self, mode: CrosshairMode) -> Self {
        self.mode = mode;
        self
    }

    /// Magnet on (weak) when off or hidden; off when any magnet is on.
    pub fn toggle_magnet(&mut self) {
        self.mode = match self.mode {
            CrosshairMode::Free | CrosshairMode::Hidden => CrosshairMode::WeakMagnet,
            CrosshairMode::WeakMagnet | CrosshairMode::StrongMagnet => CrosshairMode::Free,
        };
    }

    /// Whether the mode snaps.
    #[inline]
    pub fn is_magnet(&self) -> bool {
        self.mode.is_magnet()
    }

    /// Whether a magnet moved the line more than 2 px off the cursor.
    #[inline]
    pub fn is_snapped(&self) -> bool {
        self.is_magnet() && (self.snapped_y - self.y).abs() > 2.0
    }

    /// Move to the cursor; resets the snap to the cursor.
    pub fn update(&mut self, x: f64, y: f64, slot: Option<usize>, pos: f64, value: f64, visible: bool) {
        self.x = x;
        self.y = y;
        self.slot = slot;
        self.pos = pos;
        self.value = value;
        self.visible = visible;
        self.snapped_y = y;
        self.snapped_value = value;
    }

    /// [`Self::update`], with y clamped to `[0, height]` when `clamp` (an
    /// auto-fitted value axis, where the cursor cannot leave the plot).
    #[allow(clippy::too_many_arguments)]
    pub fn update_clamped(
        &mut self,
        x: f64,
        y: f64,
        slot: Option<usize>,
        pos: f64,
        value: f64,
        visible: bool,
        clamp: bool,
        height: f64,
    ) {
        self.x = x;
        self.slot = slot;
        self.pos = pos;
        self.value = value;
        self.visible = visible;
        self.y = if clamp { y.clamp(0.0, height.max(0.0)) } else { y };
        self.snapped_y = self.y;
        self.snapped_value = value;
    }

    /// Where the mode would snap the cursor value among `candidates`
    /// (`(value, coord)`); the cursor itself in free and hidden modes.
    pub fn snap(&self, candidates: &[f64], to_coord: impl Fn(f64) -> f64, spacing: f64) -> (f64, f64) {
        match self.mode.snap_tolerance(spacing) {
            Some(tol) => snap_to_nearest(candidates, self.value, to_coord, tol),
            None => (self.value, to_coord(self.value)),
        }
    }

    /// Store a snap result.
    pub fn set_snapped(&mut self, value: f64, y: f64) {
        self.snapped_value = value;
        self.snapped_y = y;
    }

    /// y to draw: snapped with a magnet unless dragging.
    #[inline]
    pub fn effective_y(&self, dragging: bool) -> f64 {
        if dragging || !self.is_magnet() {
            self.y
        } else {
            self.snapped_y
        }
    }

    /// Value to show: snapped with a magnet unless dragging.
    #[inline]
    pub fn effective_value(&self, dragging: bool) -> f64 {
        if dragging || !self.is_magnet() {
            self.value
        } else {
            self.snapped_value
        }
    }

    /// Hide.
    pub fn hide(&mut self) {
        self.visible = false;
    }
    /// Show.
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Top-left of a tooltip of `size` next to (`anchor_x`, effective y),
    /// offset by `offset`, flipped left/up when it would leave `area`, then
    /// clamped inside it.
    pub fn tooltip_position(&self, anchor_x: f64, dragging: bool, area: (f64, f64), size: (f64, f64), offset: (f64, f64)) -> (f64, f64) {
        let (aw, ah) = area;
        let (tw, th) = size;
        let anchor_y = self.effective_y(dragging);
        let mut x = anchor_x + offset.0;
        let mut y = anchor_y + offset.1;
        if x + tw > aw {
            x = anchor_x - tw - offset.0;
        }
        if y + th > ah {
            y = anchor_y - th - offset.1;
        }
        x = x.clamp(0.0, (aw - tw).max(0.0));
        y = y.clamp(0.0, (ah - th).max(0.0));
        (x, y)
    }
}
