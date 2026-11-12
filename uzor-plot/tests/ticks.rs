//! Reference vectors. Expected values come from the chart code this crate
//! was extracted from, so a caller switching over gets identical output.
use uzor_plot::ticks::*;

#[test]
fn nice_number_ladder() {
    let cases = [
        (0.37, 0.5), (7.0, 10.0), (23.0, 20.0), (0.0042, 0.005), (1.6, 2.0), (3.9, 5.0), (44.0, 50.0),
        (0.000731, 0.001),
    ];
    for (v, want) in cases {
        assert_eq!(nice_number(v), want, "nice_number({v})");
    }
    assert_eq!(nice_number(0.0), 1.0);
    assert_eq!(nice_number(-5.0), 1.0);
}

#[test]
fn nice_step_for_range() {
    assert_eq!(nice_step(100.0, 10.0), 10.0);
    assert_eq!(nice_step(1000.0, 10.0), 100.0);
    assert_eq!(nice_step(4.0, 10.0), 0.5);
    assert_eq!(nice_step(0.005, 10.0), 0.0005);
    assert_eq!(nice_step(0.37, 4.0), 0.1);
}

#[test]
fn nice_step_ceil_rounds_up_with_floor() {
    assert_eq!(nice_step_ceil(100.0, 0.01, 60.0), 2.0);
    assert_eq!(nice_step_ceil(1000.0, 0.5, 60.0), 20.0);
    assert_eq!(nice_step_ceil(3.0, 0.001, 60.0), 0.05);
    assert_eq!(nice_step_ceil(0.0, 0.25, 60.0), 0.25);
    assert_eq!(nice_step_ceil(130.0, 0.01, 60.0), 2.5);
    assert_eq!(nice_step_ceil(f64::NAN, 0.1, 60.0), 0.1);
}

#[test]
fn density_ticks_with_overscan() {
    let d = TickDensity::default();
    let (step, t) = d.ticks(0.0, 100.0, 300.0);
    assert_eq!(step, 10.0);
    assert_eq!(t, vec![-10.0, 0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0, 110.0]);
    let (_, t) = d.ticks(86_000.0, 87_000.0, 300.0);
    assert_eq!(t.first(), Some(&85_900.0));
    assert_eq!(t.last(), Some(&87_100.0));
    assert_eq!(t.len(), 13);
    let (step, t) = d.ticks(-3.7, 12.9, 120.0);
    assert_eq!(step, 5.0);
    assert_eq!(t, vec![-10.0, -5.0, 0.0, 5.0, 10.0, 15.0]);
    // target count clamps to [4, 20]
    assert_eq!(d.target_ticks(30.0), 4.0);
    assert_eq!(d.target_ticks(3000.0), 20.0);
    assert_eq!(d.overscan(100.0, 0.0), 0.0);
}

#[test]
fn density_builders() {
    let d = TickDensity::default().with_spacing_px(56.0).with_tick_bounds(2.0, 50.0).with_overscan_px(0.0);
    assert_eq!(d.target_ticks(112.0), 2.0);
    let (_, t) = d.ticks(0.0, 100.0, 560.0);
    assert_eq!(t, vec![0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0]);
}

#[test]
fn bounded_ticks_stay_inside() {
    assert_eq!(bounded_ticks(0.0, 100.0, 3), vec![0.0, 50.0, 100.0]);
    assert_eq!(bounded_ticks(-17.3, 42.8, 4), vec![0.0, 20.0, 40.0]);
    assert_eq!(bounded_ticks(-1582.0, 1582.0, 2), vec![0.0]);
    assert!(bounded_ticks(10.0, 10.0, 3).is_empty());
    assert!(bounded_ticks(0.0, 100.0, 0).is_empty());
}

#[test]
fn culling_drops_edges_collisions_and_reserved() {
    let c = [AxisLabel::new(100.0, 3.0), AxisLabel::new(50.0, 50.0), AxisLabel::new(0.0, 97.0)];
    let kept = cull_axis_labels(&c, 0.0, 100.0, &[], 5.0);
    assert_eq!(kept, vec![AxisLabel::new(50.0, 50.0)]);

    let c = [AxisLabel::new(1.0, 50.0), AxisLabel::new(2.0, 52.0), AxisLabel::new(3.0, 70.0)];
    let v: Vec<f64> = cull_axis_labels(&c, 0.0, 100.0, &[], 5.0).iter().map(|l| l.value).collect();
    assert_eq!(v, vec![1.0, 3.0]);

    assert!(cull_axis_labels(&[AxisLabel::new(1.0, 50.0)], 0.0, 100.0, &[(40.0, 60.0)], 5.0).is_empty());
}

#[test]
fn step_decimals_ladder() {
    assert_eq!(price_step_decimals(10.0), 2);
    assert_eq!(price_step_decimals(0.5), 2);
    assert_eq!(price_step_decimals(0.05), 2);
    assert_eq!(price_step_decimals(0.005), 3);
    assert_eq!(price_step_decimals(0.0005), 4);
    assert_eq!(price_step_decimals(1e-13), 12);
    assert_eq!(step_decimals(10.0, 0), 0);
    assert_eq!(step_decimals(0.5, 0), 1);
    assert_eq!(step_decimals(0.25, 0), 1);
}

#[test]
fn decimals_from_increment_counts_fraction() {
    assert_eq!(decimals_from_increment(0.1), Some(1));
    assert_eq!(decimals_from_increment(0.25), Some(2));
    assert_eq!(decimals_from_increment(0.00000001), Some(8));
    assert_eq!(decimals_from_increment(10.0), Some(0));
    assert_eq!(decimals_from_increment(0.0), None);
    assert_eq!(decimals_from_increment(f64::NAN), None);
}

#[test]
fn label_precision_auto_and_pinned() {
    // Increment of 4 decimals, step 10: auto keeps the coarser and trims.
    let auto = LabelPrecision::default().with_ceiling(Some(4));
    assert_eq!(auto.format(123.4, 10.0), "123.4");
    assert_eq!(auto.with_pinned(Some(0)).format(123.4, 10.0), "123");
    assert_eq!(auto.with_pinned(Some(4)).format(123.4, 10.0), "123.4000");
    // Ceiling holds when zoomed inside the increment.
    let near = LabelPrecision::default().with_ceiling(Some(3));
    assert_eq!(near.format(6.123, 0.0005), "6.123");
    assert_eq!(near.format(6.1, 0.0005), "6.1");
    assert_eq!(near.with_pinned(Some(3)).format(6.0, 0.5), "6.000");
    assert_eq!(trim_padded_fraction("86800.0"), "86800");
    assert_eq!(trim_padded_fraction("6.500"), "6.5");
    assert_eq!(trim_padded_fraction("12"), "12");
}

// Degenerate input: no panics, no NaN/inf leaking out.

#[test]
fn nice_number_non_finite_is_one() {
    assert_eq!(nice_number(f64::NAN), 1.0);
    assert_eq!(nice_number(f64::INFINITY), 1.0);
    assert_eq!(nice_number(f64::NEG_INFINITY), 1.0);
    assert_eq!(nice_step(0.0, 0.0), 1.0); // 0 / 0
    assert_eq!(nice_step(10.0, 0.0), 1.0); // 10 / 0 = inf
}

#[test]
fn target_ticks_nan_and_swapped_bounds() {
    let d = TickDensity::default();
    assert_eq!(d.target_ticks(f64::NAN), 4.0);
    assert_eq!(d.with_spacing_px(0.0).target_ticks(0.0), 4.0); // 0 / 0
    assert_eq!(d.with_spacing_px(f64::NAN).target_ticks(300.0), 4.0);
    assert_eq!(d.with_spacing_px(0.0).target_ticks(300.0), 20.0); // inf clamps to max
    let swapped = d.with_tick_bounds(20.0, 4.0);
    assert_eq!(swapped.target_ticks(30.0), 4.0);
    assert_eq!(swapped.target_ticks(3000.0), 20.0);
    assert_eq!(swapped.target_ticks(300.0), 10.0);
    let (step, t) = d.with_spacing_px(0.0).ticks(0.0, 0.0, 0.0);
    assert!(step.is_finite());
    assert!(t.iter().all(|v| v.is_finite()));
}

#[test]
fn step_ticks_degenerate_step_is_empty() {
    assert!(step_ticks(0.0, 10.0, 0.0).is_empty());
    assert!(step_ticks(0.0, 10.0, -1.0).is_empty());
    assert!(step_ticks(0.0, 10.0, f64::NAN).is_empty());
    assert!(step_ticks(0.0, 10.0, f64::INFINITY).is_empty());
    assert!(step_ticks(f64::NAN, 10.0, 1.0).is_empty());
    assert!(step_ticks(0.0, f64::INFINITY, 1.0).is_empty());
    assert!(step_ticks(f64::NEG_INFINITY, 0.0, 1.0).is_empty());
    assert_eq!(step_ticks(0.0, 3.0, 1.0), vec![0.0, 1.0, 2.0]);
}

#[test]
fn density_ticks_non_finite_range_is_empty() {
    let d = TickDensity::default();
    assert!(d.ticks(f64::NAN, 100.0, 300.0).1.is_empty());
    assert!(d.ticks(0.0, f64::INFINITY, 300.0).1.is_empty());
    assert!(d.ticks(0.0, 100.0, f64::NAN).1.iter().all(|v| v.is_finite()));
}

#[test]
fn bounded_ticks_non_finite_is_empty() {
    assert!(bounded_ticks(0.0, f64::INFINITY, 4).is_empty());
    assert!(bounded_ticks(f64::NAN, 1.0, 4).is_empty());
    assert!(bounded_ticks(f64::NEG_INFINITY, f64::INFINITY, 4).is_empty());
}

#[test]
fn axis_label_default_and_new() {
    let l = AxisLabel::default();
    assert_eq!((l.value, l.pos), (0.0, 0.0));
    assert_eq!(AxisLabel::new(1.0, 2.0).pos, 2.0);
}
