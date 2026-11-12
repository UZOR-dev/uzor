//! Print the time ticks of a 1-minute series at a few zoom levels.
//!
//! `cargo run -p uzor-plot --example time_axis`
use uzor_plot::time_scale::{CalendarLabels, TimeScale};
use uzor_plot::viewport::SlotViewport;

fn main() {
    let start = 1_700_000_000 - 1_700_000_000 % 60;
    let times: Vec<i64> = (0..5000).map(|i| start + i * 60).collect();
    let labels = CalendarLabels::default();
    let measure = |s: &str| s.chars().count() as f64 * 7.0;
    for spacing in [0.7, 3.0, 12.0, 60.0] {
        let mut vp = SlotViewport::new(1200.0).with_spacing(spacing).with_count(times.len());
        vp.scroll_to_end();
        let ticks = TimeScale::new().generate_ticks(&vp, &times, measure, &labels, false);
        let row: Vec<String> = ticks.iter().map(|t| format!("{}@{:.0}", t.label, t.x)).collect();
        println!("{spacing:>5} px/slot: {}", row.join("  "));
    }
}
