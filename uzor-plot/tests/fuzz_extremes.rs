//! Every public entry point over extreme and non-finite inputs: nothing may
//! panic or hang. Results are only checked where a contract says so.
use uzor_plot::crosshair::*;
use uzor_plot::ticks::*;
use uzor_plot::time_scale::*;
use uzor_plot::value_scale::*;
use uzor_plot::viewport::*;

const F: [f64; 14] = [
    0.0, -0.0, 1.0, -1.0, 0.5, 1e-300, -1e-300, 1e300, -1e300, f64::MAX, f64::MIN_POSITIVE,
    f64::NAN, f64::INFINITY, f64::NEG_INFINITY,
];
const I: [i64; 9] = [0, 1, -1, 1_700_000_000, i64::MAX, i64::MIN, i64::MAX / 2, i64::MIN / 2, -62_000_000_000];
const U: [usize; 5] = [0, 1, 3, 1_000, usize::MAX];

#[test]
fn ticks_api() {
    for &a in &F {
        let _ = nice_number(a);
        let _ = price_step_decimals(a);
        let _ = decimals_from_increment(a);
        for &b in &F {
            let _ = nice_step(a, b);
            let d = TickDensity::default().with_spacing_px(a).with_tick_bounds(a, b).with_overscan_px(b);
            let _ = d.target_ticks(b);
            let _ = d.step(a, b);
            let _ = d.overscan(a, b);
            for &c in &F {
                let _ = nice_step_ceil(a, b, c);
                assert!(step_ticks(a, b, c).len() <= MAX_TICKS);
                assert!(d.ticks(a, b, c).1.len() <= MAX_TICKS);
                let _ = cull_axis_labels(&[AxisLabel::new(a, b), AxisLabel::new(b, c)], a, b, &[(c, a)], c);
            }
            for &u in &U {
                assert!(bounded_ticks(a, b, u).len() <= MAX_TICKS);
                let _ = step_decimals(a, u);
            }
        }
        for &u in &U {
            let _ = format_fixed(a, u);
            let p = LabelPrecision::default().with_pinned(Some(u)).with_ceiling(Some(u)).with_min_decimals(u);
            let _ = p.format(a, a);
            let _ = LabelPrecision::default().with_min_decimals(u).format(a, 1e-300);
        }
    }
    let _ = trim_padded_fraction("");
    let _ = trim_padded_fraction(".");
}

#[test]
fn value_scale_api() {
    let kinds = [ScaleKind::Linear, ScaleKind::Log, ScaleKind::Percent, ScaleKind::Indexed];
    for &k in &kinds {
        for &lo in &F {
            for &hi in &F {
                let mut s = ValueScale::new(lo, hi).with_kind(k).with_base(lo).with_fit_margin(hi);
                for &l in &F {
                    let _ = s.value_to_coord(l, hi);
                    let _ = s.coord_to_value(l, hi);
                    let _ = s.step(l);
                    assert!(s.ticks(l).len() <= MAX_TICKS);
                    let _ = s.format_value(l, hi);
                    let _ = s.format_label(l, hi);
                    let _ = s.max_label_len(l);
                    let _ = ValueScale::min_range_at(l);
                }
                let items = [(lo, hi), (hi, lo)];
                for fit in [RangeFit::Auto, RangeFit::Follow] {
                    s.fit = fit;
                    let _ = s.auto_fit(&items, 0..usize::MAX, |&p| p);
                    let (from, to) = (5, 1); // inverted on purpose
                    let _ = s.auto_fit(&items, from..to, |&p| p);
                }
                s.zoom(lo, hi);
                s.pan(lo);
                let _ = s.set_range_clamped(lo, hi);
            }
        }
    }
}

#[test]
fn viewport_api() {
    for &w in &F {
        for &sp in &F {
            for &st in &F {
                for &u in &U {
                    let mut v = SlotViewport::new(w).with_spacing(sp).with_start(st).with_count(u)
                        .with_body_ratio(st).with_spacing_limits(sp, w);
                    let _ = (v.max_spacing(), v.visible_slots(), v.max_fit_slots(), v.start_index());
                    let _ = v.visible_range();
                    let _ = (v.slot_to_x(u), v.pos_to_x(st), v.x_to_slot(st), v.x_to_pos(st), v.body_width());
                    v.pan_px(st);
                    v.pan_slots(st);
                    v.zoom_at(st, w);
                    v.zoom_at_within(st, w, sp, st);
                    v.scroll_to_end();
                    v.clamp_start();
                    v.fit_all();
                    v.fit_all_within(sp, st);
                    v.fit_recent(u, u, sp, st);
                    v.reset_to_default();
                    v.shift_for_right_inset(true, st);
                }
            }
        }
    }
    for &k in &I {
        for items in [vec![], vec![k], vec![i64::MIN, k, i64::MAX], vec![0, 1, k]] {
            for &d in &F {
                let _ = fractional_position(&items, k, |&t| t, d);
                let _ = fractional_position(&items, i64::MIN, |&t| t, d);
                let _ = fractional_position(&items, i64::MAX, |&t| t, d);
            }
        }
    }
    for &a in &F {
        for &b in &F {
            let mut k = Kinetic::default();
            k.start(a, b);
            k.active = true;
            for &c in &F {
                let r = k.update(c, a);
                assert!(r.is_none_or(f64::is_finite));
            }
            let _ = k.velocity_from_drag(a, b);
            k.stop();
        }
    }
}

#[test]
fn crosshair_api() {
    for mode in [CrosshairMode::Free, CrosshairMode::StrongMagnet, CrosshairMode::WeakMagnet, CrosshairMode::Hidden] {
        for &a in &F {
            let _ = mode.snap_tolerance(a);
            let mut c = Crosshair::new().with_mode(mode);
            for &b in &F {
                c.update(a, b, Some(usize::MAX), a, b, true);
                c.update_clamped(a, b, None, a, b, true, true, b);
                let _ = c.snap(&[a, b], |v| v * a, b);
                let _ = snap_to_nearest(&[a, b, f64::NAN], a, |v| v / b, b);
                c.set_snapped(a, b);
                let _ = (c.effective_y(true), c.effective_value(false), c.is_snapped());
                let _ = c.tooltip_position(a, true, (a, b), (b, a), (a, a));
                c.toggle_magnet();
            }
            c.hide();
            c.show();
        }
    }
}

#[test]
fn time_scale_api() {
    let l = CalendarLabels::default();
    for &t in &I {
        let _ = timestamp_to_date(t);
        let _ = format_day_month_time(t);
        let _ = weekday_from_timestamp(t);
        let _ = l.full(t);
        for w in [TickWeight::Year, TickWeight::Month, TickWeight::Day, TickWeight::Hour, TickWeight::Second] {
            let _ = l.label(t, w);
            let _ = l.with_24h(false).with_weekday(true).label(t, w);
        }
        for &u in &I {
            let _ = weight_by_time(t, u);
        }
    }
    for y in [i32::MIN, -1, 0, 1970, i32::MAX] {
        let _ = is_leap_year(y);
        for m in [i32::MIN, 0, 2, 13, i32::MAX] {
            let _ = days_in_month(y, m);
            let _ = date_to_timestamp(y, m, i32::MIN, i32::MAX, i32::MIN, i32::MAX);
        }
    }
    for start in [0, 1, 2, 3, 4, usize::MAX] {
        let mut pts = [(i64::MIN, TickWeight::Second), (0, TickWeight::Second), (i64::MAX, TickWeight::Second)];
        fill_weights(&mut pts, start);
        fill_weights(&mut [], start);
    }
}

#[test]
fn generate_ticks_api() {
    let l = CalendarLabels::default();
    let measure = |s: &str| s.len() as f64 * 6.5;
    let regular: Vec<i64> = (0..300).map(|i| 1_700_000_000 + i * 3600).collect();
    let mut sets: Vec<Vec<i64>> = vec![regular, vec![1_700_000_000]];
    for &a in &I {
        for &b in &I {
            sets.push(vec![a, b]);
            sets.push(vec![a, 0, b]);
        }
    }
    let views = [0.0, 1.0, 800.0, 1e-300, 1e300, f64::NAN, f64::INFINITY, -800.0];
    let t0 = std::time::Instant::now();
    for items in &sets {
        for &w in &views {
            for &sp in &[6.0, 0.7, 1e-3, 1e6, 0.0, f64::NAN] {
                for &st in &[0.0, -50.0, 1e6, -1e12, 1e15, f64::NAN] {
                    let vp = SlotViewport::new(w).with_spacing(sp).with_start(st).with_count(items.len());
                    for irregular in [false, true] {
                        let t = TimeScale::new().generate_ticks(&vp, items, measure, &l, irregular);
                        assert!(t.len() <= TICK_MAX_TICKS);
                    }
                }
            }
        }
    }
    eprintln!("generate_ticks fuzz: {:?}", t0.elapsed());
}
