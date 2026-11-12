//! A value axis: linear, log, percent or indexed-to-100 mapping between
//! data values and pixel coordinates, auto-range with margins, zoom/pan,
//! ticks and labels.
//!
//! Coordinates run top-down: `0` is the top edge (the range maximum) and
//! `length` the bottom edge (the range minimum), as on a screen.

use core::ops::Range;

use crate::ticks::{nice_step, step_ticks, tick_loop_count, LabelPrecision, TickDensity, MAX_TICKS};

/// How values map onto the axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum ScaleKind {
    /// Linear in the value.
    #[default]
    Linear,
    /// Linear in `ln(value)`; equal ratios get equal distance. Values are
    /// floored at `1e-4`.
    Log,
    /// Linear in the percent change from [`ValueScale::base`]; ticks are
    /// nice percent steps.
    Percent,
    /// Linear in `value / base × 100`; ticks are nice index steps.
    Indexed,
}

impl ScaleKind {
    /// Cycle linear → log → percent → linear (indexed goes back to linear).
    pub fn next(self) -> Self {
        match self {
            Self::Linear => Self::Log,
            Self::Log => Self::Percent,
            Self::Percent | Self::Indexed => Self::Linear,
        }
    }
}

/// Who owns the visible range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum RangeFit {
    /// The user: auto-fit is off. Zoom and pan switch to this.
    Manual,
    /// Fit the visible data.
    #[default]
    Auto,
    /// Fit the visible data plus the latest item (live following).
    Follow,
}

impl RangeFit {
    /// Cycle auto → follow → manual → auto.
    pub fn next(self) -> Self {
        match self {
            Self::Auto => Self::Follow,
            Self::Follow => Self::Manual,
            Self::Manual => Self::Auto,
        }
    }
    /// Auto and follow both fit the range.
    pub fn is_auto(self) -> bool {
        matches!(self, Self::Auto | Self::Follow)
    }
    /// Only follow tracks the latest item.
    pub fn is_follow(self) -> bool {
        matches!(self, Self::Follow)
    }
}

/// A value axis model.
///
/// ```
/// use uzor_plot::value_scale::{ScaleKind, ValueScale};
/// let s = ValueScale::new(0.0, 100.0);
/// assert_eq!(s.value_to_coord(75.0, 400.0), 100.0);
/// assert_eq!(s.coord_to_value(100.0, 400.0), 75.0);
/// let log = ValueScale::new(1.0, 100.0).with_kind(ScaleKind::Log);
/// assert!((log.value_to_coord(10.0, 400.0) - 200.0).abs() < 1e-9);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct ValueScale {
    /// Visible minimum (bottom edge).
    pub min: f64,
    /// Visible maximum (top edge).
    pub max: f64,
    /// Value mapping.
    pub kind: ScaleKind,
    /// Range ownership.
    pub fit: RangeFit,
    /// Reference value for [`ScaleKind::Percent`] and [`ScaleKind::Indexed`].
    pub base: f64,
    /// Fraction of the fitted data range added above and below. Default 0.04.
    pub fit_margin: f64,
    /// Grid density.
    pub density: TickDensity,
    /// Label decimals.
    pub labels: LabelPrecision,
}

impl Default for ValueScale {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            kind: ScaleKind::Linear,
            fit: RangeFit::Auto,
            base: 100.0,
            fit_margin: 0.04,
            density: TickDensity::default(),
            labels: LabelPrecision::default(),
        }
    }
}

const LOG_FLOOR: f64 = 0.0001;

impl ValueScale {
    /// A scale showing `[min, max]`, otherwise default.
    pub fn new(min: f64, max: f64) -> Self {
        Self { min, max, ..Default::default() }
    }
    /// Set the mapping.
    pub fn with_kind(mut self, kind: ScaleKind) -> Self {
        self.kind = kind;
        self
    }
    /// Set range ownership.
    pub fn with_fit(mut self, fit: RangeFit) -> Self {
        self.fit = fit;
        self
    }
    /// Set the percent/indexed reference (ignored unless positive).
    pub fn with_base(mut self, base: f64) -> Self {
        self.set_base(base);
        self
    }
    /// Set the auto-fit margin fraction.
    pub fn with_fit_margin(mut self, margin: f64) -> Self {
        self.fit_margin = margin;
        self
    }
    /// Set the grid density.
    pub fn with_density(mut self, density: TickDensity) -> Self {
        self.density = density;
        self
    }
    /// Set the label precision.
    pub fn with_labels(mut self, labels: LabelPrecision) -> Self {
        self.labels = labels;
        self
    }

    /// `max - min`.
    #[inline]
    pub fn range(&self) -> f64 {
        self.max - self.min
    }

    /// Set the percent/indexed reference; non-positive values are ignored.
    pub fn set_base(&mut self, base: f64) {
        if base > 0.0 {
            self.base = base;
        }
    }

    /// Percent change of `value` from the base (0 when the base is 0).
    #[inline]
    pub fn to_percent(&self, value: f64) -> f64 {
        if self.base == 0.0 {
            return 0.0;
        }
        ((value - self.base) / self.base) * 100.0
    }
    /// Value at `percent` change from the base.
    #[inline]
    pub fn from_percent(&self, percent: f64) -> f64 {
        self.base * (1.0 + percent / 100.0)
    }
    /// `value` indexed so the base reads 100 (0 when the base is 0).
    #[inline]
    pub fn to_indexed(&self, value: f64) -> f64 {
        if self.base == 0.0 {
            return 0.0;
        }
        value / self.base * 100.0
    }
    /// Value whose index is `index`.
    #[inline]
    pub fn from_indexed(&self, index: f64) -> f64 {
        self.base * index / 100.0
    }

    /// Value → pixel coordinate on an axis `length` px long (top = max).
    /// An empty, inverted or `NaN` range maps everything to the middle.
    #[inline]
    pub fn value_to_coord(&self, value: f64, length: f64) -> f64 {
        match self.kind {
            ScaleKind::Linear => {
                let range = self.max - self.min;
                if range.is_nan() || range <= 0.0 {
                    return length / 2.0;
                }
                length * (1.0 - (value - self.min) / range)
            }
            ScaleKind::Percent => {
                let p = self.to_percent(value);
                let p_min = self.to_percent(self.min);
                let p_max = self.to_percent(self.max);
                let range = p_max - p_min;
                if range.is_nan() || range <= 0.0 {
                    return length / 2.0;
                }
                length * (1.0 - (p - p_min) / range)
            }
            ScaleKind::Indexed => {
                let i = self.to_indexed(value);
                let i_min = self.to_indexed(self.min);
                let i_max = self.to_indexed(self.max);
                let range = i_max - i_min;
                if range.is_nan() || range <= 0.0 {
                    return length / 2.0;
                }
                length * (1.0 - (i - i_min) / range)
            }
            ScaleKind::Log => {
                let v = value.max(LOG_FLOOR);
                let lo = self.min.max(LOG_FLOOR);
                let hi = self.max.max(lo + LOG_FLOOR);
                let (lv, l0, l1) = (v.ln(), lo.ln(), hi.ln());
                let range = l1 - l0;
                if range.is_nan() || range <= 0.0 {
                    return length / 2.0;
                }
                length * (1.0 - (lv - l0) / range)
            }
        }
    }

    /// Pixel coordinate → value on an axis `length` px long.
    ///
    /// An axis with no usable length (zero, negative or non-finite) reads
    /// every coordinate as the middle of the range, mirroring
    /// [`Self::value_to_coord`] on an empty range, instead of returning
    /// `±inf` or `NaN`.
    #[inline]
    pub fn coord_to_value(&self, coord: f64, length: f64) -> f64 {
        let t = if length.is_finite() && length > 0.0 { coord / length } else { 0.5 };
        match self.kind {
            ScaleKind::Linear => {
                let range = self.max - self.min;
                self.max - t * range
            }
            ScaleKind::Percent => {
                let p_min = self.to_percent(self.min);
                let p_max = self.to_percent(self.max);
                let range = p_max - p_min;
                self.from_percent(p_max - t * range)
            }
            ScaleKind::Indexed => {
                let i_min = self.to_indexed(self.min);
                let i_max = self.to_indexed(self.max);
                let range = i_max - i_min;
                self.from_indexed(i_max - t * range)
            }
            ScaleKind::Log => {
                let lo = self.min.max(LOG_FLOOR);
                let hi = self.max.max(lo + LOG_FLOOR);
                let (l0, l1) = (lo.ln(), hi.ln());
                (l1 - t * (l1 - l0)).exp()
            }
        }
    }

    /// Linear grid step for an axis `length` px long.
    #[inline]
    pub fn step(&self, length: f64) -> f64 {
        self.density.step(self.range(), length)
    }

    /// Grid values (in data units) for the current kind, overscan included.
    ///
    /// Linear: nice steps of the value. Percent / Indexed: nice steps of
    /// the percent / index, mapped back. Log: nice steps of `log10(value)`.
    pub fn ticks(&self, length: f64) -> Vec<f64> {
        match self.kind {
            ScaleKind::Linear => self.density.ticks(self.min, self.max, length).1,
            ScaleKind::Percent => {
                let (a, b) = (self.to_percent(self.min), self.to_percent(self.max));
                self.mapped_ticks(a, b, length, |p| self.from_percent(p))
            }
            ScaleKind::Indexed => {
                let (a, b) = (self.to_indexed(self.min), self.to_indexed(self.max));
                self.mapped_ticks(a, b, length, |i| self.from_indexed(i))
            }
            ScaleKind::Log => {
                let lo = self.min.max(LOG_FLOOR);
                let hi = self.max.max(lo + LOG_FLOOR);
                self.mapped_ticks(lo.log10(), hi.log10(), length, |l| 10.0_f64.powf(l))
            }
        }
    }

    fn mapped_ticks(&self, a: f64, b: f64, length: f64, back: impl Fn(f64) -> f64) -> Vec<f64> {
        let range = b - a;
        let step = nice_step(range, self.density.target_ticks(length));
        let margin = self.density.overscan(range, length);
        step_ticks(a - margin, b + margin, step).into_iter().map(back).collect()
    }

    /// `value` formatted with the linear grid step and [`Self::labels`].
    pub fn format_value(&self, value: f64, length: f64) -> String {
        self.labels.format(value, self.step(length))
    }

    /// Axis label for `value` in the current kind: signed `+x.xx%` for
    /// percent, the index with 2 decimals for indexed, the value otherwise.
    pub fn format_label(&self, value: f64, length: f64) -> String {
        match self.kind {
            ScaleKind::Linear | ScaleKind::Log => self.format_value(value, length),
            ScaleKind::Percent => {
                let p = self.to_percent(value);
                if p >= 0.0 {
                    format!("+{:.2}%", p)
                } else {
                    format!("{:.2}%", p)
                }
            }
            ScaleKind::Indexed => format!("{:.2}", self.to_indexed(value)),
        }
    }

    /// Characters in the widest linear-grid label inside the range (plus the
    /// maximum's label), for sizing the axis text.
    pub fn max_label_len(&self, length: f64) -> usize {
        let step = self.step(length);
        if !(self.min.is_finite() && self.max.is_finite()) {
            return self.labels.format(self.max, step).len();
        }
        let first = (self.min / step).ceil() * step;
        // Same cap as the grid itself: a grid over MAX_TICKS is not drawn,
        // so only the maximum's label counts.
        let n = tick_loop_count(first, self.max, step, MAX_TICKS).unwrap_or(0);
        let mut max_len = 0;
        for i in 0..n {
            let v = first + (i as f64) * step;
            if v >= self.max {
                break;
            }
            max_len = max_len.max(self.labels.format(v, step).len());
        }
        max_len.max(self.labels.format(self.max, step).len())
    }

    /// Fit the range to `items[visible]` when [`Self::fit`] is auto, adding
    /// [`Self::fit_margin`] of the data range on both sides. Follow also
    /// includes the last item. `extent` gives an item's `(low, high)`.
    ///
    /// Nothing changes for an empty or inverted `visible`, empty `items`, or
    /// no finite extent. Returns whether the range was set.
    pub fn auto_fit<T>(
        &mut self,
        items: &[T],
        visible: Range<usize>,
        extent: impl Fn(&T) -> (f64, f64),
    ) -> bool {
        if !self.fit.is_auto() {
            return false;
        }
        let (start, end) = (visible.start, visible.end);
        if start >= end || items.is_empty() {
            return false;
        }
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for item in &items[start.min(items.len())..end.min(items.len())] {
            let (lo, hi) = extent(item);
            min = min.min(lo);
            max = max.max(hi);
        }
        if self.fit == RangeFit::Follow {
            if let Some(last) = items.last() {
                let (lo, hi) = extent(last);
                min = min.min(lo);
                max = max.max(hi);
            }
        }
        if min.is_finite() && max.is_finite() {
            let pad = (max - min) * self.fit_margin;
            self.min = min - pad;
            self.max = max + pad;
            true
        } else {
            false
        }
    }

    /// Smallest range zoom and clamped sets allow around `center`:
    /// `|center| × 1e-6` (the centre floored at `1e-10`).
    #[inline]
    pub fn min_range_at(center: f64) -> f64 {
        center.abs().max(1e-10) * 0.000001
    }

    /// Scale the range by `factor` (> 1 zooms out) keeping `center` at the
    /// same relative position. Switches to [`RangeFit::Manual`].
    pub fn zoom(&mut self, factor: f64, center: f64) {
        let current = self.range();
        let clamped = (current * factor).max(Self::min_range_at(center));
        let ratio = (center - self.min) / current.max(0.001);
        self.min = center - clamped * ratio;
        self.max = self.min + clamped;
        self.fit = RangeFit::Manual;
    }

    /// Shift the range by `delta`. Switches to [`RangeFit::Manual`].
    pub fn pan(&mut self, delta: f64) {
        self.min += delta;
        self.max += delta;
        self.fit = RangeFit::Manual;
    }

    /// Set the range unless it is narrower than [`Self::min_range_at`] its
    /// centre or either end is not finite. Switches to [`RangeFit::Manual`]
    /// when applied.
    pub fn set_range_clamped(&mut self, min: f64, max: f64) -> bool {
        if !(min.is_finite() && max.is_finite()) {
            return false;
        }
        let center = (min + max) / 2.0;
        if max - min < Self::min_range_at(center) {
            return false;
        }
        self.min = min;
        self.max = max;
        self.fit = RangeFit::Manual;
        true
    }
}
