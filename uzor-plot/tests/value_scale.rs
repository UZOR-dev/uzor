//! Reference vectors from the chart code this crate was extracted from,
//! plus the indexed-to-100 kind, which is new here.
use uzor_plot::ticks::LabelPrecision;
use uzor_plot::value_scale::{RangeFit, ScaleKind, ValueScale};

#[test]
fn linear_mapping_round_trips() {
    let s = ValueScale::new(0.0, 100.0);
    assert_eq!(s.value_to_coord(100.0, 300.0), 0.0);
    assert_eq!(s.value_to_coord(0.0, 300.0), 300.0);
    assert_eq!(s.coord_to_value(150.0, 300.0), 50.0);
    assert_eq!(ValueScale::new(5.0, 5.0).value_to_coord(5.0, 300.0), 150.0);
}

#[test]
fn log_ticks_and_mapping() {
    let s = ValueScale::new(1.0, 100_000.0).with_kind(ScaleKind::Log);
    let t = s.ticks(300.0);
    assert_eq!(t.len(), 13);
    assert_eq!(t[1], 1.0);
    assert_eq!(t[5], 100.0);
    assert_eq!(t[11], 100_000.0);
    assert_eq!(t[12], 316_227.766_016_837_9);
    assert_eq!(s.value_to_coord(100.0, 500.0), 300.0);
    assert_eq!(s.coord_to_value(250.0, 500.0), 316.227_766_016_837_96);
}

#[test]
fn percent_ticks_and_labels() {
    let s = ValueScale::new(90.0, 110.0).with_kind(ScaleKind::Percent).with_base(100.0);
    let t = s.ticks(300.0);
    assert_eq!(t.first(), Some(&88.0));
    assert_eq!(t[6], 100.0);
    assert_eq!(t.last(), Some(&112.000_000_000_000_01));
    assert_eq!(s.format_label(95.0, 300.0), "-5.00%");
    assert_eq!(s.format_label(110.0, 300.0), "+10.00%");
    assert_eq!(s.value_to_coord(100.0, 300.0), 150.0);
}

#[test]
fn indexed_to_100() {
    let s = ValueScale::new(50.0, 75.0).with_kind(ScaleKind::Indexed).with_base(50.0);
    assert_eq!(s.to_indexed(75.0), 150.0);
    assert_eq!(s.from_indexed(150.0), 75.0);
    assert_eq!(s.format_label(60.0, 300.0), "120.00");
    assert_eq!(s.value_to_coord(75.0, 300.0), 0.0);
    assert_eq!(s.value_to_coord(50.0, 300.0), 300.0);
    assert_eq!(s.coord_to_value(150.0, 300.0), 62.5);
    // Index range 100..150 at 300px: step 5 index points = 2.5 value units.
    let t = s.ticks(300.0);
    assert!(t.contains(&50.0) && t.contains(&75.0), "{t:?}");
    for w in t.windows(2) {
        assert!((w[1] - w[0] - 2.5).abs() < 1e-9);
    }
    // Base zero never divides.
    let mut z = s;
    z.base = 0.0;
    assert_eq!(z.to_indexed(10.0), 0.0);
    assert_eq!(z.value_to_coord(10.0, 300.0), 150.0);
}

#[test]
fn auto_fit_margins_and_follow() {
    let items = [(9.0, 11.0), (8.0, 10.0), (10.0, 14.0), (12.0, 20.0)];
    let mut s = ValueScale::new(0.0, 1.0);
    assert!(s.auto_fit(&items, 0..3, |&e| e));
    assert_eq!((s.min, s.max), (7.76, 14.24));
    s.fit = RangeFit::Follow;
    assert!(s.auto_fit(&items, 0..2, |&e| e));
    assert_eq!((s.min, s.max), (7.52, 20.48));
    // Manual and empty windows leave the range alone.
    s.fit = RangeFit::Manual;
    assert!(!s.auto_fit(&items, 0..4, |&e| e));
    let mut a = ValueScale::new(0.0, 1.0);
    assert!(!a.auto_fit(&items, 2..2, |&e| e));
    assert!(!a.auto_fit::<(f64, f64)>(&[], 0..2, |&e| e));
    assert_eq!((a.min, a.max), (0.0, 1.0));
    // Custom margin.
    let mut m = ValueScale::new(0.0, 1.0).with_fit_margin(0.0);
    m.auto_fit(&items, 0..4, |&e| e);
    assert_eq!((m.min, m.max), (8.0, 20.0));
}

#[test]
fn zoom_pan_and_clamp() {
    let mut s = ValueScale::new(0.0, 100.0);
    s.zoom(0.5, 25.0);
    assert_eq!((s.min, s.max), (12.5, 62.5));
    assert_eq!(s.fit, RangeFit::Manual);
    s.pan(10.0);
    assert_eq!((s.min, s.max), (22.5, 72.5));
    let mut c = ValueScale::new(0.0, 1.0);
    assert!(!c.set_range_clamped(100_000.0, 100_000.05));
    assert!(c.set_range_clamped(100_000.0, 100_000.2));
    assert_eq!(c.fit, RangeFit::Manual);
}

#[test]
fn labels_and_widths() {
    let s = ValueScale::new(86_000.0, 87_000.0).with_labels(LabelPrecision::default().with_ceiling(Some(1)));
    assert_eq!(s.step(300.0), 100.0);
    assert_eq!(s.format_value(86_800.5, 300.0), "86800.5");
    assert_eq!(s.max_label_len(300.0), 5);
    let pinned = s.with_labels(LabelPrecision::default().with_pinned(Some(4)));
    assert_eq!(pinned.max_label_len(300.0), 10);
}

#[test]
fn mode_cycles() {
    assert_eq!(ScaleKind::Linear.next(), ScaleKind::Log);
    assert_eq!(ScaleKind::Log.next(), ScaleKind::Percent);
    assert_eq!(ScaleKind::Percent.next(), ScaleKind::Linear);
    assert_eq!(ScaleKind::Indexed.next(), ScaleKind::Linear);
    assert_eq!(RangeFit::Auto.next(), RangeFit::Follow);
    assert_eq!(RangeFit::Follow.next(), RangeFit::Manual);
    assert_eq!(RangeFit::Manual.next(), RangeFit::Auto);
}

// Degenerate input: no panics, no NaN/inf leaking out.

#[test]
fn coord_to_value_zero_length_reads_the_middle() {
    let s = ValueScale::new(0.0, 100.0);
    for len in [0.0, -10.0, f64::NAN, f64::INFINITY] {
        assert_eq!(s.coord_to_value(150.0, len), 50.0, "linear, length {len}");
    }
    for kind in [ScaleKind::Linear, ScaleKind::Log, ScaleKind::Percent, ScaleKind::Indexed] {
        let s = ValueScale::new(10.0, 1000.0).with_kind(kind).with_base(100.0);
        let v = s.coord_to_value(42.0, 0.0);
        assert!(v.is_finite(), "{kind:?}: {v}");
        assert!(v > 10.0 && v < 1000.0, "{kind:?}: {v}");
    }
    let log = ValueScale::new(1.0, 100.0).with_kind(ScaleKind::Log);
    assert!((log.coord_to_value(0.0, 0.0) - 10.0).abs() < 1e-9);
}

#[test]
fn nan_range_maps_to_the_middle() {
    for kind in [ScaleKind::Linear, ScaleKind::Percent, ScaleKind::Indexed] {
        let s = ValueScale::new(f64::NAN, 100.0).with_kind(kind);
        assert_eq!(s.value_to_coord(5.0, 300.0), 150.0, "{kind:?}");
    }
    // Log floors a NaN minimum like any value below the floor.
    let log = ValueScale::new(f64::NAN, 100.0).with_kind(ScaleKind::Log);
    assert!(log.value_to_coord(5.0, 300.0).is_finite());
}

#[test]
fn ticks_and_labels_survive_degenerate_input() {
    for kind in [ScaleKind::Linear, ScaleKind::Log, ScaleKind::Percent, ScaleKind::Indexed] {
        let s = ValueScale::new(0.0, 100.0).with_kind(kind);
        for len in [0.0, f64::NAN] {
            assert!(s.ticks(len).iter().all(|v| v.is_finite()), "{kind:?} length {len}");
            let _ = s.max_label_len(len);
        }
        let inf = ValueScale::new(0.0, f64::INFINITY).with_kind(kind);
        assert!(inf.ticks(300.0).iter().all(|v| !v.is_nan()), "{kind:?}");
        let _ = inf.max_label_len(300.0);
        let nan = ValueScale::new(f64::NAN, f64::NAN).with_kind(kind);
        assert!(nan.ticks(300.0).iter().all(|v| v.is_finite()), "{kind:?}");
        let _ = nan.max_label_len(300.0);
    }
}

// Reviewer fuzz cases.

#[test]
fn ticks_tiny_length_all_kinds() {
    for kind in [ScaleKind::Linear, ScaleKind::Log, ScaleKind::Percent, ScaleKind::Indexed] {
        let s = ValueScale::new(1.0, 100.0).with_kind(kind);
        for len in [1e-300, 0.001] {
            let t = s.ticks(len);
            assert!(t.len() <= uzor_plot::ticks::MAX_TICKS, "{kind:?} {len}");
            assert!(t.len() < 50, "{kind:?} {len}: {} ticks", t.len());
            assert!(t.iter().all(|v| v.is_finite()), "{kind:?} {len}");
            let _ = s.max_label_len(len);
        }
    }
}

#[test]
fn max_label_len_huge_grid_is_capped() {
    // Dense grid: max_ticks huge, spacing tiny, so the grid passes MAX_TICKS
    // and only the maximum's label is measured.
    let d = uzor_plot::ticks::TickDensity::default().with_spacing_px(1e-9).with_tick_bounds(1.0, 1e15);
    let s = ValueScale::new(0.0, 1e12).with_density(d);
    assert_eq!(s.max_label_len(300.0), s.format_value(1e12, 300.0).len());
    assert!(s.ticks(300.0).is_empty());
}

#[test]
fn set_range_clamped_rejects_non_finite() {
    let mut s = ValueScale::new(0.0, 100.0);
    assert!(!s.set_range_clamped(f64::NAN, 10.0));
    assert!(!s.set_range_clamped(0.0, f64::NAN));
    assert!(!s.set_range_clamped(f64::NEG_INFINITY, 10.0));
    assert!(!s.set_range_clamped(0.0, f64::INFINITY));
    assert_eq!((s.min, s.max, s.fit), (0.0, 100.0, RangeFit::Auto));
    assert!(s.set_range_clamped(0.0, 10.0));
    assert_eq!((s.min, s.max, s.fit), (0.0, 10.0, RangeFit::Manual));
}
