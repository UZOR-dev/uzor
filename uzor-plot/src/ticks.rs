//! Nice-step ticks, tick density and label precision.
//!
//! The step ladder is `1, 2, 5, 10, 20, 50, ...` built by multiplying the
//! decade base by the `[2, 2.5, 2]` pattern ([`NICE_MULTIPLIERS`]). A step a
//! little under the rough step (down to 80% of it) is preferred over the next
//! larger rung, so grids stay a touch denser rather than sparser.
//!
//! [`nice_step_ceil`] is the other common rule: round the rough step *up* to
//! `1, 2, 2.5, 5 × 10^k`, never below a floor (useful for row buckets that
//! must not be finer than an instrument increment).
//!
//! Everything here is pure `f64` math and string formatting.

/// Multiplier pattern of the nice-step ladder: `base × 2 × 2.5 × 2 = base × 10`.
pub const NICE_MULTIPLIERS: [f64; 3] = [2.0, 2.5, 2.0];

/// The nice ladder value closest to `value` from the `1, 2, 5 × 10^n` rungs.
///
/// Returns the rung just below `value` when it is at least 80% of `value`,
/// otherwise the first rung at or above it. Non-positive or non-finite
/// input (`NaN`, `±inf`) gives `1.0`.
///
/// ```
/// use uzor_plot::ticks::nice_number;
/// assert_eq!(nice_number(23.0), 20.0); // 20 >= 0.8 * 23
/// assert_eq!(nice_number(7.0), 10.0);  // 5 < 0.8 * 7, so the next rung
/// ```
pub fn nice_number(value: f64) -> f64 {
    if !value.is_finite() || value <= 0.0 {
        return 1.0;
    }

    let exp = value.log10().floor();
    let base = 10.0_f64.powf(exp);

    let mut current = base;
    let mut idx = 0;

    while current < value {
        current *= NICE_MULTIPLIERS[idx % 3];
        idx += 1;
        if idx > 10 {
            break;
        }
    }

    if idx > 0 {
        let prev_idx = idx - 1;
        let mut check = base;
        for i in 0..prev_idx {
            check *= NICE_MULTIPLIERS[i % 3];
        }
        if check >= value * 0.8 {
            return check;
        }
    }

    current
}

/// Nice step for `range` split into about `target_ticks` intervals.
#[inline]
pub fn nice_step(range: f64, target_ticks: f64) -> f64 {
    nice_number(range / target_ticks)
}

/// Round `range / target_count` *up* to `1, 2, 2.5, 5 × 10^k`, never below
/// `min_step` (itself floored at `1e-12`). `target_count` is floored at 2.
/// A non-finite or non-positive range gives the floor.
pub fn nice_step_ceil(range: f64, min_step: f64, target_count: f64) -> f64 {
    let min_step = min_step.max(1e-12);
    if !range.is_finite() || range <= 0.0 {
        return min_step;
    }

    let raw_step = (range / target_count.max(2.0)).max(min_step);
    if raw_step <= min_step {
        return min_step;
    }

    let exp = raw_step.log10().floor();
    let base = 10f64.powf(exp);
    let mantissa = raw_step / base;

    let nice_mantissa = if mantissa <= 1.0 {
        1.0
    } else if mantissa <= 2.0 {
        2.0
    } else if mantissa <= 2.5 {
        2.5
    } else if mantissa <= 5.0 {
        5.0
    } else {
        10.0
    };

    (nice_mantissa * base).max(min_step)
}

// ---------------------------------------------------------------------------
// Density
// ---------------------------------------------------------------------------

/// How dense an axis' ticks are, in pixels.
///
/// The target tick count is `length_px / spacing_px` clamped to
/// `[min_ticks, max_ticks]` (bounds given the wrong way round are swapped;
/// a `NaN` count, e.g. `0 / 0`, gives the lower bound). Generated ticks run `overscan_px` worth of
/// value past both ends, so labels reach the edges while panning.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct TickDensity {
    /// Desired pixels between grid lines. Default 30.
    pub spacing_px: f64,
    /// Lower clamp on the target tick count. Default 4.
    pub min_ticks: f64,
    /// Upper clamp on the target tick count. Default 20.
    pub max_ticks: f64,
    /// Pixels of overscan generated past each end. Default 50.
    pub overscan_px: f64,
}

impl Default for TickDensity {
    fn default() -> Self {
        Self { spacing_px: 30.0, min_ticks: 4.0, max_ticks: 20.0, overscan_px: 50.0 }
    }
}

impl TickDensity {
    /// Set the desired pixels between grid lines.
    pub fn with_spacing_px(mut self, px: f64) -> Self {
        self.spacing_px = px;
        self
    }
    /// Set the clamp on the target tick count.
    pub fn with_tick_bounds(mut self, min: f64, max: f64) -> Self {
        self.min_ticks = min;
        self.max_ticks = max;
        self
    }
    /// Set the overscan past each end, in pixels.
    pub fn with_overscan_px(mut self, px: f64) -> Self {
        self.overscan_px = px;
        self
    }

    /// Target tick count for an axis `length_px` long.
    #[inline]
    pub fn target_ticks(&self, length_px: f64) -> f64 {
        let (lo, hi) = if self.min_ticks <= self.max_ticks {
            (self.min_ticks, self.max_ticks)
        } else {
            (self.max_ticks, self.min_ticks)
        };
        let raw = length_px / self.spacing_px;
        // Same result as `f64::clamp` for ordered bounds, but no panic on
        // `NaN` or swapped bounds.
        if raw.is_nan() || raw < lo {
            lo
        } else if raw > hi {
            hi
        } else {
            raw
        }
    }

    /// Nice step for `range` on an axis `length_px` long.
    #[inline]
    pub fn step(&self, range: f64, length_px: f64) -> f64 {
        nice_step(range, self.target_ticks(length_px))
    }

    /// Value-space overscan for `range` on an axis `length_px` long.
    ///
    /// 0 when the axis is not usable: shorter than [`MIN_AXIS_PX`] (zero,
    /// sub-pixel, negative) or not finite, or when the margin itself would
    /// not be finite. A sub-pixel axis would otherwise turn 50 px of
    /// overscan into a span millions of steps wide.
    #[inline]
    pub fn overscan(&self, range: f64, length_px: f64) -> f64 {
        if length_px.is_finite() && length_px >= MIN_AXIS_PX {
            let margin = self.overscan_px * range / length_px;
            if margin.is_finite() {
                margin
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Ticks for `[min, max]` on an axis `length_px` long, overscan
    /// included. Returns the step and the tick values (ascending), at most
    /// [`MAX_TICKS`] of them (see [`step_ticks`]).
    pub fn ticks(&self, min: f64, max: f64, length_px: f64) -> (f64, Vec<f64>) {
        let range = max - min;
        let step = self.step(range, length_px);
        let margin = self.overscan(range, length_px);
        (step, step_ticks(min - margin, max + margin, step))
    }
}

/// Shortest axis, in pixels, that gets overscan ([`TickDensity::overscan`]).
pub const MIN_AXIS_PX: f64 = 1.0;

/// Most ticks one grid may hold: [`step_ticks`] and [`bounded_ticks`]
/// return **no ticks** for a span that needs more than this many steps.
///
/// A real axis needs tens of ticks; a grid this dense is degenerate input
/// (a huge span over a tiny step), and cutting it short would draw a
/// partial axis that looks valid, so it is dropped whole instead. The cap
/// also bounds the work and memory per call.
pub const MAX_TICKS: usize = 10_000;

/// Upper bound on [`step_ticks`]' up-front allocation; longer runs grow as usual.
const STEP_TICKS_PREALLOC: usize = 1024;

/// Loop count for a grid of `step` from `first` up to `hi`:
/// `ceil((hi - first) / step) + 1`, computed in `f64` so it cannot
/// overflow. `Some(0)` when that is not positive, `None` when it is `NaN`
/// or the step count `ceil((hi - first) / step)` is above `limit`.
#[inline]
pub(crate) fn tick_loop_count(first: f64, hi: f64, step: f64, limit: usize) -> Option<usize> {
    let steps = ((hi - first) / step).ceil();
    if steps.is_nan() || steps > limit as f64 {
        return None;
    }
    let count = steps + 1.0;
    Some(if count > 0.0 { count as usize } else { 0 })
}

/// Multiples of `step` in `[lo, hi)`, computed by index (no accumulation).
///
/// Empty when `step` is not positive and finite, either end is not
/// finite, or the span needs more than [`MAX_TICKS`] steps; never returns
/// more than [`MAX_TICKS`] values.
pub fn step_ticks(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    if !(step.is_finite() && step > 0.0 && lo.is_finite() && hi.is_finite()) {
        return Vec::new();
    }
    let first = (lo / step).ceil() * step;
    // Values pushed are at most ceil((hi - first) / step); the loop runs
    // one index past that, as the original did, and breaks at `hi`.
    let Some(num_ticks) = tick_loop_count(first, hi, step, MAX_TICKS) else {
        return Vec::new();
    };
    let mut ticks = Vec::with_capacity(num_ticks.min(STEP_TICKS_PREALLOC));
    for i in 0..num_ticks {
        let v = first + (i as f64) * step;
        if v >= hi {
            break;
        }
        ticks.push(v);
    }
    ticks
}

/// Nice ticks strictly inside `[min, max]` (both ends inclusive within
/// `1e-9 × step`), about `target_count` intervals. Empty for an empty or
/// non-finite range, a zero target, or a grid of more than [`MAX_TICKS`].
pub fn bounded_ticks(min: f64, max: f64, target_count: usize) -> Vec<f64> {
    let range = max - min;
    if !range.is_finite() || range <= 0.0 || target_count == 0 {
        return Vec::new();
    }

    let step = nice_step(range, target_count as f64);
    if step <= 0.0 {
        return Vec::new();
    }

    let first = (min / step).ceil() * step;
    // The tolerance can admit one value past ceil((max - first) / step).
    let Some(num_ticks) = tick_loop_count(first, max, step, MAX_TICKS - 1) else {
        return Vec::new();
    };
    let tolerance = step * 1e-9;

    let mut ticks = Vec::new();
    for i in 0..num_ticks {
        let value = first + (i as f64) * step;
        if value > max + tolerance {
            break;
        }
        if value >= min - tolerance {
            ticks.push(value);
        }
    }
    ticks
}

// ---------------------------------------------------------------------------
// Label culling
// ---------------------------------------------------------------------------

/// A candidate axis label: its value and the pixel coordinate of its centre.
///
/// Build it with [`AxisLabel::new`] or [`Default`] (value and position 0).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct AxisLabel {
    /// Data value.
    pub value: f64,
    /// Pixel coordinate of the label centre along the axis.
    pub pos: f64,
}

impl AxisLabel {
    /// A label for `value` centred at `pos`.
    pub const fn new(value: f64, pos: f64) -> Self {
        Self { value, pos }
    }
}

/// Keep labels that fit fully inside `[start, end]`, miss every `reserved`
/// interval and do not overlap a label kept before them.
///
/// `half_extent` is half a label's footprint along the axis, margin
/// included. Candidates are taken in order; the first one wins a collision.
pub fn cull_axis_labels(
    candidates: &[AxisLabel],
    start: f64,
    end: f64,
    reserved: &[(f64, f64)],
    half_extent: f64,
) -> Vec<AxisLabel> {
    let mut occupied: Vec<(f64, f64)> = reserved.to_vec();
    let mut kept = Vec::new();

    for candidate in candidates {
        let a = candidate.pos - half_extent;
        let b = candidate.pos + half_extent;

        if a < start || b > end {
            continue;
        }
        if occupied.iter().any(|&(x0, x1)| x0 < b && a < x1) {
            continue;
        }

        occupied.push((a, b));
        kept.push(*candidate);
    }

    kept
}

// ---------------------------------------------------------------------------
// Label precision
// ---------------------------------------------------------------------------

const DECADE_THRESHOLDS: [f64; 12] = [
    1.0, 0.1, 0.01, 0.001, 0.0001, 0.00001, 0.000001, 0.0000001, 0.00000001, 0.000000001,
    0.0000000001, 0.00000000001,
];

/// Decimals needed to show multiples of `step`, never fewer than `min_decimals`.
///
/// `step >= 1` needs 0, `>= 0.1` needs 1, ... down to 12 for anything finer
/// than `1e-11`.
pub fn step_decimals(step: f64, min_decimals: usize) -> usize {
    let mut d = DECADE_THRESHOLDS.len();
    for (i, t) in DECADE_THRESHOLDS.iter().enumerate() {
        if step >= *t {
            d = i;
            break;
        }
    }
    d.max(min_decimals)
}

/// Decimals for a price-like step: [`step_decimals`] with a floor of 2.
#[inline]
pub fn price_step_decimals(step: f64) -> usize {
    step_decimals(step, 2)
}

/// Decimal places implied by a positive finite increment (`0.25` → 2,
/// `0.1` → 1, `10` → 0), read from its 12-digit rendering. `None` for
/// non-finite or non-positive input.
pub fn decimals_from_increment(increment: f64) -> Option<usize> {
    if !increment.is_finite() || increment <= 0.0 {
        return None;
    }
    let rendered = format!("{:.12}", increment);
    let fraction = match rendered.split_once('.') {
        Some((_, fraction)) => fraction,
        None => "",
    };
    let trimmed = fraction.trim_end_matches('0');
    if trimmed.is_empty() {
        Some(0)
    } else {
        Some(trimmed.len().min(12))
    }
}

/// Drop zeros that fixed-precision formatting padded onto a fraction
/// (`86800.0` → `86800`, `6.500` → `6.5`). Labels without a `.` are returned as is.
pub fn trim_padded_fraction(label: &str) -> String {
    match label.split_once('.') {
        Some((whole, frac)) => {
            let trimmed = frac.trim_end_matches('0');
            if trimmed.is_empty() {
                whole.to_string()
            } else {
                format!("{whole}.{trimmed}")
            }
        }
        None => label.to_string(),
    }
}

/// Most fractional digits [`format_fixed`] (and so [`LabelPrecision::format`])
/// writes; larger requests are capped. `f64` carries about 17 significant
/// digits, and the formatter itself rejects precisions above `u16::MAX`.
pub const MAX_DECIMALS: usize = 64;

/// `value` with exactly `decimals` fractional digits (at most [`MAX_DECIMALS`]).
#[inline]
pub fn format_fixed(value: f64, decimals: usize) -> String {
    format!("{:.prec$}", value, prec = decimals.min(MAX_DECIMALS))
}

/// How many decimals an axis label shows.
///
/// - `pinned`: a user-chosen count, used as is and padded with zeros.
/// - otherwise auto: the step's decimals ([`step_decimals`] with `min_decimals`),
///   capped by `ceiling` (e.g. the data's own increment) when set, with padded
///   zeros trimmed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct LabelPrecision {
    /// Fixed decimal count; wins over everything else.
    pub pinned: Option<usize>,
    /// Upper bound on auto decimals.
    pub ceiling: Option<usize>,
    /// Lower bound on the step-derived decimals. Default 2 (price-like).
    pub min_decimals: usize,
}

impl Default for LabelPrecision {
    fn default() -> Self {
        Self { pinned: None, ceiling: None, min_decimals: 2 }
    }
}

impl LabelPrecision {
    /// Pin the decimal count (`None` = auto).
    pub fn with_pinned(mut self, pinned: Option<usize>) -> Self {
        self.pinned = pinned;
        self
    }
    /// Cap auto decimals (`None` = no cap).
    pub fn with_ceiling(mut self, ceiling: Option<usize>) -> Self {
        self.ceiling = ceiling;
        self
    }
    /// Floor on step-derived decimals.
    pub fn with_min_decimals(mut self, min: usize) -> Self {
        self.min_decimals = min;
        self
    }

    /// Decimals for labels on a grid of `step`.
    pub fn decimals(&self, step: f64) -> usize {
        let from_step = step_decimals(step, self.min_decimals);
        match self.pinned {
            Some(p) => p,
            None => self.ceiling.map(|c| c.min(from_step)).unwrap_or(from_step),
        }
    }

    /// Format `value` as a label on a grid of `step`.
    ///
    /// ```
    /// use uzor_plot::ticks::LabelPrecision;
    /// let auto = LabelPrecision::default().with_ceiling(Some(1));
    /// assert_eq!(auto.format(86_800.0, 100.0), "86800");
    /// assert_eq!(auto.format(86_800.5, 100.0), "86800.5");
    /// let pinned = auto.with_pinned(Some(1));
    /// assert_eq!(pinned.format(86_800.0, 100.0), "86800.0");
    /// ```
    pub fn format(&self, value: f64, step: f64) -> String {
        let s = format_fixed(value, self.decimals(step));
        if self.pinned.is_some() {
            s
        } else {
            trim_padded_fraction(&s)
        }
    }
}
