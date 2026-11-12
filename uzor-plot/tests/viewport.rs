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
