//! Calendar-aware time axis over a [`SlotViewport`].
//!
//! Items sit in evenly spaced slots (one per bar); their timestamps can be
//! regular or not. [`TimeScale::generate_ticks`] picks grid lines on round
//! calendar boundaries for the zoom level:
//!
//! - zoomed out: years → months → days (1st of month, then every 2/3/4/5
//!   days from the 1st via per-month tables, so cells stay even);
//! - zoomed in: 12h → 4h → 1h → 30m → 5m → 1m → 30s → 10s → 5s → 1s, with
//!   day boundaries kept for context;
//! - irregular series (gaps far over the median, or forced): calendar
//!   boundaries snapped to the nearest real item, falling back to per-item
//!   weights (the Lightweight Charts `weightByTime` scheme).
//!
//! Labels go through a [`TimeLabeler`]; [`CalendarLabels`] covers the usual
//! day/month orders, 12/24h and weekday prefixes with caller-supplied
//! month and weekday names. Times are Unix seconds, UTC; shift them before
//! calling for another zone.

use crate::viewport::SlotViewport;

/// One second.
pub const SECOND: i64 = 1;
/// Seconds in a minute.
pub const MINUTE: i64 = 60;
/// Seconds in an hour.
pub const HOUR: i64 = 3600;
/// Seconds in a day.
pub const DAY: i64 = 86400;

/// Something placed on a time axis, with a Unix time in seconds.
pub trait Timed {
    /// Unix time, seconds.
    fn time(&self) -> i64;
}

impl Timed for i64 {
    #[inline]
    fn time(&self) -> i64 {
        *self
    }
}

impl<V> Timed for (i64, V) {
    #[inline]
    fn time(&self) -> i64 {
        self.0
    }
}

/// Importance of a time tick, from the calendar boundary it falls on.
/// Higher is more important (year > month > day > hours > minutes > seconds).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum TickWeight {
    /// Inside one second.
    LessThanSecond = 1,
    /// Second boundary.
    #[default]
    Second = 5,
    /// Minute boundary.
    Minute1 = 10,
    /// 5-minute boundary.
    Minute5 = 15,
    /// 30-minute boundary.
    Minute30 = 20,
    /// Hour boundary.
    Hour = 30,
    /// 3-hour boundary.
    Hour3 = 31,
    /// 4-hour boundary.
    Hour4 = 35,
    /// 6-hour boundary.
    Hour6 = 36,
    /// 12-hour boundary.
    Hour12 = 37,
    /// Midnight.
    Day = 50,
    /// First of a month.
    Month = 60,
    /// First of a year.
    Year = 70,
}

impl TickWeight {
    /// Year or month.
    pub fn is_major(&self) -> bool {
        matches!(self, TickWeight::Year | TickWeight::Month)
    }
    /// Day.
    pub fn is_medium(&self) -> bool {
        matches!(self, TickWeight::Day)
    }
}

/// A chosen time tick.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TimeTick {
    /// Slot index (negative or past the data when extrapolated).
    pub slot: i64,
    /// Pixel x.
    pub x: f64,
    /// Boundary weight, for styling.
    pub weight: TickWeight,
    /// Label text.
    pub label: String,
}

/// Time tick generator. Stateless today; kept as a type for settings.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct TimeScale {}

impl TimeScale {
    /// A time scale with default settings.
    pub fn new() -> Self {
        Self::default()
    }
}


/// Days in each month (non-leap year)
const DAYS_IN_MONTH: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// Check if year is a leap year
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Get days in a specific month (1-indexed, handles leap years).
///
/// A `month` outside `1..=12` has no days: returns 0 instead of panicking.
pub fn days_in_month(year: i32, month: i32) -> i32 {
    if !(1..=12).contains(&month) {
        0
    } else if month == 2 && is_leap_year(year) {
        29
    } else {
        DAYS_IN_MONTH[(month - 1) as usize]
    }
}

/// Earliest year [`timestamp_to_date`] returns (`i32::MIN + 1`, so a caller
/// stepping one year back cannot overflow).
pub const MIN_DATE_YEAR: i32 = i32::MIN + 1;
/// Latest year [`timestamp_to_date`] returns (`i32::MAX - 1`, so a caller
/// stepping one year forward cannot overflow).
pub const MAX_DATE_YEAR: i32 = i32::MAX - 1;
/// First second of [`MIN_DATE_YEAR`]; earlier timestamps saturate to it.
pub const MIN_DATE_TIMESTAMP: i64 = days_from_civil(MIN_DATE_YEAR as i64, 1, 1) * DAY;
/// Last second of [`MAX_DATE_YEAR`]; later timestamps saturate to it.
pub const MAX_DATE_TIMESTAMP: i64 = days_from_civil(MAX_DATE_YEAR as i64, 12, 31) * DAY + DAY - 1;

/// Days from 1970-01-01 to `year-month-day` in the proleptic Gregorian
/// calendar (Howard Hinnant's `days_from_civil`). `month` must be 1..=12;
/// exact for any `i32` year in `i64`.
const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400; // [0, 399]
    let mp = if month > 2 { month - 3 } else { month + 9 }; // March = 0
    let doy = (153 * mp + 2) / 5 + day - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// `(year, month 1-12, day 1-31)` for days from 1970-01-01 (Howard
/// Hinnant's `civil_from_days`).
const fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11], March = 0
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

/// Convert Unix timestamp (seconds, UTC) to date components.
/// Returns: (year, month 1-12, day 1-31, hour 0-23, minute 0-59, second 0-59)
///
/// Closed form, constant time. Timestamps outside
/// [`MIN_DATE_TIMESTAMP`]`..=`[`MAX_DATE_TIMESTAMP`] (years
/// [`MIN_DATE_YEAR`]`..=`[`MAX_DATE_YEAR`]) saturate to the nearest end.
pub fn timestamp_to_date(ts: i64) -> (i32, i32, i32, i32, i32, i32) {
    let ts = ts.clamp(MIN_DATE_TIMESTAMP, MAX_DATE_TIMESTAMP);
    let time_of_day = ts.rem_euclid(DAY);
    let hour = (time_of_day / HOUR) as i32;
    let minute = ((time_of_day % HOUR) / MINUTE) as i32;
    let second = (time_of_day % MINUTE) as i32;

    let (year, month, day) = civil_from_days(ts.div_euclid(DAY));
    (year as i32, month as i32, day as i32, hour, minute, second)
}

/// Convert date components to Unix timestamp (seconds, UTC).
/// month: 1-12, day: 1-31
///
/// Closed form, constant time, exact for every `i32` year (computed in
/// `i64`, which cannot overflow for `i32` inputs). Out-of-range fields
/// roll over linearly: a `day`, `hour`, `minute` or `second` past its
/// range spills into the next unit; a month below 1 counts as January and
/// a month above 12 as January of the next year.
pub fn date_to_timestamp(year: i32, month: i32, day: i32, hour: i32, minute: i32, second: i32) -> i64 {
    let (y, m) = if month > 12 {
        (year as i64 + 1, 1)
    } else if month < 1 {
        (year as i64, 1)
    } else {
        (year as i64, month as i64)
    };
    let days = days_from_civil(y, m, 1) + (day as i64 - 1);
    days * DAY + hour as i64 * HOUR + minute as i64 * MINUTE + second as i64
}

/// LWC intraday divisor table (ms-per-boundary, ascending) — verbatim from
/// `lightweight-charts/src/model/horz-scale-behavior-time/time-scale-point-weight-generator.ts`.
const INTRADAY_DIVISORS: &[(i64, TickWeight)] = &[
    (1_000,       TickWeight::Second),
    (60_000,      TickWeight::Minute1),
    (300_000,     TickWeight::Minute5),
    (1_800_000,   TickWeight::Minute30),
    (3_600_000,   TickWeight::Hour),
    (10_800_000,  TickWeight::Hour3),
    (21_600_000,  TickWeight::Hour6),
    (43_200_000,  TickWeight::Hour12),
];

/// Weight for a bar given its and the previous bar's timestamps (UTC ms).
///
/// Returns the highest calendar boundary that was actually crossed between
/// `prev_ms` and `cur_ms`. This is the LWC `weightByTime` function in Rust.
pub fn weight_by_time(cur_ms: i64, prev_ms: i64) -> TickWeight {
    let (cy, cm, cd, _, _, _) = timestamp_to_date(cur_ms / 1000);
    let (py, pm, pd, _, _, _) = timestamp_to_date(prev_ms / 1000);
    if cy != py { return TickWeight::Year; }
    if cm != pm { return TickWeight::Month; }
    if cd != pd { return TickWeight::Day; }
    for (div, w) in INTRADAY_DIVISORS.iter().rev() {
        if cur_ms / div != prev_ms / div {
            return *w;
        }
    }
    TickWeight::LessThanSecond
}

/// Whether the last (up to 100) gaps are irregular: the largest positive
/// gap exceeds twice the median one.
fn is_non_uniform<T: Timed>(bars: &[T]) -> bool {
    let n = bars.len();
    if n < 3 { return false; }
    let sample = n.min(100);
    let start = n - sample;
    let mut gaps: Vec<i64> = Vec::with_capacity(sample - 1);
    for i in (start + 1)..n {
        let g = (bars[i].time() - bars[i - 1].time()).abs();
        if g > 0 { gaps.push(g); }
    }
    if gaps.len() < 2 { return false; }
    gaps.sort_unstable();
    let median = gaps[gaps.len() / 2];
    let max = *gaps.last().unwrap();
    median > 0 && max > median.saturating_mul(2)
}

/// Fill per-bar weights given a sorted slice of `(timestamp_secs, weight)`
/// tuples. `start` is the first index to (re-)compute (0 = full bulk pass,
/// `N` = incremental tail re-weight after appending bars). Weights are
/// written into `points[i].1`.
///
/// Mirrors LWC `fillWeightsForPoints` — first point gets a weight estimated
/// from the average interval, and downstream points compare against their
/// real predecessor.
#[allow(clippy::needless_range_loop)]
pub fn fill_weights(points: &mut [(i64, TickWeight)], start: usize) {
    if points.is_empty() { return; }
    let mut prev_ms: Option<i64> = if start > 0 {
        Some(points[start - 1].0 * 1000)
    } else {
        None
    };
    let mut total_diff: i64 = 0;
    let mut diff_count: i64 = 0;

    for i in start..points.len() {
        let cur_ms = points[i].0 * 1000;
        if let Some(p) = prev_ms {
            points[i].1 = weight_by_time(cur_ms, p);
            total_diff += cur_ms - p;
            diff_count += 1;
        }
        prev_ms = Some(cur_ms);
    }

    if start == 0 && points.len() > 1 && diff_count > 0 {
        let avg = (total_diff + diff_count - 1) / diff_count;
        let approx_prev_ms = points[0].0 * 1000 - avg;
        points[0].1 = weight_by_time(points[0].0 * 1000, approx_prev_ms);
    }
}

impl TimeScale {
    /// Convert bar index to timestamp using real bar data where available.
    ///
    /// For indices within `[0..bars.len())` returns the actual bar timestamp.
    /// For indices outside that range extrapolates from the nearest edge using `bar_interval`.
    fn bar_idx_to_timestamp<T: Timed>(idx: i64, bars: &[T], first_ts: i64, bar_interval: i64) -> i64 {
        if bars.is_empty() {
            return first_ts + idx * bar_interval;
        }
        if idx >= 0 && (idx as usize) < bars.len() {
            return bars[idx as usize].time();
        }
        if idx < 0 {
            first_ts + idx * bar_interval
        } else {
            let last_ts = bars[bars.len() - 1].time();
            let beyond = idx - (bars.len() as i64 - 1);
            last_ts + beyond * bar_interval
        }
    }

    /// Convert timestamp to bar index using real bar data where available.
    ///
    /// For timestamps within the data range performs a binary search.
    /// For timestamps outside the range extrapolates from the nearest edge.
    fn timestamp_to_bar_idx<T: Timed>(ts: i64, bars: &[T], first_ts: i64, bar_interval: i64) -> i64 {
        if bars.is_empty() || bar_interval <= 0 {
            return (ts - first_ts) / bar_interval.max(1);
        }
        let last_ts = bars[bars.len() - 1].time();
        if ts < first_ts {
            (ts - first_ts) / bar_interval
        } else if ts > last_ts {
            let beyond_secs = ts - last_ts;
            (bars.len() as i64 - 1) + beyond_secs / bar_interval
        } else {
            match bars.binary_search_by_key(&ts, |b| b.time()) {
                Ok(idx) => idx as i64,
                Err(idx) => idx as i64,
            }
        }
    }

    /// Nearest real slot to `ts`, clamped to the data (never extrapolates).
    fn timestamp_to_bar_idx_nearest_clamped<T: Timed>(ts: i64, bars: &[T]) -> usize {
        if bars.is_empty() {
            return 0;
        }
        let last = bars.len() - 1;
        if ts <= bars[0].time() {
            return 0;
        }
        if ts >= bars[last].time() {
            return last;
        }
        match bars.binary_search_by_key(&ts, |b| b.time()) {
            Ok(idx) => idx,
            Err(idx) => {
                let before = idx - 1;
                let after = idx;
                let d_before = (ts - bars[before].time()).abs();
                let d_after = (bars[after].time() - ts).abs();
                if d_before <= d_after { before } else { after }
            }
        }
    }

    /// Grid cell constraints (in pixels)
    const MIN_CELL_WIDTH: f64 = 50.0;
    const MAX_CELL_WIDTH: f64 = 250.0;

    /// Allowed day steps for grid
    const DAY_STEPS: [i32; 5] = [1, 2, 3, 4, 5];

    /// Tick days for each (month_days, step) combination
    /// Format: array of day numbers to show ticks on
    /// The gap from last tick to 1st of next month is the "cheat" cell
    const TICKS_31_STEP1: [i32; 31] = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31];
    const TICKS_31_STEP2: [i32; 15] = [1,3,5,7,9,11,13,15,17,19,21,23,25,27,29];
    const TICKS_31_STEP3: [i32; 10] = [1,4,7,10,13,16,19,22,25,28];
    const TICKS_31_STEP4: [i32; 8] = [1,5,9,13,17,21,25,29];
    const TICKS_31_STEP5: [i32; 6] = [1,6,11,16,21,26];

    const TICKS_30_STEP1: [i32; 30] = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30];
    const TICKS_30_STEP2: [i32; 15] = [1,3,5,7,9,11,13,15,17,19,21,23,25,27,29];
    const TICKS_30_STEP3: [i32; 10] = [1,4,7,10,13,16,19,22,25,28];
    const TICKS_30_STEP4: [i32; 8] = [1,5,9,13,17,21,25,29];
    const TICKS_30_STEP5: [i32; 6] = [1,6,11,16,21,26];

    const TICKS_29_STEP1: [i32; 29] = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29];
    const TICKS_29_STEP2: [i32; 15] = [1,3,5,7,9,11,13,15,17,19,21,23,25,27,29];
    const TICKS_29_STEP3: [i32; 10] = [1,4,7,10,13,16,19,22,25,28];
    const TICKS_29_STEP4: [i32; 8] = [1,5,9,13,17,21,25,29];
    const TICKS_29_STEP5: [i32; 6] = [1,6,11,16,21,26];

    const TICKS_28_STEP1: [i32; 28] = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28];
    const TICKS_28_STEP2: [i32; 14] = [1,3,5,7,9,11,13,15,17,19,21,23,25,27];
    const TICKS_28_STEP3: [i32; 10] = [1,4,7,10,13,16,19,22,25,28];
    const TICKS_28_STEP4: [i32; 7] = [1,5,9,13,17,21,25];
    const TICKS_28_STEP5: [i32; 6] = [1,6,11,16,21,26];

    /// Get tick days for a given month length and step
    fn get_tick_days(month_days: i32, step: i32) -> &'static [i32] {
        match (month_days, step) {
            (31, 1) => &Self::TICKS_31_STEP1,
            (31, 2) => &Self::TICKS_31_STEP2,
            (31, 3) => &Self::TICKS_31_STEP3,
            (31, 4) => &Self::TICKS_31_STEP4,
            (31, 5) => &Self::TICKS_31_STEP5,
            (30, 1) => &Self::TICKS_30_STEP1,
            (30, 2) => &Self::TICKS_30_STEP2,
            (30, 3) => &Self::TICKS_30_STEP3,
            (30, 4) => &Self::TICKS_30_STEP4,
            (30, 5) => &Self::TICKS_30_STEP5,
            (29, 1) => &Self::TICKS_29_STEP1,
            (29, 2) => &Self::TICKS_29_STEP2,
            (29, 3) => &Self::TICKS_29_STEP3,
            (29, 4) => &Self::TICKS_29_STEP4,
            (29, 5) => &Self::TICKS_29_STEP5,
            (28, 1) => &Self::TICKS_28_STEP1,
            (28, 2) => &Self::TICKS_28_STEP2,
            (28, 3) => &Self::TICKS_28_STEP3,
            (28, 4) => &Self::TICKS_28_STEP4,
            (28, 5) => &Self::TICKS_28_STEP5,
            (_, 1) => &Self::TICKS_31_STEP1,
            (_, 2) => &Self::TICKS_31_STEP2,
            (_, 3) => &Self::TICKS_31_STEP3,
            (_, 4) => &Self::TICKS_31_STEP4,
            _ => &Self::TICKS_31_STEP5,
        }
    }

    /// Time ticks for the visible part of `items` (sorted by time), sorted by x.
    ///
    /// - `measure_text` gives a label's pixel width; labels are spaced so
    ///   they never overlap and neighbours never repeat the same text.
    /// - `labels` formats each tick.
    /// - `irregular` forces the calendar-grid path for series whose items
    ///   are not on a time cadence (they still need real timestamps).
    ///   Series with gaps over twice the median take that path on their own.
    ///
    /// Returns no ticks for no items, or for a viewport whose spacing is not
    /// positive and finite or whose start is not finite.
    ///
    /// Regular series measure how many pixels a day spans: over 200 px use
    /// intraday steps, otherwise day steps from the 1st of each month (or
    /// months only once three or more month starts are visible).
    pub fn generate_ticks<T: Timed, F>(
        &self,
        viewport: &SlotViewport,
        items: &[T],
        measure_text: F,
        labels: &dyn TimeLabeler,
        irregular: bool,
    ) -> Vec<TimeTick>
    where
        F: Fn(&str) -> f64,
    {
        let bars = items;
        let mut ticks = Vec::new();

        // No ticks for no data or a viewport that cannot place a slot
        // (spacing not positive and finite, start not finite).
        let usable_viewport = viewport.spacing.is_finite()
            && viewport.spacing > 0.0
            && viewport.start.is_finite();
        if bars.is_empty() || !usable_viewport {
            return ticks;
        }

        if irregular {
            self.generate_calendar_grid_ticks(
                &mut ticks,
                bars,
                viewport,
                &measure_text,
                labels,
            );
            if ticks.is_empty() {
                self.generate_per_bar_ticks(
                    &mut ticks,
                    bars,
                    viewport,
                    &measure_text,
                    labels,
                );
            }
            ticks.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
            return ticks;
        }

        let non_uniform = bars.len() >= 3 && is_non_uniform(bars);
        if non_uniform {
            self.generate_calendar_grid_ticks(
                &mut ticks,
                bars,
                viewport,
                &measure_text,
                labels,
            );
            if ticks.is_empty() {
                self.generate_per_bar_ticks(
                    &mut ticks,
                    bars,
                    viewport,
                    &measure_text,
                    labels,
                );
            }
            ticks.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
            return ticks;
        }

        let bar_interval = if bars.len() >= 2 {
            let sample_count = bars.len().min(10);
            let sample_start = bars.len() - sample_count;
            let total_time = bars[bars.len() - 1].time() - bars[sample_start].time();
            (total_time / (sample_count - 1) as i64).max(1)
        } else {
            HOUR
        };

        let first_ts = bars[0].time();

        let visible_bars = viewport.visible_slots();
        let start_bar = viewport.start.floor() as i64;
        let end_bar = (viewport.start + visible_bars as f64).ceil() as i64;

        let start_ts = Self::bar_idx_to_timestamp(start_bar - 50, bars, first_ts, bar_interval);
        let end_ts = Self::bar_idx_to_timestamp(end_bar + 50, bars, first_ts, bar_interval);

        let bars_per_day = DAY / bar_interval;
        let day_width_px = bars_per_day as f64 * viewport.spacing;

        if day_width_px > 4.0 * Self::MIN_CELL_WIDTH {
            self.generate_intraday_ticks(
                &mut ticks,
                start_ts, end_ts,
                bars, first_ts, bar_interval,
                viewport,
                day_width_px,
                &measure_text,
                labels,
            );
        } else {
            self.generate_daily_ticks(
                &mut ticks,
                start_ts, end_ts,
                bars, first_ts, bar_interval,
                viewport,
                day_width_px,
                &measure_text,
                labels,
            );
        }

        ticks.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        ticks
    }

    /// One candidate per visible item, weighted by the boundary crossed since
    /// the previous item (`weight_by_time`), then picked by weight and gap.
    fn generate_per_bar_ticks<T: Timed, F>(
        &self,
        ticks: &mut Vec<TimeTick>,
        bars: &[T],
        viewport: &SlotViewport,
        measure_text: &F,
        labels: &dyn TimeLabeler,
    ) where
        F: Fn(&str) -> f64,
    {
        let visible_bars = viewport.visible_slots() as i64;
        let start_bar = ((viewport.start.floor() as i64) - 50).max(0);
        let end_bar = ((viewport.start as i64) + visible_bars + 50).min(bars.len() as i64);

        if end_bar <= start_bar {
            return;
        }

        let lead = (start_bar - 1).max(0) as usize;
        let from = lead;
        let to = end_bar as usize;
        let mut pts: Vec<(i64, TickWeight)> = bars[from..to]
            .iter()
            .map(|b| (b.time(), TickWeight::Second))
            .collect();
        fill_weights(&mut pts, if from == 0 { 0 } else { 1 });

        let mut candidates: Vec<(i64, f64, TickWeight, i64)> =
            Vec::with_capacity(pts.len());
        for (i_local, (ts, weight)) in pts.iter().enumerate() {
            let bar_idx = (from + i_local) as i64;
            let x = viewport.slot_to_x(bar_idx as usize);
            if x < -100.0 || x > viewport.width + 100.0 {
                continue;
            }
            candidates.push((bar_idx, x, *weight, *ts));
        }

        self.select_ticks_with_spacing(ticks, candidates, viewport, measure_text, labels);
    }

    /// Round calendar boundaries for irregular series, each snapped to the
    /// nearest real item. Density comes from the median gap over the visible
    /// slice. Past the last item, boundaries extrapolate forward by that
    /// median so the grid continues into empty space on the right.
    fn generate_calendar_grid_ticks<T: Timed, F>(
        &self,
        ticks: &mut Vec<TimeTick>,
        bars: &[T],
        viewport: &SlotViewport,
        measure_text: &F,
        labels: &dyn TimeLabeler,
    ) where
        F: Fn(&str) -> f64,
    {
        if bars.len() < 2 {
            return;
        }

        let visible_bars = viewport.visible_slots() as i64;
        let start_bar = ((viewport.start.floor() as i64) - 50)
            .max(0)
            .min(bars.len() as i64 - 1) as usize;
        let end_bar = (((viewport.start as i64) + visible_bars + 50) as usize)
            .min(bars.len() - 1);
        if end_bar <= start_bar {
            return;
        }

        let mut gaps: Vec<i64> = Vec::with_capacity(end_bar - start_bar);
        for i in (start_bar + 1)..=end_bar {
            let g = bars[i].time() - bars[i - 1].time();
            if g > 0 { gaps.push(g); }
        }
        if gaps.is_empty() {
            return;
        }
        gaps.sort_unstable();
        let median_interval = gaps[gaps.len() / 2].max(1);

        let start_ts = bars[start_bar].time();
        let mut end_ts = bars[end_bar].time();
        if end_ts <= start_ts {
            return;
        }

        let last_real_idx = bars.len() as i64 - 1;
        let end_bar_vp = (viewport.start + visible_bars as f64).ceil() as i64 + 50;
        if end_bar_vp > last_real_idx {
            let beyond = end_bar_vp - last_real_idx;
            let extrapolated_end_ts = bars[last_real_idx as usize].time() + beyond * median_interval;
            if extrapolated_end_ts > end_ts {
                end_ts = extrapolated_end_ts;
            }
        }

        let bars_per_day = (DAY / median_interval).max(1);
        let day_width_px = bars_per_day as f64 * viewport.spacing;

        let mut boundaries: Vec<(i64, TickWeight)> = Vec::new();

        if day_width_px > 4.0 * Self::MIN_CELL_WIDTH {
            let (time_interval, weight) = if day_width_px > 86400.0 * Self::MIN_CELL_WIDTH {
                (SECOND, TickWeight::Second)
            } else if day_width_px > 17280.0 * Self::MIN_CELL_WIDTH {
                (5 * SECOND, TickWeight::Second)
            } else if day_width_px > 8640.0 * Self::MIN_CELL_WIDTH {
                (10 * SECOND, TickWeight::Second)
            } else if day_width_px > 2880.0 * Self::MIN_CELL_WIDTH {
                (30 * SECOND, TickWeight::Second)
            } else if day_width_px > 1440.0 * Self::MIN_CELL_WIDTH {
                (MINUTE, TickWeight::Minute1)
            } else if day_width_px > 288.0 * Self::MIN_CELL_WIDTH {
                (5 * MINUTE, TickWeight::Minute5)
            } else if day_width_px > 48.0 * Self::MIN_CELL_WIDTH {
                (30 * MINUTE, TickWeight::Minute30)
            } else if day_width_px > 24.0 * Self::MIN_CELL_WIDTH {
                (HOUR, TickWeight::Hour)
            } else if day_width_px > 6.0 * Self::MIN_CELL_WIDTH {
                (4 * HOUR, TickWeight::Hour4)
            } else {
                (12 * HOUR, TickWeight::Hour4)
            };

            let (mut year, mut month, mut day, _, _, _) = timestamp_to_date(start_ts);
            let mut day_ts = date_to_timestamp(year, month, day, 0, 0, 0);
            if day_ts > start_ts {
                day -= 1;
                if day < 1 {
                    month -= 1;
                    if month < 1 { year -= 1; month = 12; }
                    day = days_in_month(year, month);
                }
                day_ts = date_to_timestamp(year, month, day, 0, 0, 0);
            }
            while day_ts <= end_ts {
                let w = if day == 1 { TickWeight::Month } else { TickWeight::Day };
                boundaries.push((day_ts, w));
                day += 1;
                if day > days_in_month(year, month) {
                    day = 1;
                    month += 1;
                    if month > 12 { month = 1; year += 1; }
                }
                day_ts = date_to_timestamp(year, month, day, 0, 0, 0);
            }

            const DAY_SEC: i64 = 86_400;
            let first_boundary = (start_ts / time_interval) * time_interval;
            let mut boundary_ts = first_boundary;
            while boundary_ts <= end_ts {
                if boundary_ts.rem_euclid(DAY_SEC) != 0 {
                    boundaries.push((boundary_ts, weight));
                }
                boundary_ts += time_interval;
            }
        } else {
            let mut step_days: i32 = 0;
            for &step in Self::DAY_STEPS.iter().rev() {
                let cell_width = day_width_px * step as f64;
                if (Self::MIN_CELL_WIDTH..=Self::MAX_CELL_WIDTH).contains(&cell_width) {
                    step_days = step;
                    break;
                }
            }
            if step_days == 0 {
                let max_step_width = day_width_px * 5.0;
                step_days = if max_step_width < Self::MIN_CELL_WIDTH { 0 } else { 5 };
            }

            if step_days == 0 {
                let (mut year, mut month, _, _, _, _) = timestamp_to_date(start_ts);
                loop {
                    let ts = date_to_timestamp(year, month, 1, 0, 0, 0);
                    if ts > end_ts { break; }
                    if ts >= start_ts - 31 * DAY {
                        boundaries.push((ts, TickWeight::Month));
                    }
                    month += 1;
                    if month > 12 { month = 1; year += 1; }
                }
            } else {
                let (mut year, mut month, _, _, _, _) = timestamp_to_date(start_ts);
                month -= 1;
                if month < 1 { year -= 1; month = 12; }
                loop {
                    let month_start = date_to_timestamp(year, month, 1, 0, 0, 0);
                    if month_start > end_ts + 31 * DAY { break; }
                    if month_start >= start_ts - 31 * DAY {
                        let month_days = days_in_month(year, month);
                        let tick_days = Self::get_tick_days(month_days, step_days);
                        for &day in tick_days {
                            let ts = date_to_timestamp(year, month, day, 0, 0, 0);
                            if ts < start_ts - DAY || ts > end_ts + DAY { continue; }
                            let w = if day == 1 { TickWeight::Month } else { TickWeight::Day };
                            boundaries.push((ts, w));
                        }
                    }
                    month += 1;
                    if month > 12 { month = 1; year += 1; }
                }
            }
        }

        if boundaries.is_empty() {
            return;
        }

        boundaries.sort_by_key(|&(ts, _)| ts);
        let last_ts = bars[end_bar].time();
        let mut by_bar: Vec<(i64, i64, TickWeight)> = Vec::with_capacity(boundaries.len());
        for (ts, weight) in boundaries {
            let idx: i64 = if ts > last_ts {
                end_bar as i64 + (ts - last_ts) / median_interval
            } else {
                Self::timestamp_to_bar_idx_nearest_clamped(ts, &bars[..=end_bar]) as i64
            };
            if idx < start_bar as i64 {
                continue;
            }
            match by_bar.last_mut() {
                Some(last) if last.0 == idx => {
                    if weight > last.2 {
                        last.1 = ts;
                        last.2 = weight;
                    }
                }
                _ => by_bar.push((idx, ts, weight)),
            }
        }

        let candidates: Vec<(i64, f64, TickWeight, i64)> = by_bar
            .into_iter()
            .map(|(idx, ts, weight)| {
                let x = viewport.pos_to_x(idx as f64);
                (idx, x, weight, ts)
            })
            .collect();

        self.select_ticks_with_spacing(ticks, candidates, viewport, measure_text, labels);
    }

    /// Generate intraday ticks (hours, minutes) for high zoom levels
    #[allow(clippy::too_many_arguments)]
    fn generate_intraday_ticks<T: Timed, F>(
        &self,
        ticks: &mut Vec<TimeTick>,
        start_ts: i64,
        end_ts: i64,
        bars: &[T],
        first_ts: i64,
        bar_interval: i64,
        viewport: &SlotViewport,
        day_width_px: f64,
        measure_text: &F,
        labels: &dyn TimeLabeler,
    ) where
        F: Fn(&str) -> f64,
    {
        let (time_interval, weight) = if day_width_px > 86400.0 * Self::MIN_CELL_WIDTH {
            (SECOND, TickWeight::Second)
        } else if day_width_px > 17280.0 * Self::MIN_CELL_WIDTH {
            (5 * SECOND, TickWeight::Second)
        } else if day_width_px > 8640.0 * Self::MIN_CELL_WIDTH {
            (10 * SECOND, TickWeight::Second)
        } else if day_width_px > 2880.0 * Self::MIN_CELL_WIDTH {
            (30 * SECOND, TickWeight::Second)
        } else if day_width_px > 1440.0 * Self::MIN_CELL_WIDTH {
            (MINUTE, TickWeight::Minute1)
        } else if day_width_px > 288.0 * Self::MIN_CELL_WIDTH {
            (5 * MINUTE, TickWeight::Minute5)
        } else if day_width_px > 48.0 * Self::MIN_CELL_WIDTH {
            (30 * MINUTE, TickWeight::Minute30)
        } else if day_width_px > 24.0 * Self::MIN_CELL_WIDTH {
            (HOUR, TickWeight::Hour)
        } else if day_width_px > 6.0 * Self::MIN_CELL_WIDTH {
            (4 * HOUR, TickWeight::Hour4)
        } else {
            (12 * HOUR, TickWeight::Hour4)
        };

        let mut candidates: Vec<(i64, f64, TickWeight, i64)> = Vec::new();

        self.add_day_boundaries(&mut candidates, start_ts, end_ts, bars, first_ts, bar_interval, viewport);

        let first_boundary = (start_ts / time_interval) * time_interval;
        let mut boundary_ts = first_boundary;
        while boundary_ts <= end_ts {
            let bar_idx = Self::timestamp_to_bar_idx(boundary_ts, bars, first_ts, bar_interval);
            let x = viewport.pos_to_x(bar_idx as f64);

            if x >= -100.0 && x <= viewport.width + 100.0 {
                const DAY_SEC_I: i64 = 86_400;
                if boundary_ts.rem_euclid(DAY_SEC_I) != 0 {
                    candidates.push((bar_idx, x, weight, boundary_ts));
                }
            }
            boundary_ts += time_interval;
        }

        self.select_ticks_with_spacing(ticks, candidates, viewport, measure_text, labels);
    }

    /// Generate daily ticks with uniform grid cells using lookup tables
    ///
    /// Algorithm:
    /// 1. Find all months in/near viewport
    /// 2. For each month, use the pre-computed tick table for (month_days, step)
    /// 3. Each month has uniform spacing with "cheat" cell at end (last_tick → 1st of next month)
    #[allow(clippy::too_many_arguments)]
    fn generate_daily_ticks<T: Timed, F>(
        &self,
        ticks: &mut Vec<TimeTick>,
        start_ts: i64,
        end_ts: i64,
        bars: &[T],
        first_ts: i64,
        bar_interval: i64,
        viewport: &SlotViewport,
        day_width_px: f64,
        measure_text: &F,
        labels: &dyn TimeLabeler,
    ) where
        F: Fn(&str) -> f64,
    {
        let mut candidates: Vec<(i64, f64, TickWeight, i64)> = Vec::new();

        let (mut year, mut month, _, _, _, _) = timestamp_to_date(start_ts);
        month -= 1;
        if month < 1 { year -= 1; month = 12; }

        let mut months_in_range: Vec<(i32, i32)> = Vec::new();
        loop {
            let ts = date_to_timestamp(year, month, 1, 0, 0, 0);
            if ts > end_ts + 31 * DAY { break; }
            if ts >= start_ts - 31 * DAY {
                months_in_range.push((year, month));
            }
            month += 1;
            if month > 12 { month = 1; year += 1; }
        }

        let visible_month_count = months_in_range.iter().filter(|&&(y, m)| {
            let ts = date_to_timestamp(y, m, 1, 0, 0, 0);
            let bar_idx = Self::timestamp_to_bar_idx(ts, bars, first_ts, bar_interval);
            if bar_idx < 0 { return false; }
            let x = viewport.pos_to_x(bar_idx as f64);
            x >= 0.0 && x <= viewport.width
        }).count();

        if visible_month_count >= 3 {
            for &(y, m) in &months_in_range {
                let ts = date_to_timestamp(y, m, 1, 0, 0, 0);
                let bar_idx = Self::timestamp_to_bar_idx(ts, bars, first_ts, bar_interval);
                let x = viewport.pos_to_x(bar_idx as f64);
                if x >= -50.0 && x <= viewport.width + 50.0 {
                    candidates.push((bar_idx, x, TickWeight::Month, ts));
                }
            }
            self.select_ticks_with_spacing(ticks, candidates, viewport, measure_text, labels);
            return;
        }

        let mut step_days: i32 = 0;
        for &step in Self::DAY_STEPS.iter().rev() {
            let cell_width = day_width_px * step as f64;
            if (Self::MIN_CELL_WIDTH..=Self::MAX_CELL_WIDTH).contains(&cell_width) {
                step_days = step;
                break;
            }
        }

        if step_days == 0 {
            let max_step_width = day_width_px * 5.0;
            if max_step_width < Self::MIN_CELL_WIDTH {
                for &(y, m) in &months_in_range {
                    let ts = date_to_timestamp(y, m, 1, 0, 0, 0);
                    let bar_idx = Self::timestamp_to_bar_idx(ts, bars, first_ts, bar_interval);
                    let x = viewport.pos_to_x(bar_idx as f64);
                    if x >= -50.0 && x <= viewport.width + 50.0 {
                        candidates.push((bar_idx, x, TickWeight::Month, ts));
                    }
                }
                self.select_ticks_with_spacing(ticks, candidates, viewport, measure_text, labels);
                return;
            }
            step_days = 5;
        }

        for &(y, m) in &months_in_range {
            let month_days = days_in_month(y, m);
            let tick_days = Self::get_tick_days(month_days, step_days);

            for &day in tick_days {
                let ts = date_to_timestamp(y, m, day, 0, 0, 0);
                let bar_idx = Self::timestamp_to_bar_idx(ts, bars, first_ts, bar_interval);
                let x = viewport.pos_to_x(bar_idx as f64);

                if x < -50.0 || x > viewport.width + 50.0 { continue; }

                let weight = if day == 1 {
                    TickWeight::Month
                } else {
                    TickWeight::Day
                };

                let label = labels.label(ts, weight);
                ticks.push(TimeTick {
                    slot: bar_idx,
                    x,
                    weight,
                    label,
                });
            }
        }
    }

    /// Add day boundaries (midnight UTC)
    #[allow(clippy::too_many_arguments)]
    fn add_day_boundaries<T: Timed>(
        &self,
        candidates: &mut Vec<(i64, f64, TickWeight, i64)>,
        start_ts: i64,
        end_ts: i64,
        bars: &[T],
        first_ts: i64,
        bar_interval: i64,
        viewport: &SlotViewport,
    ) {
        let (mut year, mut month, mut day, _, _, _) = timestamp_to_date(start_ts);
        let mut ts = date_to_timestamp(year, month, day, 0, 0, 0);

        if ts > start_ts {
            day -= 1;
            if day < 1 {
                month -= 1;
                if month < 1 { year -= 1; month = 12; }
                day = days_in_month(year, month);
            }
            ts = date_to_timestamp(year, month, day, 0, 0, 0);
        }

        while ts <= end_ts {
            let bar_idx = Self::timestamp_to_bar_idx(ts, bars, first_ts, bar_interval);
            let x = viewport.pos_to_x(bar_idx as f64);

            if x >= -100.0 && x <= viewport.width + 100.0 {
                let weight = if day == 1 { TickWeight::Month } else { TickWeight::Day };
                candidates.push((bar_idx, x, weight, ts));
            }

            day += 1;
            if day > days_in_month(year, month) {
                day = 1;
                month += 1;
                if month > 12 { month = 1; year += 1; }
            }
            ts = date_to_timestamp(year, month, day, 0, 0, 0);
        }
    }

    /// Select ticks with proper spacing.
    ///
    /// Per LWC `_buildMarksImpl`: both the *left* and *right* index gap to the
    /// nearest already-accepted candidate must be `>= max_indexes_per_mark`
    /// where `max_indexes_per_mark = ceil(label_px / bar_spacing)`. Without
    /// the right-side check labels can crowd ahead of the cursor on
    /// non-uniform spacing.
    fn select_ticks_with_spacing<F>(
        &self,
        ticks: &mut Vec<TimeTick>,
        mut candidates: Vec<(i64, f64, TickWeight, i64)>,
        viewport: &SlotViewport,
        measure_text: &F,
        labels: &dyn TimeLabeler,
    ) where
        F: Fn(&str) -> f64,
    {
        if candidates.is_empty() {
            return;
        }

        candidates.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));

        let mut seen_weights: Vec<TickWeight> = Vec::new();
        for c in &candidates {
            if !seen_weights.contains(&c.2) { seen_weights.push(c.2); }
        }
        let label_px = seen_weights.iter()
            .map(|&w| {
                let sample = labels.label(0, w);
                measure_text(&sample)
            })
            .fold(0.0_f64, f64::max)
            .max(20.0);
        let bar_spacing = viewport.spacing.max(1e-6);
        let max_indexes_per_mark = ((label_px / bar_spacing).ceil() as i64).max(1);

        let mut accepted: Vec<i64> = Vec::new();
        let mut accepted_labels: Vec<String> = Vec::new();
        let mut used_positions: Vec<(f64, f64)> = Vec::new();

        for (bar_idx, x, weight, ts) in candidates {
            if x < -50.0 || x > viewport.width + 50.0 {
                continue;
            }

            let left_idx = accepted.iter().rposition(|&i| i < bar_idx);
            let right_idx = accepted.iter().position(|&i| i > bar_idx);
            let left_gap = left_idx.map(|i| bar_idx - accepted[i]);
            let right_gap = right_idx.map(|i| accepted[i] - bar_idx);

            if let Some(g) = left_gap {
                if g < max_indexes_per_mark { continue; }
            }
            if let Some(g) = right_gap {
                if g < max_indexes_per_mark { continue; }
            }

            let label = labels.label(ts, weight);

            let dup_of_neighbor = left_idx.is_some_and(|i| accepted_labels[i] == label)
                || right_idx.is_some_and(|i| accepted_labels[i] == label);
            if dup_of_neighbor {
                continue;
            }

            let measured_width = measure_text(&label);
            let half_width = measured_width / 2.0 + 5.0;

            let conflicts = used_positions
                .iter()
                .any(|(ox, ohw)| (x - ox).abs() < half_width + ohw + 10.0);
            if conflicts {
                continue;
            }

            used_positions.push((x, half_width));

            let pos = accepted.binary_search(&bar_idx).unwrap_or_else(|p| p);
            accepted.insert(pos, bar_idx);
            accepted_labels.insert(pos, label.clone());

            ticks.push(TimeTick {
                slot: bar_idx,
                x,
                weight,
                label,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Labels
// ---------------------------------------------------------------------------

/// Turns a tick time and weight into label text.
pub trait TimeLabeler {
    /// Label for `ts` (Unix seconds) at `weight`.
    fn label(&self, ts: i64, weight: TickWeight) -> String;
}

impl<F: Fn(i64, TickWeight) -> String> TimeLabeler for F {
    fn label(&self, ts: i64, weight: TickWeight) -> String {
        self(ts, weight)
    }
}

/// English short month names.
pub const ENGLISH_MONTHS: [&str; 12] =
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
/// English short weekday names, Sunday first.
pub const ENGLISH_WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// How a day tick shows its date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum DateOrder {
    /// `21.01` (crosshair: `21.01`).
    DayMonth,
    /// `01/21`.
    MonthDay,
    /// `2026-01-21`.
    YearMonthDay,
    /// `21 Jan` (crosshair: `21 Jan`).
    #[default]
    DayMonthName,
}

/// Calendar labels with caller-supplied month and weekday names.
///
/// Year ticks show the year, month ticks the month name, day ticks the date
/// in [`DateOrder`] (optionally prefixed with the weekday), hour and minute
/// ticks `HH:MM` or `h:MM AM`, second ticks add `:SS`. The 12-hour
/// markers are [`Self::am`] and [`Self::pm`] (`AM` / `PM` by default).
///
/// ```
/// use uzor_plot::time_scale::{CalendarLabels, TickWeight, TimeLabeler};
/// let l = CalendarLabels::default();
/// assert_eq!(l.label(1_700_000_000, TickWeight::Day), "14 Nov");
/// assert_eq!(l.label(1_700_000_000, TickWeight::Hour), "22:13");
/// assert_eq!(l.with_24h(false).label(1_700_000_000, TickWeight::Hour), "10:13 PM");
/// let lower = l.with_24h(false).with_am_pm("am", "pm");
/// assert_eq!(lower.label(1_700_000_000, TickWeight::Hour), "10:13 pm");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct CalendarLabels<'a> {
    /// Short month names, January first.
    pub month_names: &'a [&'a str; 12],
    /// Short weekday names, Sunday first.
    pub weekday_names: &'a [&'a str; 7],
    /// Day tick format.
    pub date_order: DateOrder,
    /// 24-hour clock (otherwise 12-hour with AM/PM).
    pub use_24h: bool,
    /// Prefix day ticks with the weekday.
    pub show_weekday: bool,
    /// 12-hour marker before noon. Default `AM`.
    pub am: &'a str,
    /// 12-hour marker from noon. Default `PM`.
    pub pm: &'a str,
}

impl Default for CalendarLabels<'static> {
    fn default() -> Self {
        Self::new(&ENGLISH_MONTHS, &ENGLISH_WEEKDAYS)
    }
}

impl<'a> CalendarLabels<'a> {
    /// Labels with these names, `21 Jan` dates, 24-hour clock, no weekday,
    /// `AM` / `PM` markers for the 12-hour clock.
    pub fn new(month_names: &'a [&'a str; 12], weekday_names: &'a [&'a str; 7]) -> Self {
        Self {
            month_names,
            weekday_names,
            date_order: DateOrder::DayMonthName,
            use_24h: true,
            show_weekday: false,
            am: "AM",
            pm: "PM",
        }
    }
    /// Set the day tick format.
    pub fn with_date_order(mut self, order: DateOrder) -> Self {
        self.date_order = order;
        self
    }
    /// Use the 24-hour clock or not.
    pub fn with_24h(mut self, on: bool) -> Self {
        self.use_24h = on;
        self
    }
    /// Prefix day ticks with the weekday or not.
    pub fn with_weekday(mut self, on: bool) -> Self {
        self.show_weekday = on;
        self
    }
    /// Set the 12-hour clock markers (before noon, from noon).
    pub fn with_am_pm(mut self, am: &'a str, pm: &'a str) -> Self {
        self.am = am;
        self.pm = pm;
        self
    }

    /// `hour` (0-23) on the 12-hour clock with its marker.
    fn twelve_hour(&self, hour: i32) -> (i32, &'a str) {
        if hour == 0 {
            (12, self.am)
        } else if hour < 12 {
            (hour, self.am)
        } else if hour == 12 {
            (12, self.pm)
        } else {
            (hour - 12, self.pm)
        }
    }

    /// Full date and time, for a crosshair readout (`21.01 14:05` style,
    /// per [`Self::date_order`] and the clock setting).
    pub fn full(&self, ts: i64) -> String {
        let (year, month, day, hour, minute, _) = timestamp_to_date(ts);
        let time_str = if self.use_24h {
            format!("{:02}:{:02}", hour, minute)
        } else {
            let (h12, ampm) = self.twelve_hour(hour);
            format!("{}:{:02} {}", h12, minute, ampm)
        };
        let date_str = match self.date_order {
            DateOrder::DayMonth => format!("{:02}.{:02}", day, month),
            DateOrder::MonthDay => format!("{:02}/{:02}", month, day),
            DateOrder::YearMonthDay => format!("{}-{:02}-{:02}", year, month, day),
            DateOrder::DayMonthName => format!("{} {}", day, self.month_names[(month - 1) as usize]),
        };
        format!("{} {}", date_str, time_str)
    }
}

impl TimeLabeler for CalendarLabels<'_> {
    fn label(&self, ts: i64, weight: TickWeight) -> String {
        let (year, month, day, hour, minute, second) = timestamp_to_date(ts);
        let month_name = self.month_names[(month - 1) as usize];
        let (h12, ampm) = self.twelve_hour(hour);
        match weight {
            TickWeight::Year => format!("{}", year),
            TickWeight::Month => month_name.to_string(),
            TickWeight::Day => {
                let dow = if self.show_weekday {
                    format!("{} ", self.weekday_names[weekday_from_timestamp(ts) as usize])
                } else {
                    String::new()
                };
                match self.date_order {
                    DateOrder::DayMonth => format!("{}{}.{:02}", dow, day, month),
                    DateOrder::MonthDay => format!("{}{:02}/{}", dow, month, day),
                    DateOrder::YearMonthDay => format!("{}{}-{:02}-{:02}", dow, year, month, day),
                    DateOrder::DayMonthName => format!("{}{} {}", dow, day, month_name),
                }
            }
            TickWeight::Second | TickWeight::LessThanSecond => {
                if self.use_24h {
                    format!("{:02}:{:02}:{:02}", hour, minute, second)
                } else {
                    format!("{}:{:02}:{:02} {}", h12, minute, second, ampm)
                }
            }
            _ => {
                if self.use_24h {
                    format!("{:02}:{:02}", hour, minute)
                } else {
                    format!("{}:{:02} {}", h12, minute, ampm)
                }
            }
        }
    }
}

/// `DD.MM HH:MM` for `ts` (Unix seconds, UTC).
pub fn format_day_month_time(ts: i64) -> String {
    let (_, month, day, hour, minute, _) = timestamp_to_date(ts);
    format!("{:02}.{:02} {:02}:{:02}", day, month, hour, minute)
}

/// Weekday of `ts` (Unix seconds, UTC), 0 = Sunday.
pub fn weekday_from_timestamp(ts: i64) -> u8 {
    (ts.div_euclid(86400) + 4).rem_euclid(7) as u8
}
