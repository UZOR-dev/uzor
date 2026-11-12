//! Fit a value axis to some data and print its grid in each scale kind.
//!
//! `cargo run -p uzor-plot --example value_axis`
use uzor_plot::value_scale::{ScaleKind, ValueScale};

fn main() {
    let data = [(98.0, 104.0), (101.0, 109.5), (95.5, 102.0), (100.0, 118.0)];
    let height = 400.0;
    for kind in [ScaleKind::Linear, ScaleKind::Log, ScaleKind::Percent, ScaleKind::Indexed] {
        let mut axis = ValueScale::new(0.0, 1.0).with_kind(kind).with_base(100.0);
        axis.auto_fit(&data, 0..data.len(), |&e| e);
        print!("{kind:?}:");
        for v in axis.ticks(height) {
            print!(" {}@{:.0}", axis.format_label(v, height), axis.value_to_coord(v, height));
        }
        println!();
    }
}
