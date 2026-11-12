//! Reference behaviour of the slot viewport and kinetic panning.
use uzor_plot::viewport::{fractional_position, Kinetic, SlotViewport};

#[test]
fn mapping_and_window() {
    let v = SlotViewport::new(800.0).with_spacing(10.0);
    assert_eq!(v.visible_slots(), 80);
    assert_eq!(v.slot_to_x(0), 5.0);
    assert_eq!(v.slot_to_x(1), 15.0);
    let v = v.with_count(10);
    assert_eq!(v.x_to_slot(5.0), Some(0));
    assert_eq!(v.x_to_slot(15.0), Some(1));
    assert_eq!(v.x_to_slot(-5.0), None);
    assert_eq!(v.x_to_slot(150.0), None);
    let w = SlotViewport::new(100.0).with_spacing(10.0).with_start(10.0).with_count(50);
    assert_eq!(w.visible_range(), 9..21);
    assert_eq!(w.x_to_pos(25.0), 12.5);
    assert_eq!(SlotViewport::new(0.0).max_fit_slots(), 0);
    assert_eq!(SlotViewport::new(70.0).max_fit_slots(), 100);
}

#[test]
fn pan_zoom_and_limits() {
    let mut v = SlotViewport::new(800.0).with_spacing(10.0).with_count(1000);
    v.pan_px(50.0);
    assert_eq!(v.start, -5.0);
    v.pan_slots(-5.0);
    assert_eq!(v.start, 0.0);
    // The slot under x = 200 (slot 20) stays under it.
    v.zoom_at(200.0, 2.0);
    assert_eq!(v.spacing, 20.0);
    assert_eq!(v.x_to_pos(200.0), 20.0);
    v.zoom_at(0.0, 1e6);
    assert_eq!(v.spacing, 400.0); // width * 0.5
    v.zoom_at(0.0, 1e-9);
    assert_eq!(v.spacing, 0.7);
    // Zero width does not fail.
    let mut z = SlotViewport::new(0.0);
    z.zoom_at(0.0, 2.0);
    assert_eq!(z.spacing, 0.7);
}

#[test]
fn fits_and_insets() {
    let mut v = SlotViewport::new(1000.0).with_count(500);
    v.reset_to_default();
    assert_eq!(v.spacing, 10.0);
    assert_eq!(v.start, 404.0); // 499 + 5 - 100
    v.fit_all();
    assert_eq!((v.spacing, v.start), (2.0, 0.0));
    v.scroll_to_end();
    assert_eq!(v.start, 0.0);
    v.start = 900.0;
    v.clamp_start();
    assert_eq!(v.start, 0.0);
    v.spacing = 10.0;
    v.shift_for_right_inset(true, 300.0);
    assert_eq!(v.start, 30.0);
    v.shift_for_right_inset(false, 300.0);
    assert_eq!(v.start, 0.0);
}

#[test]
fn fractional_positions() {
    let keys = [60i64, 120, 180, 300];
    let k = |x: &i64| *x;
    assert_eq!(fractional_position(&keys, 120, k, 60.0), 1.0);
    assert_eq!(fractional_position(&keys, 240, k, 60.0), 2.5);
    assert_eq!(fractional_position(&keys, 0, k, 60.0), -0.75); // avg step 80
    assert_eq!(fractional_position(&keys, 420, k, 60.0), 4.0); // last step 120
    assert_eq!(fractional_position(&[100i64], 160, k, 60.0), 1.0);
    assert_eq!(fractional_position::<i64>(&[], 5, k, 60.0), 0.0);
}

#[test]
fn kinetic_decays_and_stops() {
    let mut k = Kinetic::default();
    assert!(!k.is_active());
    k.start(0.0001, 1000.0);
    assert!(!k.is_active());
    k.start(1.0, 1000.0);
    let d = k.update(1100.0, 10.0).unwrap();
    assert!(k.velocity < 1.0 && k.velocity > 0.5);
    assert!((d - k.velocity * 100.0 / 10.0).abs() < 1e-12);
    let mut t = 1100.0;
    let mut n = 0;
    while k.is_active() && n < 10_000 {
        t += 16.0;
        k.update(t, 10.0);
        n += 1;
    }
    assert!(!k.is_active() && n < 10_000);
    assert!((k.velocity_from_drag(100.0, 50.0) - 0.6).abs() < 1e-12);
    assert_eq!(k.velocity_from_drag(100.0, 0.0), 0.0);
    k.start(2.0, 0.0);
    k.stop();
    assert_eq!((k.active, k.velocity), (false, 0.0));
}

// Degenerate input: no panics, no NaN/inf written into the state.

fn finite(v: &SlotViewport) -> bool {
    v.start.is_finite() && v.spacing.is_finite() && v.spacing > 0.0
}

#[test]
fn fit_recent_with_no_slots_is_a_no_op() {
    for width in [0.0, 800.0] {
        let mut v = SlotViewport::new(width).with_count(10).with_spacing(8.0).with_start(3.0);
        v.fit_recent(0, 0, 0.7, 400.0);
        assert_eq!((v.spacing, v.start), (8.0, 3.0), "width {width}");
    }
    let mut v = SlotViewport::new(f64::NAN).with_count(10);
    let before = v;
    v.fit_recent(95, 5, 0.7, 400.0);
    assert_eq!(v.spacing, before.spacing);
    assert_eq!(v.start, before.start);
    // Swapped limits: max is raised to min, as documented.
    let mut v = SlotViewport::new(800.0).with_count(10);
    v.fit_recent(95, 5, 10.0, 2.0);
    assert_eq!(v.spacing, 10.0);
    assert!(finite(&v));
}

#[test]
fn zoom_and_fit_within_swapped_limits_do_not_panic() {
    let mut v = SlotViewport::new(800.0).with_count(100).with_spacing(8.0);
    v.zoom_at_within(400.0, 2.0, 50.0, 10.0);
    assert_eq!(v.spacing, 50.0);
    assert!(finite(&v));
    let mut v = SlotViewport::new(800.0).with_count(100).with_spacing(8.0);
    v.fit_all_within(50.0, 10.0);
    assert_eq!(v.spacing, 50.0);
    assert_eq!(v.start, 0.0);
    // NaN limits do not panic either.
    let mut v = SlotViewport::new(800.0).with_count(100).with_spacing(8.0);
    v.zoom_at_within(400.0, 2.0, f64::NAN, f64::NAN);
    assert!(finite(&v));
    v.fit_all_within(f64::NAN, 10.0);
    assert!(finite(&v));
}

#[test]
fn zero_spacing_never_writes_inf_into_start() {
    let mut v = SlotViewport::new(800.0).with_count(100).with_spacing(0.0).with_start(5.0);
    assert_eq!(v.x_to_pos(400.0), 5.0);
    assert_eq!(v.x_to_slot(400.0), None);
    assert_eq!(v.x_to_slot(0.0), None);
    v.zoom_at_within(400.0, 2.0, 0.7, 400.0);
    assert_eq!((v.spacing, v.start), (0.0, 5.0));
    v.zoom_at(400.0, 2.0);
    assert_eq!((v.spacing, v.start), (0.0, 5.0));
    assert_eq!(v.visible_slots(), 1);
    let _ = v.visible_range();
    // NaN spacing and NaN factor / x.
    let mut v = SlotViewport::new(800.0).with_count(100).with_spacing(f64::NAN).with_start(5.0);
    assert_eq!(v.x_to_pos(400.0), 5.0);
    assert_eq!(v.x_to_slot(400.0), None);
    v.zoom_at(400.0, 2.0);
    assert_eq!(v.start, 5.0);
    let mut v = SlotViewport::new(800.0).with_count(100).with_spacing(8.0).with_start(5.0);
    v.zoom_at(400.0, f64::NAN);
    assert_eq!((v.spacing, v.start), (8.0, 5.0));
    v.zoom_at(f64::NAN, 2.0);
    assert_eq!((v.spacing, v.start), (8.0, 5.0));
    assert_eq!(v.x_to_slot(f64::NAN), None);
    // Non-positive limits would make a zero spacing: refused.
    v.zoom_at_within(400.0, 0.0, 0.0, 0.0);
    assert_eq!((v.spacing, v.start), (8.0, 5.0));
    let mut huge = SlotViewport::new(800.0).with_count(100).with_start(1e300);
    let _ = huge.visible_range();
    huge.start = f64::INFINITY;
    let _ = huge.visible_range();
}

#[test]
fn kinetic_zero_spacing_returns_none_and_keeps_state() {
    let mut k = Kinetic::default();
    k.start(2.0, 0.0);
    let before = k;
    assert_eq!(k.update(16.0, 0.0), None);
    assert_eq!(k, before);
    assert_eq!(k.update(16.0, -1.0), None);
    assert_eq!(k.update(16.0, f64::NAN), None);
    assert_eq!(k.update(16.0, f64::INFINITY), None);
    assert_eq!(k.update(f64::NAN, 8.0), None);
    assert_eq!(k, before);
    let d = k.update(16.0, 8.0).expect("still moving");
    assert!(d.is_finite());
}

#[test]
fn kinetic_nan_time_or_velocity_returns_none_and_keeps_state() {
    // Reviewer case: start at a NaN time.
    let mut k = Kinetic::default();
    k.start(2.0, f64::NAN);
    let before = k;
    assert_eq!(k.update(16.0, 8.0), None);
    assert!(k.velocity.is_finite());
    assert_eq!(k.velocity, before.velocity);
    assert!(k.last_time.is_nan());
    assert_eq!(k.active, before.active);
    // A NaN last_time set directly.
    let mut k = Kinetic::default();
    k.start(2.0, 0.0);
    k.last_time = f64::NAN;
    assert_eq!(k.update(16.0, 8.0), None);
    assert_eq!(k.velocity, 2.0);
    // Infinite stored time.
    k.last_time = f64::INFINITY;
    assert_eq!(k.update(16.0, 8.0), None);
    assert_eq!(k.velocity, 2.0);
    // NaN damping or velocity would leak NaN: refused, state kept.
    let mut k = Kinetic::default();
    k.start(2.0, 0.0);
    k.damping = f64::NAN;
    assert_eq!(k.update(16.0, 8.0), None);
    assert_eq!((k.velocity, k.last_time, k.active), (2.0, 0.0, true));
    let mut k = Kinetic::default();
    k.start(2.0, 0.0);
    k.velocity = f64::NAN;
    assert_eq!(k.update(16.0, 8.0), None);
    assert_eq!(k.last_time, 0.0);
    // Valid motion still runs and decays.
    let mut k = Kinetic::default();
    k.start(2.0, 0.0);
    let d = k.update(16.0, 8.0).unwrap();
    assert_eq!(d, 2.0 * 0.997_f64.powf(16.0) * 16.0 / 8.0);
    assert_eq!(k.last_time, 16.0);
}
