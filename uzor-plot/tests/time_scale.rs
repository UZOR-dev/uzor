//! Reference vectors from the chart code this crate was extracted from.
use uzor_plot::time_scale::*;
use uzor_plot::viewport::SlotViewport;

const T0: i64 = 1_699_990_000;

fn measure(s: &str) -> f64 {
    s.chars().count() as f64 * 6.5
}

fn regular(step: i64, n: i64) -> Vec<i64> {
    (0..n).map(|i| T0 - T0 % step + i * step).collect()
}

fn summary(t: &[TimeTick]) -> Vec<(i64, f64, u8, String)> {
    t.iter().map(|t| (t.slot, t.x, t.weight as u8, t.label.clone())).collect()
}

fn s(v: &[(i64, f64, u8, &str)]) -> Vec<(i64, f64, u8, String)> {
    v.iter().map(|&(a, b, c, d)| (a, b, c, d.to_string())).collect()
}

#[test]
fn intraday_half_hours() {
    let items = regular(60, 1500);
    let vp = SlotViewport::new(800.0).with_spacing(6.0).with_start(1300.0).with_count(1500);
    let t = TimeScale::new().generate_ticks(&vp, &items, measure, &CalendarLabels::default(), false);
    assert_eq!(
        summary(&t),
        s(&[(1294, -33.0, 20, "17:00"), (1324, 147.0, 20, "17:30"), (1354, 327.0, 20, "18:00"), (1384, 507.0, 20, "18:30"), (1414, 687.0, 20, "19:00")])
    );
}

#[test]
fn daily_shows_months_when_three_are_visible() {
    let items = regular(86400, 1500);
    let vp = SlotViewport::new(800.0).with_spacing(6.0).with_start(1400.0).with_count(1500);
    let t = TimeScale::new().generate_ticks(&vp, &items, measure, &CalendarLabels::default(), false);
    assert_eq!(
        summary(&t),
        s(&[(1417, 105.0, 60, "Oct"), (1448, 291.0, 60, "Nov"), (1478, 471.0, 60, "Dec"), (1509, 657.0, 60, "Jan"), (1540, 843.0, 60, "Feb")])
    );
    // Zoomed far out: every other month, all month-weighted.
    let vp = SlotViewport::new(800.0).with_spacing(0.7).with_start(300.0).with_count(1500);
    let t = TimeScale::new().generate_ticks(&vp, &items, measure, &CalendarLabels::default(), false);
    assert_eq!(t.len(), 22);
    assert!(t.iter().all(|t| t.weight == TickWeight::Month));
    assert_eq!((t[2].slot, t[2].label.as_str()), (353, "Nov"));
    assert_eq!(t[2].x, 37.449_999_999_999_996);
}

#[test]
fn seconds_get_seconds_labels() {
    let items = regular(15, 1500);
    let vp = SlotViewport::new(600.0).with_spacing(40.0).with_start(100.0).with_count(1500);
    let t = TimeScale::new().generate_ticks(&vp, &items, measure, &CalendarLabels::default(), false);
    let labels: Vec<&str> = t.iter().map(|t| t.label.as_str()).collect();
    assert_eq!(labels, ["19:51:30", "19:52:00", "19:52:30", "19:53:00", "19:53:30", "19:54:00", "19:54:30", "19:55:00"]);
    assert_eq!(t[1].x, 100.0);
}

#[test]
fn irregular_series_snap_round_boundaries_to_items() {
    let mut v = vec![];
    let mut t = T0;
    for i in 0..300i64 {
        t += match i % 11 {
            0 => 900,
            1 | 2 => 0,
            3 => 7,
            5 => 3600,
            _ => 40,
        };
        v.push(t);
    }
    let vp = SlotViewport::new(800.0).with_spacing(8.0).with_start(100.0).with_count(300);
    let t = TimeScale::new().generate_ticks(&vp, &v, measure, &CalendarLabels::default(), false);
    let got: Vec<(i64, &str)> = t.iter().map(|t| (t.slot, t.label.as_str())).collect();
    assert_eq!(
        got,
        [(98, "07:20"), (105, "08:35"), (114, "08:55"), (121, "10:05"), (129, "11:15"), (136, "11:35"), (143, "12:45"),
         (153, "13:55"), (164, "15:15"), (171, "16:30"), (180, "16:50"), (187, "18:00"), (196, "19:10"), (203, "20:00")]
    );
    assert!(t.iter().all(|t| t.weight == TickWeight::Minute5));
}

#[test]
fn ticks_never_repeat_or_overlap() {
    for (step, sp) in [(60, 2.0), (60, 30.0), (3600, 4.0), (86400, 13.0), (1, 200.0)] {
        let items = regular(step, 1000);
        let vp = SlotViewport::new(1000.0).with_spacing(sp).with_start(500.0).with_count(1000);
        let t = TimeScale::new().generate_ticks(&vp, &items, measure, &CalendarLabels::default(), false);
        assert!(!t.is_empty());
        for w in t.windows(2) {
            assert!(w[0].x <= w[1].x);
            assert_ne!(w[0].label, w[1].label);
            let gap = w[1].x - w[0].x;
            assert!(gap >= measure(&w[0].label) / 2.0 + measure(&w[1].label) / 2.0, "{step} {sp}: {w:?}");
        }
    }
}

#[test]
fn empty_and_tiny_series() {
    let vp = SlotViewport::new(800.0).with_count(0);
    assert!(TimeScale::new().generate_ticks::<i64, _>(&vp, &[], measure, &CalendarLabels::default(), false).is_empty());
    let one = [T0];
    let vp = SlotViewport::new(800.0).with_count(1);
    let _ = TimeScale::new().generate_ticks(&vp, &one, measure, &CalendarLabels::default(), true);
}

#[test]
fn calendar_math() {
    assert_eq!(timestamp_to_date(0), (1970, 1, 1, 0, 0, 0));
    assert_eq!(timestamp_to_date(951_782_400), (2000, 2, 29, 0, 0, 0));
    assert_eq!(timestamp_to_date(-1), (1969, 12, 31, 23, 59, 59));
    assert_eq!(date_to_timestamp(2024, 3, 1, 12, 30, 15), 1_709_296_215);
    assert!(is_leap_year(2000) && !is_leap_year(1900) && is_leap_year(2024));
    assert_eq!(days_in_month(2023, 2), 28);
    assert_eq!(days_in_month(2024, 2), 29);
    assert_eq!(weekday_from_timestamp(0), 4);
    assert_eq!(weekday_from_timestamp(-1), 3);
}

#[test]
fn weights() {
    assert_eq!(weight_by_time(1_704_067_200_000, 1_704_067_199_000), TickWeight::Year);
    assert_eq!(weight_by_time(3_600_000, 3_599_000), TickWeight::Hour);
    assert_eq!(weight_by_time(300_000, 240_000), TickWeight::Minute5);
    assert_eq!(weight_by_time(1500, 1200), TickWeight::LessThanSecond);
    let mut pts: Vec<(i64, TickWeight)> = [0i64, 60, 120, 3600, 86400].iter().map(|&t| (t, TickWeight::Second)).collect();
    fill_weights(&mut pts, 0);
    let w: Vec<TickWeight> = pts.iter().map(|p| p.1).collect();
    // The first point compares against one average step earlier, in 1969.
    assert_eq!(w, [TickWeight::Year, TickWeight::Minute1, TickWeight::Minute1, TickWeight::Hour, TickWeight::Day]);
    assert!(TickWeight::Year > TickWeight::Month && TickWeight::Month.is_major() && TickWeight::Day.is_medium());
}

#[test]
fn labels() {
    let ts = 1_700_000_000; // Tue 2023-11-14 22:13:20 UTC
    let l = CalendarLabels::default();
    assert_eq!(l.label(ts, TickWeight::Year), "2023");
    assert_eq!(l.label(ts, TickWeight::Month), "Nov");
    assert_eq!(l.label(ts, TickWeight::Day), "14 Nov");
    assert_eq!(l.label(ts, TickWeight::Minute5), "22:13");
    assert_eq!(l.label(ts, TickWeight::Second), "22:13:20");
    let us = l.with_date_order(DateOrder::MonthDay).with_24h(false).with_weekday(true);
    assert_eq!(us.label(ts, TickWeight::Day), "Tue 11/14");
    assert_eq!(us.label(ts, TickWeight::Second), "10:13:20 PM");
    assert_eq!(l.with_date_order(DateOrder::DayMonth).label(ts, TickWeight::Day), "14.11");
    assert_eq!(l.with_date_order(DateOrder::YearMonthDay).label(ts, TickWeight::Day), "2023-11-14");
    assert_eq!(l.with_date_order(DateOrder::DayMonth).full(ts), "14.11 22:13");
    assert_eq!(us.full(ts), "11/14 10:13 PM");
    assert_eq!(format_day_month_time(ts), "14.11 22:13");
    let names = ["янв", "фев", "мар", "апр", "май", "июн", "июл", "авг", "сен", "окт", "ноя", "дек"];
    let days = ["вс", "пн", "вт", "ср", "чт", "пт", "сб"];
    let ru = CalendarLabels::new(&names, &days).with_weekday(true);
    assert_eq!(ru.label(ts, TickWeight::Day), "вт 14 ноя");
    let custom = |t: i64, _w: TickWeight| format!("t{t}");
    assert_eq!(custom.label(5, TickWeight::Day), "t5");
}

#[test]
fn days_in_month_out_of_range_is_zero() {
    assert_eq!(days_in_month(2024, 2), 29);
    assert_eq!(days_in_month(2023, 12), 31);
    for m in [0, 13, -1, i32::MIN, i32::MAX] {
        assert_eq!(days_in_month(2024, m), 0, "month {m}");
    }
    // Months past 12 add no days instead of panicking.
    assert_eq!(date_to_timestamp(2024, 14, 1, 0, 0, 0), date_to_timestamp(2025, 1, 1, 0, 0, 0));
}

#[test]
fn twelve_hour_markers_are_configurable() {
    let l = CalendarLabels::default().with_24h(false);
    assert_eq!(l.label(T0, TickWeight::Hour), "7:26 PM");
    assert_eq!(l.full(T0), "14 Nov 7:26 PM");
    let ru = l.with_am_pm("ДП", "ПП");
    assert_eq!(ru.label(T0, TickWeight::Hour), "7:26 ПП");
    assert_eq!(ru.label(T0 - 12 * 3600, TickWeight::Second), "7:26:40 ДП");
    assert_eq!(ru.full(T0 - 19 * 3600), "14 Nov 12:26 ДП");
}

#[test]
fn degenerate_viewports_do_not_panic() {
    let items = regular(60, 500);
    let labels = CalendarLabels::default();
    for (width, spacing, start) in [
        (0.0, 6.0, 0.0),
        (800.0, 0.0, 0.0),
        (800.0, f64::NAN, 10.0),
        (f64::NAN, 6.0, 10.0),
        (800.0, 6.0, f64::NAN),
        (800.0, 6.0, -1e6),
        (800.0, 6.0, 1e6),
    ] {
        let vp = SlotViewport::new(width).with_spacing(spacing).with_start(start).with_count(items.len());
        for irregular in [false, true] {
            let t = TimeScale::new().generate_ticks(&vp, &items, measure, &labels, irregular);
            assert!(t.iter().all(|t| !t.x.is_nan()), "{width} {spacing} {start} {irregular}");
            if spacing.is_nan() || spacing <= 0.0 || start.is_nan() {
                assert!(t.is_empty(), "{width} {spacing} {start} {irregular}");
            }
        }
    }
}

/// The year-by-year conversion this crate used to ship (and the chart code
/// it came from), kept as a reference for the closed form.
fn naive_to_date(ts: i64) -> (i32, i32, i32, i32, i32, i32) {
    let tod = ts.rem_euclid(86400);
    let mut days = ts.div_euclid(86400);
    let mut year = 1970_i32;
    if days >= 0 {
        loop {
            let n = if is_leap_year(year) { 366 } else { 365 };
            if days < n {
                break;
            }
            days -= n;
            year += 1;
        }
    } else {
        loop {
            year -= 1;
            days += if is_leap_year(year) { 366 } else { 365 };
            if days >= 0 {
                break;
            }
        }
    }
    let mut month = 1;
    while days >= days_in_month(year, month) as i64 {
        days -= days_in_month(year, month) as i64;
        month += 1;
    }
    (year, month, days as i32 + 1, (tod / 3600) as i32, ((tod % 3600) / 60) as i32, (tod % 60) as i32)
}

fn naive_to_ts(year: i32, month: i32, day: i32, h: i32, mi: i32, s: i32) -> i64 {
    let mut days: i64 = 0;
    if year >= 1970 {
        for y in 1970..year {
            days += if is_leap_year(y) { 366 } else { 365 };
        }
    } else {
        for y in year..1970 {
            days -= if is_leap_year(y) { 366 } else { 365 };
        }
    }
    for m in 1..month {
        days += days_in_month(year, m) as i64;
    }
    days += (day - 1) as i64;
    days * 86400 + h as i64 * 3600 + mi as i64 * 60 + s as i64
}

#[test]
fn closed_form_calendar_matches_year_by_year() {
    // Every day from 1600 to 2400 (both leap-century rules), at a few times of day.
    let lo = naive_to_ts(1600, 1, 1, 0, 0, 0);
    let hi = naive_to_ts(2400, 12, 31, 0, 0, 0);
    let mut ts = lo;
    while ts <= hi {
        for tod in [0, 1, 3599, 43_210, 86_399] {
            let t = ts + tod;
            let d = timestamp_to_date(t);
            assert_eq!(d, naive_to_date(t), "ts {t}");
            assert_eq!(date_to_timestamp(d.0, d.1, d.2, d.3, d.4, d.5), t, "round trip {t}");
        }
        ts += 86_400;
    }
    // Out-of-range fields roll over the same way as before.
    for (y, m, d) in [(2024, 0, 1), (2024, 14, 1), (2023, 13, 5), (2024, 2, 30), (2024, 1, 0), (1969, 12, 32)] {
        assert_eq!(date_to_timestamp(y, m, d, 25, 61, 61), naive_to_ts(y, m, d, 25, 61, 61), "{y}-{m}-{d}");
    }
}

#[test]
fn calendar_extremes_are_constant_time_and_saturate() {
    // Used to step year by year: hang (i64 edges) or overflow (i32 years).
    assert_eq!(timestamp_to_date(i64::MAX), timestamp_to_date(MAX_DATE_TIMESTAMP));
    assert_eq!(timestamp_to_date(i64::MIN), timestamp_to_date(MIN_DATE_TIMESTAMP));
    assert_eq!(timestamp_to_date(MAX_DATE_TIMESTAMP), (MAX_DATE_YEAR, 12, 31, 23, 59, 59));
    assert_eq!(timestamp_to_date(MIN_DATE_TIMESTAMP), (MIN_DATE_YEAR, 1, 1, 0, 0, 0));
    assert_eq!(date_to_timestamp(MAX_DATE_YEAR, 12, 31, 23, 59, 59), MAX_DATE_TIMESTAMP);
    assert_eq!(date_to_timestamp(MIN_DATE_YEAR, 1, 1, 0, 0, 0), MIN_DATE_TIMESTAMP);
    // Full i32 inputs do not overflow.
    let _ = date_to_timestamp(i32::MAX, i32::MAX, i32::MAX, i32::MAX, i32::MAX, i32::MAX);
    let _ = date_to_timestamp(i32::MIN, i32::MIN, i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    assert_eq!(date_to_timestamp(i32::MAX, 13, 1, 0, 0, 0) - date_to_timestamp(i32::MAX, 12, 1, 0, 0, 0), 31 * 86_400);
    // Round trip at a far but in-window date.
    let t = date_to_timestamp(-5_000_000, 2, 29, 12, 0, 0);
    assert_eq!(timestamp_to_date(t), (-5_000_000, 2, 29, 12, 0, 0));
    // Labels at the edges do not panic.
    let l = CalendarLabels::default();
    for ts in [i64::MIN, i64::MAX, 0, -1] {
        let _ = l.full(ts);
        let _ = l.label(ts, TickWeight::Day);
        let _ = weekday_from_timestamp(ts);
    }
}
