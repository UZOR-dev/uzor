//! Crosshair behaviour, ported from the chart code this crate was extracted from.
use uzor_plot::crosshair::{snap_to_nearest, Crosshair, CrosshairMode};

const OHLC: [f64; 4] = [100.0, 110.0, 90.0, 105.0];
const BODY: [f64; 2] = [100.0, 105.0];

fn at(mode: CrosshairMode, value: f64) -> Crosshair {
    let mut c = Crosshair::new().with_mode(mode);
    c.update(0.0, 200.0 - value, Some(0), 0.0, value, true);
    c
}

#[test]
fn strong_magnet_snaps_to_body() {
    let to_y = |p: f64| 200.0 - p;
    let (v, y) = at(CrosshairMode::StrongMagnet, 103.0).snap(&BODY, to_y, 8.0);
    assert_eq!((v, y), (105.0, 95.0));
    assert_eq!(at(CrosshairMode::StrongMagnet, 101.0).snap(&BODY, to_y, 8.0).0, 100.0);
    // 25 px away is past the 20 px floor.
    assert_eq!(at(CrosshairMode::StrongMagnet, 130.0).snap(&BODY, to_y, 8.0).0, 130.0);
}

#[test]
fn weak_magnet_snaps_to_any_candidate() {
    let to_y = |p: f64| 200.0 - p;
    assert_eq!(at(CrosshairMode::WeakMagnet, 109.0).snap(&OHLC, to_y, 8.0).0, 110.0);
    assert_eq!(at(CrosshairMode::WeakMagnet, 91.0).snap(&OHLC, to_y, 8.0).0, 90.0);
    assert_eq!(at(CrosshairMode::WeakMagnet, 97.0).snap(&OHLC, to_y, 8.0).0, 100.0);
    assert_eq!(at(CrosshairMode::WeakMagnet, 125.0).snap(&OHLC, to_y, 8.0).0, 125.0);
    assert_eq!(at(CrosshairMode::Free, 109.0).snap(&OHLC, to_y, 8.0), (109.0, 91.0));
}

#[test]
fn tolerances() {
    assert_eq!(CrosshairMode::StrongMagnet.snap_tolerance(8.0), Some(20.0));
    assert_eq!(CrosshairMode::StrongMagnet.snap_tolerance(1000.0), Some(80.0));
    assert_eq!(CrosshairMode::WeakMagnet.snap_tolerance(100.0), Some(25.0));
    assert_eq!(CrosshairMode::Free.snap_tolerance(100.0), None);
    assert_eq!(snap_to_nearest(&[], 5.0, |v| v, 100.0), (5.0, 5.0));
    assert_eq!(snap_to_nearest(&[4.0, 6.0], 5.0, |v| v, 1.0), (4.0, 4.0)); // tie keeps the first
}

#[test]
fn effective_values_and_toggle() {
    let mut c = Crosshair::new().with_mode(CrosshairMode::StrongMagnet);
    c.update(0.0, 100.0, None, 0.0, 50.0, true);
    c.set_snapped(55.0, 95.0);
    assert!(c.is_snapped());
    assert_eq!((c.effective_y(false), c.effective_value(false)), (95.0, 55.0));
    assert_eq!((c.effective_y(true), c.effective_value(true)), (100.0, 50.0));
    c.toggle_magnet();
    assert_eq!(c.mode, CrosshairMode::Free);
    c.toggle_magnet();
    assert_eq!(c.mode, CrosshairMode::WeakMagnet);
    c.hide();
    assert!(!c.visible);
    c.update_clamped(0.0, -40.0, None, 0.0, 1.0, true, true, 300.0);
    assert_eq!((c.y, c.snapped_y), (0.0, 0.0));
    c.update_clamped(0.0, -40.0, None, 0.0, 1.0, true, false, 300.0);
    assert_eq!(c.y, -40.0);
}

#[test]
fn tooltip_flips_and_clamps() {
    let area = (800.0, 600.0);
    let size = (150.0, 100.0);
    let mut c = Crosshair::new();
    c.update(100.0, 100.0, None, 0.0, 0.0, true);
    assert_eq!(c.tooltip_position(100.0, false, area, size, (10.0, 10.0)), (110.0, 110.0));
    assert_eq!(c.tooltip_position(750.0, false, area, size, (10.0, 10.0)).0, 590.0);
    c.update(100.0, 550.0, None, 0.0, 0.0, true);
    assert_eq!(c.tooltip_position(100.0, false, area, size, (10.0, 10.0)).1, 440.0);
    let mut m = Crosshair::new().with_mode(CrosshairMode::StrongMagnet);
    m.update(100.0, 200.0, None, 0.0, 0.0, true);
    m.set_snapped(0.0, 150.0);
    assert_eq!(m.tooltip_position(100.0, false, area, size, (10.0, 10.0)).1, 160.0);
    assert_eq!(m.tooltip_position(100.0, true, area, size, (10.0, 10.0)).1, 210.0);
    let p = Crosshair::new().tooltip_position(0.0, false, (100.0, 50.0), size, (-10.0, -10.0));
    assert_eq!(p, (0.0, 0.0));
}
