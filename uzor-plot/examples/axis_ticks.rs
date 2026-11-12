//! Print the grid and labels of a 300px value axis.
//!
//! `cargo run -p uzor-plot --example axis_ticks`
use uzor_plot::ticks::{LabelPrecision, TickDensity};

fn main() {
    let (min, max, height) = (86_000.0, 87_000.0, 300.0);
    let (step, ticks) = TickDensity::default().ticks(min, max, height);
    let labels = LabelPrecision::default().with_ceiling(Some(1));
    println!("step {step}");
    for t in ticks {
        println!("{:>10}", labels.format(t, step));
    }
}
